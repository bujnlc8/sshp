#![allow(clippy::new_without_default)]
use crate::cfg::Config;
use crate::utils;
use anyhow::Result;
use clap::{Arg, ArgMatches, Command};

use crate::cmds::{Start, SubCmd};
use std::io::prelude::*;

pub struct MultiDynamicProxy {}

impl SubCmd for MultiDynamicProxy {
    fn usage<'a>() -> Command<'a> {
        Command::new("multi_proxy")
            .about("Open Multi SSH Dynamic Proxy")
            .visible_alias("m")
            .arg(
                Arg::new("operation")
                    .help("operation type to operate the multi dynamic proxy")
                    .short('t')
                    .default_value("start")
                    .possible_values(vec!["start", "stop", "restart"]),
            )
            .arg(
                Arg::new("config")
                    .help("config file path")
                    .short('c')
                    .required(false)
                    .default_value("~/.config/sshp.toml"),
            )
    }

    fn handler(&self, arg: &ArgMatches) -> Result<()> {
        let config = Config::loads(arg.value_of("config"))?;
        let addr = config.get_multi_dynamic_local_addr();
        let forward = self.get_forward_addr(&config);
        match arg.value_of("operation").unwrap() {
            "start" => {
                utils::stop_probe_process(addr)?;
                self.start_with_probe(&config, addr)?;
            }
            "stop" => {
                utils::stop_probe_process(addr)?;
                self.stop(addr, forward.as_str(), true)?;
            }
            "restart" => {
                utils::stop_probe_process(addr)?;
                self.stop(addr, forward.as_str(), true)?;
                self.start_with_probe(&config, addr)?;
            }
            _ => {}
        }
        Ok(())
    }
}

impl Start for MultiDynamicProxy {
    fn start(&self, config: &Config, echo: bool) -> Result<()> {
        let remote = format!(
            "{}:{}",
            config.get_multi_dynamic_remote_ip(),
            config.get_multi_dynamic_remote_port(),
        );
        let forward = self.get_forward_addr(config);
        let forward = forward.as_str();
        let available_port = match config.get_multi_dynamic_local_forward_port() {
            Some(e) => e,
            None => utils::get_avaliable_port() as usize,
        };
        let mut local_forward = std::process::Command::new("ssh")
            .args(vec![
                "-CNf",
                "-o",
                format!(
                    "ServerAliveInterval={}",
                    config.get_multi_dynamic_heart_beat_interval()
                )
                .as_str(),
                "-o",
                "StrictHostKeyChecking=no",
                "-L",
                format!("{}:{}", available_port, remote).as_str(),
                forward,
                "-p",
                config.get_multi_dynamic_forward_port().to_string().as_str(),
            ])
            .stderr(std::process::Stdio::piped())
            .spawn()?;
        let (tx, rx) = std::sync::mpsc::channel();
        let mut stderr = local_forward.stderr.take().unwrap();
        std::thread::spawn(move || {
            let mut s = vec![0; 1024 * 10];
            // sleep 0.5s to get all error message
            std::thread::sleep(std::time::Duration::from_millis(500));
            let n = stderr.read(&mut s).unwrap();
            if tx
                .send(String::from_utf8_lossy(&s[..n]).to_string())
                .is_err()
            {};
        });
        let status = local_forward.wait()?;
        let addr = config.get_multi_dynamic_local_addr();
        match rx.recv_timeout(std::time::Duration::from_secs(2)) {
            Ok(e) => {
                if e.contains("failed") || e.contains("Address already in use") || !status.success()
                {
                    self.stop(addr, forward, echo)?;
                    anyhow::bail!("Open multi dynamic proxy failed: \n{}", e.trim());
                }
            }
            Err(e) => {
                if e == std::sync::mpsc::RecvTimeoutError::Disconnected {
                    self.stop(addr, forward, echo)?;
                    anyhow::bail!("receive data from thread error happend, {}", e);
                }
            }
        }
        if status.success() {
            // dynamic proxy
            let remote_user_local = format!(
                "{}@{}",
                config.get_multi_dynamic_remote_user(),
                config
                    .get_multi_dynamic_local_addr()
                    .split(':')
                    .collect::<Vec<&str>>()[0]
            );
            let mut dynamic_proxy = std::process::Command::new("ssh")
                .args(vec![
                    "-CNf",
                    "-o",
                    format!(
                        "ServerAliveInterval={}",
                        config.get_multi_dynamic_heart_beat_interval()
                    )
                    .as_str(),
                    "-D",
                    addr,
                    remote_user_local.as_str(),
                    "-p",
                    available_port.to_string().as_str(),
                    "-o",
                    "StrictHostKeyChecking=no",
                ])
                .stderr(std::process::Stdio::piped())
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .spawn()?;
            let (tx, rx) = std::sync::mpsc::channel();
            let mut stderr = dynamic_proxy.stderr.take().unwrap();
            std::thread::spawn(move || {
                // sleep 0.5s to get all error message and other message
                std::thread::sleep(std::time::Duration::from_millis(500));
                let mut s = vec![0; 1024 * 10];
                let n = stderr.read(&mut s).unwrap();
                if tx
                    .send(String::from_utf8_lossy(&s[..n]).to_string())
                    .is_err()
                {}
            });
            let status = dynamic_proxy.wait()?;
            match rx.recv_timeout(std::time::Duration::from_secs(2)) {
                Ok(e) => {
                    if e.contains("failed")
                        || e.contains("Address already in use")
                        || !status.success()
                    {
                        self.stop(addr, forward, echo)?;
                        anyhow::bail!("Open multi dynamic proxy failed:\n{}", e.trim());
                    }
                }
                Err(e) => {
                    if e == std::sync::mpsc::RecvTimeoutError::Disconnected {
                        self.stop(addr, forward, echo)?;
                        anyhow::bail!("receive data from thread error happend, {}", e);
                    }
                }
            }
            if !utils::check_result(utils::check(addr), addr, echo) {
                self.stop(addr, forward, echo)?;
                anyhow::bail!("curl check {} failed.", addr);
            }
        } else {
            self.stop(addr, forward, echo)?;
            anyhow::bail!(
                "Open dynamic proxy failed, status code is {}",
                status.to_string()
            );
        }
        Ok(())
    }
}

impl MultiDynamicProxy {
    pub fn new() -> Self {
        Self {}
    }
    fn get_forward_addr(&self, config: &Config) -> String {
        format!(
            "{}@{}",
            config.get_multi_dynamic_forward_user(),
            config.get_multi_dynamic_forward_ip()
        )
    }

    fn stop(&self, addr: &str, forward: &str, echo: bool) -> Result<()> {
        let mut pids = utils::get_pids(addr)?;
        let pid2 = utils::get_pids(forward)?;
        pids.extend(pid2);
        for pid in pids.as_slice() {
            #[cfg(target_family = "unix")]
            utils::kill_child_by_pid(pid.to_owned())?;
            #[cfg(target_os = "windows")]
            utils::kill_child_by_pid_windows(pid)?;
        }
        if echo {
            if !pids.is_empty() {
                utils::print_with_color("Stop Success!\n", 34, false);
            } else {
                utils::print_with_color("No Process to Kill.\n", 33, false);
            }
        }
        Ok(())
    }
}
