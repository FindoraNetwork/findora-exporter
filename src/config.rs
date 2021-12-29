use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use std::{collections::HashMap, fs::File, path::Path};

pub(crate) const DEFAULT_CONFIG_PATH: &str = "config.json";

/// Returns Config structure from input file path of a JSON file, otherwise returns default Config
/// structure if the input file path does not exist.
///
/// The default config path equals the const DEFAULT_CONFIG_PATH variable.
pub(crate) fn read_config(path: &Path) -> Result<Config> {
    let mut cfg = Config::default();
    if path.exists() {
        let f = File::open(path).with_context(|| format!("read config file failed: {:?}", path))?;
        cfg = serde_json::from_reader(f)
            .with_context(|| format!("deserialize config file failed: {:?}", path))?;
    }

    Ok(cfg)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Crawler {
    pub(crate) targets: Vec<Target>,
}

impl Default for Crawler {
    fn default() -> Self {
        Crawler {
            targets: vec![
                Target {
                    host_addr: "http://127.0.0.1:26657".to_string(),
                    task_name: "get_network_functional".to_string(),
                    frequency_ms: 15000,
                    registry: None,
                },
                Target {
                    host_addr: "http://127.0.0.1:26657".to_string(),
                    task_name: "get_total_validators".to_string(),
                    frequency_ms: 15000,
                    registry: None,
                },
            ],
        }
    }
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Target {
    pub(crate) host_addr: String,
    pub(crate) task_name: String,
    pub(crate) frequency_ms: u64,
    pub(crate) registry: Option<Registry>,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Registry {
    pub(crate) prefix: String,
    #[serde(flatten)]
    pub(crate) labels: HashMap<String, String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::TmpDir;
    use std::{env, fs, path::PathBuf};

    #[test]
    fn test_read_config_fail_back_to_default_values() {
        let want = Config::default();
        let got = read_config(Path::new("")).unwrap();
        assert_eq!(want, got);
    }

    #[test]
    fn test_read_config() {
        let tmp_dir = TmpDir::new(format!(
            "{}/findora_exporter_test_read_config",
            env::temp_dir().display()
        ))
        .unwrap();
        let cfg_path = PathBuf::from(&format!("{}/config.json", tmp_dir.path().display()));

        let mut want = Config::default();
        want.server.listen_addr = "0.0.0.0:33456".to_string();
        let mut labels = HashMap::new();
        labels.insert("env".to_string(), "dev".to_string());
        want.crawler.targets.push(Target {
            host_addr: "https://somewhere.com/metrics:443".to_string(),
            task_name: "get_network_functional".to_string(),
            frequency_ms: 1000,
            registry: Some(Registry {
                prefix: "findora_exporter".to_string(),
                labels,
            }),
        });

        let json = serde_json::to_string(&want).unwrap();
        fs::write(cfg_path.as_path(), &json).unwrap();

        let got = read_config(cfg_path.as_path()).unwrap();
        assert_eq!(want, got);
    }
}
