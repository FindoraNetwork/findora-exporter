use anyhow::{Context, Result};
use serde::Deserialize;

use std::{collections::HashMap, fs::File, path::Path};

const DEFAULT_CONFIG_PATH: &str = "config.json";

pub(crate) fn read_config() -> Result<Config> {
    let mut cfg = Config::default();
    if Path::new(DEFAULT_CONFIG_PATH).exists() {
        let f = File::open(DEFAULT_CONFIG_PATH).context("read config file failed")?;
        cfg = serde_json::from_reader(f).context("deserialize config file failed")?;
    }

    Ok(cfg)
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub(crate) struct Config {
    pub(crate) log_level: String,
    pub(crate) crawler: Crawler,
    pub(crate) server: Server,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            log_level: "trace".to_string(),
            crawler: Crawler::default(),
            server: Server::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub(crate) struct Crawler {
    pub(crate) targets: Vec<Target>,
}

impl Default for Crawler {
    fn default() -> Self {
        Crawler {
            targets: vec![Target {
                host_addr: "http://127.0.0.1:26657".to_string(),
                frequency_ms: 3000,
                registry: None,
            }],
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct Target {
    pub(crate) host_addr: String,
    pub(crate) frequency_ms: u64,
    pub(crate) registry: Option<Registry>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct Registry {
    pub(crate) prefix: String,
    #[serde(flatten)]
    pub(crate) labels: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub(crate) struct Server {
    pub(crate) listen_addr: String,
}

impl Default for Server {
    fn default() -> Self {
        Server {
            listen_addr: "127.0.0.1:9090".to_string(),
        }
    }
}
