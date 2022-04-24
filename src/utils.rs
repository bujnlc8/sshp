use anyhow::Result;
use regex::Regex;

pub fn print_with_color(text: &str, color: u8, hightlight: bool) {
    let mut s = Vec::new();
    if hightlight {
        s.push("\x1b[1m");
    }
    let tmp;
    if (30..=39).contains(&color) {
        tmp = format!("\x1b[{}m", color);
        s.push(tmp.as_str());
    }
    s.push(text);
    s.push("\x1b[0m");
    print!("{}", s.join(""));
}
pub fn check(addr: &str) -> Result<String> {
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

pub fn check_result(res: Result<String>, addr: &str) -> bool {
    match res {
        Ok(e) => {
            if e.is_empty()
                || (!e.contains("Connection refused")
                    && !e.contains("connection to proxy closed")
                    && !e.contains("curl: ("))
            {
                print_with_color("Open Dynamic Proxy Success, listen addr is ", 32, false);
                print_with_color(addr, 37, true);
                print_with_color(".", 32, true);
                println!();
                true
            } else {
                print_with_color("Listen ", 32, false);
                print_with_color(addr, 37, true);
                print_with_color("success, ", 32, false);
                print_with_color(
                    "but curl www.baidu.com through the tunnel failed: \n",
                    31,
                    true,
                );
                print_with_color(e.as_str(), 31, true);
                println!();
                false
            }
        }
        Err(e) => {
            print_with_color("Listen ", 32, false);
            print_with_color(addr, 37, true);
            print_with_color("success, ", 32, false);
            print_with_color(
                "but little error happen when check the tunnel by curling www.baidu.com: \n",
                31,
                true,
            );
            print_with_color(e.to_string().as_str(), 31, true);
            println!();
            true
        }
    }
}

/// get child pids by listen addr
pub fn get_pids(addr: &str) -> Result<Vec<usize>> {
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

pub fn get_child_pid(ppid: usize) -> Result<usize> {
    let pgrep = std::process::Command::new("pgrep")
        .arg("-P")
        .arg(ppid.to_string().as_str())
        .stdout(std::process::Stdio::piped())
        .output()?;
    let pgrep_output = String::from_utf8_lossy(&pgrep.stdout).to_string();
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
pub fn kill_child_by_pid(pid: usize) -> Result<()> {
    std::process::Command::new("kill")
        .args(vec!["-9", pid.to_string().as_str()])
        .status()?;
    Ok(())
}

/// kill child by pid windows
#[cfg(target_os = "windows")]
pub fn kill_child_by_pid_windows(pid: usize) -> Result<()> {
    std::process::Command::new("taskkill")
        .args(vec!["/F", "/PID", pid.to_string().as_str()])
        .status()?;
    Ok(())
}

pub fn get_avaliable_port() -> u16 {
    (1025..65535)
        .find(|port| std::net::TcpListener::bind(("127.0.0.1", *port)).is_ok())
        .unwrap_or(50002)
}

#[cfg(test)]
mod test {
    use crate::utils;

    #[test]
    fn test_get_pids() {
        utils::get_pids("localhost:50003").unwrap();
    }
}
