use crate::cfg::Config;
use anyhow::Result;
use clap::{Arg, ArgMatches, Command};

use crate::cmds::SubCmd;
use std::io::prelude::*;

pub struct DynamicProxy;

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

    fn handler(arg: &ArgMatches) -> Result<()> {
        let config = Config::loads(arg.value_of("config"))?;
        let addr = config.get_dynamic_local_addr();
        match arg.value_of("operation").unwrap() {
            "start" => {
                DynamicProxy::start(&config, addr)?;
            }
            "stop" => {
                DynamicProxy::stop(addr)?;
            }
            "restart" => {
                DynamicProxy::stop(addr)?;
                DynamicProxy::start(&config, addr)?;
            }
            _ => {}
        }
        Ok(())
    }
}

impl DynamicProxy {
    fn start(config: &Config, addr: &str) -> Result<()> {
        let remote = format!(
            "{}@{}",
            config.get_dynamic_remote_user(),
            config.get_dynamic_remote_ip(),
        );
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
            tx.send(String::from_utf8_lossy(&s[..n]).to_string())
                .unwrap();
        });
        let status = child.wait()?;
        if let Ok(e) = rx.recv_timeout(std::time::Duration::from_secs(2)) {
            if e.contains("failed") || e.contains("Address already in use") || !status.success() {
                println!("Open dynamic proxy failed: \n{}", e.trim());
                if !e.contains("Address already in use") {
                    DynamicProxy::stop(addr)?;
                }
                std::process::exit(1);
            }
        }
        if status.success() && !DynamicProxy::check_result(DynamicProxy::check(addr), addr) {
            DynamicProxy::stop(addr)?;
        }
        Ok(())
    }

    fn stop(addr: &str) -> Result<()> {
        let pids = DynamicProxy::get_pids(addr)?;
        for pid in pids.as_slice() {
            #[cfg(target_family = "unix")]
            DynamicProxy::kill_child_by_pid(pid.to_owned())?;
            #[cfg(target_os = "windows")]
            DynamicProxy::kill_child_by_pid_windows(pid)?;
        }
        if !pids.is_empty() {
            println!("Stop Success!");
        } else {
            println!("No Process to Kill.")
        }
        Ok(())
    }
}
