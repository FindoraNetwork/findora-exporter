use std::{env, path::Path, sync::Arc};

use prometheus::core::AtomicU64;

mod config;
mod crawler;
mod metrics;
mod server;
mod tasks;
mod utils;

fn main() {
    match parse_command() {
        Command::Help => print_help(),
        Command::ConfigPath(path) => run(&path),
    }
}

fn run(cfg_path: &str) {
    let cfg = config::read_config(Path::new(cfg_path)).expect("read config failed");

    let log_level = match cfg.log_level.to_lowercase().as_ref() {
        "trace" => log::Level::Trace,
        "debug" => log::Level::Debug,
        "error" => log::Level::Error,
        "warn" => log::Level::Warn,
        "info" => log::Level::Info,
        _ => log::Level::Trace,
    };
    simple_logger::init_with_level(log_level).expect("simple logger init failed");

    let metrics =
        Arc::new(metrics::Metrics::<AtomicU64>::new(&cfg.crawler).expect("metrics new failed"));
    let server = server::Server::new(&cfg.server, metrics.clone());
    let mut crawler = crawler::Crawler::new(&cfg.crawler, metrics).expect("crawler new failed");

    let threads = vec![server.run().expect("server thread run failed")];

    ctrlc::set_handler(move || {
        server.close();
        crawler.close();
    })
    .expect("setting Ctrl-C handler failed");

    for t in threads {
        // no matter what we need to wait all of the thread stopped
        let _ = t.join();
    }
}

fn print_help() {
    println!(
        "Usage findora-exporter [OPTION]... [FILE]...
Just run the program without any options will using default config settings,
Default config path is a file named `config.json` under the current folder.

Mandatory arguments to long options.
--config    specific the config file path"
    )
}

#[derive(Debug, PartialEq)]
enum Command {
    ConfigPath(String),
    Help,
}

fn parse_command() -> Command {
    let args: Vec<String> = env::args().collect();
    match args.len() {
        0..=1 => Command::ConfigPath(crate::config::DEFAULT_CONFIG_PATH.to_string()),
        _ => match args[1].as_ref() {
            "--config" => Command::ConfigPath(args[2].clone()),
            _ => Command::Help,
        },
    }
}

#[cfg(test)]
mod test_util {
    use anyhow::{Context, Result};
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    pub(crate) struct TmpDir {
        path: Option<PathBuf>,
    }

    impl TmpDir {
        pub(crate) fn new<P: Into<PathBuf>>(path: P) -> Result<Self> {
            let p = path.into();
            fs::create_dir_all(&p)
                .with_context(|| format!("failed to create directory: {:?}", p))?;
            Ok(Self { path: Some(p) })
        }

        pub(crate) fn path(&self) -> &Path {
            self.path.as_ref().expect("tmp dir has been removed")
        }

        pub(crate) fn remove(&mut self) {
            if let Some(p) = &self.path {
                let _ = fs::remove_dir_all(p);
                self.path = None;
            }
        }
    }

    impl Drop for TmpDir {
        fn drop(&mut self) {
            self.remove();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config, test_util::TmpDir};
    use nix::{
        sys::{
            signal::{kill, Signal},
            wait::waitpid,
        },
        unistd,
        unistd::ForkResult,
    };
    use std::fs;

    #[test]
    fn test_parse_command() {
        assert_eq!(
            Command::ConfigPath(config::DEFAULT_CONFIG_PATH.to_string()),
            parse_command()
        );
    }

    #[test]
    fn test_run() {
        let tmp_dir = TmpDir::new(format!("{}/test_run", env::temp_dir().display())).unwrap();
        let cfg_path = format!("{}/config.json", tmp_dir.path().display());

        let mut cfg = config::Config {
            log_level: "info".to_string(),
            ..Default::default()
        };
        // crawling the findora mainnet for testing
        cfg.crawler.targets = vec![config::Target {
            host_addr: "https://prod-mainnet.prod.findora.org:26657".to_string(),
            task_name: config::TaskName::TotalCountOfValidators,
            registry: None,
            extra_opts: None,
        }];
        let json = serde_json::to_string(&cfg).unwrap();
        fs::write(&cfg_path, &json).unwrap();

        match unsafe { unistd::fork() } {
            Ok(ForkResult::Parent { child }) => {
                if let Err(ureq::Error::Status(code, _)) = ureq::get("127.0.0.1:9090").call() {
                    assert_eq!(403, code);
                };

                if let Err(ureq::Error::Status(code, _)) =
                    ureq::get("127.0.0.1:9090/not_metrics").call()
                {
                    assert_eq!(403, code);
                };

                if let Ok(response) = ureq::get("127.0.0.1:9090/metrics").call() {
                    assert_eq!(200, response.status());
                };

                kill(child, Signal::SIGTERM).unwrap();
                waitpid(child, None).unwrap();
            }
            Ok(ForkResult::Child) => {
                run(&cfg_path);
                std::process::exit(0);
            }
            Err(e) => panic!("{:?}", e),
        }
    }
}
