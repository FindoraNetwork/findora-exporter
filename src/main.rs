use std::sync::Arc;

mod config;
mod crawler;
mod metrics;
mod server;

fn main() {
    let cfg = config::read_config().expect("read config failed");

    let log_level = match cfg.log_level.to_lowercase().as_ref() {
        "trace" => log::Level::Trace,
        "debug" => log::Level::Debug,
        "error" => log::Level::Error,
        "warn" => log::Level::Warn,
        "info" => log::Level::Info,
        _ => log::Level::Trace,
    };
    simple_logger::init_with_level(log_level).expect("simple logger init failed");

    let metrics = Arc::new(metrics::Metrics::new(&cfg.crawler).expect("metrics new failed"));
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
