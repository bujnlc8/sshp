pub mod dynamic_proxy;
pub mod multi_proxy;
use crate::cfg::Config;
use crate::utils;
use anyhow::Result;
use clap::{ArgMatches, Command};
use fork::{fork, Fork};
use std::io::Write;

pub trait SubCmd {
    fn usage<'a>() -> Command<'a>;
    fn handler(&self, arg: &ArgMatches) -> Result<()>;
}

pub trait Start {
    fn start(&self, config: &Config, echo: bool) -> Result<()>;
    fn start_with_probe(&self, config: &Config, addr: &str) -> Result<()> {
        let log_path = utils::get_log_file(addr);
        match fork() {
            Ok(Fork::Parent(child)) => {
                let pid_file_path = utils::get_pid_file(addr);
                let mut pid_file = std::fs::File::create(pid_file_path)?;
                pid_file.write_all(child.to_string().as_bytes())?;
                utils::write_log(&log_path, format!("{} start ...", addr).as_str())?;
                self.start(config, true)?;
            }
            Ok(Fork::Child) => {
                std::thread::sleep(std::time::Duration::from_secs(30));
                let mut failed_times = 0;
                loop {
                    if let Ok(pids) = utils::get_pids(addr) {
                        if pids.is_empty() {
                            utils::write_log(
                                &log_path,
                                format!("{} start {}th in probe ...", addr, failed_times + 1)
                                    .as_str(),
                            )?;
                            if let Err(e) = self.start(config, false) {
                                failed_times += 1;
                                utils::write_log(
                                    &log_path,
                                    format!(
                                        "{} restart {}th error happend, {}",
                                        addr, failed_times, e
                                    )
                                    .as_str(),
                                )?;
                                if failed_times >= 3600 {
                                    std::process::exit(1);
                                }
                            }
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_secs(5));
                }
            }
            Err(e) => {
                println!("Fork failed, {}", e);
                std::process::exit(1);
            }
        }
        Ok(())
    }
}
