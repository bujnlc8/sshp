use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    dynamic_proxy: Option<DynamicProxyConfig>,
    multi_proxy: Option<MultiDynamicProxyConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DynamicProxyConfig {
    local_addr: String,
    remote_user: Option<String>,
    remote_ip: String,
    remote_port: Option<usize>,
    heart_beat_interval: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MultiDynamicProxyConfig {
    local_addr: String,
    local_forward_port: Option<usize>,
    remote_user: Option<String>,
    remote_ip: String,
    remote_port: Option<usize>,
    heart_beat_interval: Option<usize>,
    forward_ip: String,
    forward_port: Option<usize>,
    forward_user: Option<String>,
}

impl Config {
    fn get_home_dir() -> Result<String> {
        Ok(dirs::home_dir()
            .expect("get home dir failed")
            .to_string_lossy()
            .to_string())
    }

    pub fn loads(path: Option<&str>) -> Result<Self> {
        let config_path = match path {
            Some(e) => std::path::PathBuf::from(e.replace('~', Config::get_home_dir()?.as_str())),
            None => std::path::PathBuf::from(Config::get_home_dir()?)
                .join(".config")
                .join("sshp.toml"),
        };
        if !config_path.exists() {
            println!("{} not found", config_path.to_string_lossy());
            std::process::exit(1);
        }
        let config: Config = toml::from_str(std::fs::read_to_string(config_path)?.as_str())?;
        Ok(config)
    }

    pub fn load_dynamic_config(&self) -> &DynamicProxyConfig {
        if let Some(ref e) = self.dynamic_proxy {
            return e;
        }
        eprintln!("Cannot loads config file");
        std::process::exit(1);
    }

    pub fn get_dynamic_local_addr(&self) -> &str {
        self.load_dynamic_config().local_addr.as_str()
    }

    pub fn get_dynamic_remote_user(&self) -> &str {
        match &self.load_dynamic_config().remote_user {
            Some(e) => e.as_str(),
            None => "root",
        }
    }

    pub fn get_dynamic_remote_port(&self) -> usize {
        self.load_dynamic_config().remote_port.unwrap_or(22)
    }

    pub fn get_dynamic_remote_ip(&self) -> &str {
        self.load_dynamic_config().remote_ip.as_str()
    }

    pub fn get_dynamic_heart_beat_interval(&self) -> usize {
        self.load_dynamic_config().heart_beat_interval.unwrap_or(60)
    }

    pub fn load_multi_dynamic_config(&self) -> &MultiDynamicProxyConfig {
        if let Some(ref e) = self.multi_proxy {
            return e;
        }
        eprintln!("Cannot loads config file");
        std::process::exit(1);
    }

    pub fn get_multi_dynamic_local_addr(&self) -> &str {
        self.load_multi_dynamic_config().local_addr.as_str()
    }

    pub fn get_multi_dynamic_remote_user(&self) -> &str {
        match &self.load_multi_dynamic_config().remote_user {
            Some(e) => e.as_str(),
            None => "root",
        }
    }

    pub fn get_multi_dynamic_remote_port(&self) -> usize {
        self.load_multi_dynamic_config().remote_port.unwrap_or(22)
    }

    pub fn get_multi_dynamic_remote_ip(&self) -> &str {
        self.load_multi_dynamic_config().remote_ip.as_str()
    }

    pub fn get_multi_dynamic_heart_beat_interval(&self) -> usize {
        self.load_multi_dynamic_config()
            .heart_beat_interval
            .unwrap_or(60)
    }

    pub fn get_multi_dynamic_forward_user(&self) -> &str {
        match &self.load_multi_dynamic_config().forward_user {
            Some(e) => e.as_str(),
            None => "root",
        }
    }

    pub fn get_multi_dynamic_forward_port(&self) -> usize {
        self.load_multi_dynamic_config().forward_port.unwrap_or(22)
    }

    pub fn get_multi_dynamic_forward_ip(&self) -> &str {
        self.load_multi_dynamic_config().forward_ip.as_str()
    }

    pub fn get_multi_dynamic_local_forward_port(&self) -> Option<usize> {
        self.load_multi_dynamic_config().local_forward_port
    }
}
