use crate::cfg::Config;
use crate::utils;
use anyhow::Result;
use clap::{Arg, ArgMatches, Command};

use crate::cmds::SubCmd;
use std::io::prelude::*;

pub struct MultiDynamicProxy;

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

    fn handler(arg: &ArgMatches) -> Result<()> {
        let config = Config::loads(arg.value_of("config"))?;
        let addr = config.get_dynamic_local_addr();
        let forward = MultiDynamicProxy::get_forward_addr(&config);
        match arg.value_of("operation").unwrap() {
            "start" => {
                MultiDynamicProxy::start(&config)?;
            }
            "stop" => {
                MultiDynamicProxy::stop(addr, forward.as_str())?;
            }
            "restart" => {
                MultiDynamicProxy::stop(addr, forward.as_str())?;
                MultiDynamicProxy::start(&config)?;
            }
            _ => {}
        }
        Ok(())
    }
}

impl MultiDynamicProxy {
    fn get_forward_addr(config: &Config) -> String {
        format!(
            "{}@{}",
            config.get_multi_dynamic_forward_user(),
            config.get_multi_dynamic_forward_ip()
        )
    }

    fn start(config: &Config) -> Result<()> {
        let remote = format!(
            "{}:{}",
            config.get_multi_dynamic_remote_ip(),
            config.get_multi_dynamic_remote_port(),
        );
        let forward = MultiDynamicProxy::get_forward_addr(config);
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
            tx.send(String::from_utf8_lossy(&s[..n]).to_string())
                .unwrap();
        });
        let status = local_forward.wait()?;
        if let Ok(e) = rx.recv_timeout(std::time::Duration::from_secs(2)) {
            if e.contains("failed") || e.contains("Address already in use") || !status.success() {
                utils::print_with_color(
                    format!("Open multi dynamic proxy failed: \n{}\n", e.trim()).as_str(),
                    31,
                    true,
                );
                std::process::exit(1);
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
            let addr = config.get_multi_dynamic_local_addr();
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
                tx.send(String::from_utf8_lossy(&s[..n]).to_string())
                    .unwrap();
            });
            let status = dynamic_proxy.wait()?;
            if let Ok(e) = rx.recv_timeout(std::time::Duration::from_secs(2)) {
                if e.contains("failed") || e.contains("Address already in use") || !status.success()
                {
                    utils::print_with_color(
                        format!("Open multi dynamic proxy failed: \n{}\n", e.trim()).as_str(),
                        31,
                        true,
                    );
                    if !e.contains("Address already in use") {
                        MultiDynamicProxy::stop(addr, forward)?;
                    }
                    std::process::exit(1);
                }
            }
            if !utils::check_result(utils::check(addr), addr) {
                MultiDynamicProxy::stop(addr, forward)?;
            }
        }
        Ok(())
    }

    fn stop(addr: &str, forward: &str) -> Result<()> {
        let mut pids = utils::get_pids(addr)?;
        let pid2 = utils::get_pids(forward)?;
        pids.extend(pid2);
        for pid in pids.as_slice() {
            #[cfg(target_family = "unix")]
            utils::kill_child_by_pid(pid.to_owned())?;
            #[cfg(target_os = "windows")]
            utils::kill_child_by_pid_windows(pid)?;
        }
        if !pids.is_empty() {
            utils::print_with_color("Stop Success!\n", 34, false);
        } else {
            utils::print_with_color("No Process to Kill.\n", 33, false);
        }
        Ok(())
    }
}
