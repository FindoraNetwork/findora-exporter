use std::{env, sync::Arc};

mod config;
mod crawler;
mod metrics;
mod server;

fn main() {
    match parse_command() {
        Command::Help => print_help(),
        Command::ConfigPath(p) => {
            let cfg = config::read_config(&p).expect("read config failed");

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
                Arc::new(metrics::Metrics::new(&cfg.crawler).expect("metrics new failed"));
            let server = server::Server::new(&cfg.server, metrics.clone());
            let crawler = crawler::Crawler::new(&cfg.crawler, metrics);

            let threads = vec![
                server.run().expect("server thread run failed"),
                crawler.run().expect("crawler thread run failed"),
            ];

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
    }
}

fn print_help() {
    println!(
        "Usage findora-exporter [OPTION]... [FILE]...
Just run the program without any options will using default config settings.

Mandatory arguments to long options.
--config    specific the config file path"
    )
}

enum Command {
    ConfigPath(String),
    Help,
}

fn parse_command() -> Command {
    let args: Vec<String> = env::args().collect();
    match args.len() {
        0..=1 => Command::ConfigPath("".to_string()),
        _ => match args[0].as_ref() {
            "--config" => Command::ConfigPath(args[1].clone()),
            _ => Command::Help,
        },
    }
}
