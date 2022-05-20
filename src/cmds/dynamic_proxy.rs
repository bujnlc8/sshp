#![allow(clippy::new_without_default)]

use crate::cfg::Config;
use crate::utils;
use anyhow::Result;
use clap::{Arg, ArgMatches, Command};

use crate::cmds::{Start, SubCmd};
use std::io::prelude::*;

pub struct DynamicProxy {}

impl SubCmd for DynamicProxy {
    fn usage<'a>() -> Command<'a> {
        Command::new("dynamic_proxy")
            .about("Open SSH Dynamic Proxy")
            .visible_alias("d")
            .arg(
                Arg::new("operation")
                    .help("operation type to operate the dynamic proxy")
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
        let addr = config.get_dynamic_local_addr();
        match arg.value_of("operation").unwrap() {
            "start" => {
                utils::stop_probe_process(addr)?;
                self.start_with_probe(&config, addr)?;
            }
            "stop" => {
                utils::stop_probe_process(addr)?;
                self.stop(addr, true)?;
            }
            "restart" => {
                utils::stop_probe_process(addr)?;
                self.stop(addr, true)?;
                self.start_with_probe(&config, addr)?;
            }
            _ => {}
        }
        Ok(())
    }
}

impl Start for DynamicProxy {
    fn start(&self, config: &Config, echo: bool) -> Result<()> {
        let remote = format!(
            "{}@{}",
            config.get_dynamic_remote_user(),
            config.get_dynamic_remote_ip(),
        );
        let addr = config.get_dynamic_local_addr();
        let mut child = std::process::Command::new("ssh")
            .args(vec![
                "-CNf",
                "-o",
                format!(
                    "ServerAliveInterval={}",
                    config.get_dynamic_heart_beat_interval()
                )
                .as_str(),
                "-o",
                "StrictHostKeyChecking=no",
                "-D",
                addr,
                remote.as_str(),
                "-p",
                config.get_dynamic_remote_port().to_string().as_str(),
            ])
            .stderr(std::process::Stdio::piped())
            .spawn()?;
        let (tx, rx) = std::sync::mpsc::channel();
        let mut stderr = child.stderr.take().unwrap();
        std::thread::spawn(move || {
            let mut s = vec![0; 1024 * 10];
            // sleep 1s to get all error message
            std::thread::sleep(std::time::Duration::from_millis(1000));
            let n = stderr.read(&mut s).unwrap();
            if tx
                .send(String::from_utf8_lossy(&s[..n]).to_string())
                .is_err()
            {}
        });
        let status = child.wait()?;
        if let Ok(e) = rx.recv_timeout(std::time::Duration::from_secs(2)) {
            if e.contains("failed") || e.contains("Address already in use") || !status.success() {
                self.stop(addr, echo)?;
                anyhow::bail!("Open dynamic proxy failed: \n{}", e.trim());
            }
        }
        if status.success() {
            if !utils::check_result(utils::check(addr), addr, echo) {
                self.stop(addr, echo)?;
                anyhow::bail!("curl check {} failed.", addr);
            }
        } else {
            self.stop(addr, echo)?;
            anyhow::bail!(
                "Open dynamic proxy failed, status code is {}",
                status.to_string()
            );
        }
        Ok(())
    }
}

impl DynamicProxy {
    pub fn new() -> Self {
        Self {}
    }

    fn stop(&self, addr: &str, echo: bool) -> Result<()> {
        let pids = utils::get_pids(addr)?;
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
