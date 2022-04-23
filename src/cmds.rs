pub mod dynamic_proxy;
pub mod multi_proxy;
use anyhow::Result;
use clap::{ArgMatches, Command};
use regex::Regex;

pub trait SubCmd {
    fn usage<'a>() -> Command<'a>;
    fn handler(arg: &ArgMatches) -> Result<()>;
    fn check(addr: &str) -> Result<String> {
        let child = std::process::Command::new("curl")
            .arg("--socks5")
            .arg(addr)
            .arg("https://www.baidu.com")
            .stderr(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .spawn()?
            .wait_with_output()?;
        if !child.stderr.is_empty() {
            Ok(String::from_utf8_lossy(child.stderr.as_slice()).to_string())
        } else {
            Ok("".to_string())
        }
    }

    fn check_result(res: Result<String>, addr: &str) -> bool {
        match res {
            Ok(e) => {
                if e.is_empty()
                    || (!e.contains("Connection refused")
                        && !e.contains("connection to proxy closed")
                        && !e.contains("curl: ("))
                {
                    println!("Open Dynamic Proxy Success, listen addr is {}: \n", addr);
                    true
                } else {
                    println!(
                        "Listen {} success, but curl www.baidu.com through the tunnel failed: \n{}.",
                        addr, e,
                    );
                    false
                }
            }
            Err(e) => {
                println!("Listen {} success, but little error happen when check the tunnel by curling www.baidu.com: \n{}.", addr, e);
                true
            }
        }
    }

    /// get child pids by listen addr
    fn get_pids(addr: &str) -> Result<Vec<usize>> {
        let mut res = Vec::new();
        let mut ps = std::process::Command::new("ps")
            .arg("aux")
            .stdout(std::process::Stdio::piped())
            .spawn()?;
        if let Some(ps_output) = ps.stdout.take() {
            let grep = std::process::Command::new("grep")
                .arg(addr)
                .stdin(ps_output)
                .stdout(std::process::Stdio::piped())
                .spawn()?;
            ps.wait()?;
            let grep_output = String::from_utf8_lossy(&grep.wait_with_output()?.stdout).to_string();
            let re = Regex::new(r"\s+").unwrap();
            for x in grep_output.split('\n') {
                let x = x.trim();
                if x.is_empty() || x.contains("grep") {
                    continue;
                }
                let replace = re.replace_all(x, "!#");
                let pid: &str = replace.split("!#").collect::<Vec<&str>>()[1];
                res.push(pid.parse()?)
            }
        }
        Ok(res)
    }

    fn get_child_pid(ppid: usize) -> Result<usize> {
        let pgrep = std::process::Command::new("pgrep")
            .arg("-P")
            .arg(ppid.to_string().as_str())
            .stdout(std::process::Stdio::piped())
            .output()?;
        let pgrep_output = String::from_utf8_lossy(&pgrep.stdout).to_string();
        println!("{}, {}", pgrep_output, ppid);
        for x in pgrep_output.split('\n') {
            let x = x.trim();
            if x.is_empty() {
                continue;
            }
            return Ok(x.parse()?);
        }
        Ok(0)
    }

    /// kill child by pid unix
    #[cfg(target_family = "unix")]
    fn kill_child_by_pid(pid: usize) -> Result<()> {
        std::process::Command::new("kill")
            .args(vec!["-9", pid.to_string().as_str()])
            .status()?;
        Ok(())
    }

    /// kill child by pid windows
    #[cfg(target_os = "windows")]
    fn kill_child_by_pid_windows(pid: usize) -> Result<()> {
        std::process::Command::new("taskkill")
            .args(vec!["/F", "/PID", pid.to_string().as_str()])
            .status()?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::cmds::dynamic_proxy::DynamicProxy;
    use crate::cmds::SubCmd;

    #[test]
    fn test_get_pids() {
        DynamicProxy::get_pids("localhost:50003").unwrap();
    }
}
