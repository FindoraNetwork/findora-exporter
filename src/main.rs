use lazy_static::lazy_static;
use prometheus::{opts, register_gauge, register_histogram_vec};
use prometheus::{Gauge, HistogramVec};

use std::sync::{Arc, RwLock};
use std::thread;

mod config;
mod crawler;
mod server;

lazy_static! {
    static ref VALIDATOR_ADDRESSES_HISTOGRAM: HistogramVec = register_histogram_vec!(
        "validator_addresses",
        "Total consensus validator addresses.",
        &["env"],
    )
    .expect("validator metrics create failed");
    static ref CONSENSUS_POWER: Gauge = register_gauge!(opts!(
        "consensus_power",
        "current consensus network voting power"
    ))
    .expect("consensus power gauge create failed");
}

fn main() {
    simple_logger::init().expect("simple logger init failed");
    let cfg = config::read_config().expect("read config failed");

    let server = server::Server::new(&cfg.server);
    let mut crawlers = vec![];

    for target in cfg.crawler.targets {
        crawlers.push(Arc::new(RwLock::new(crawler::Crawler::new(
            &target.host_addr,
            target.frequency_ms,
        ))));
    }

    let mut threads = Vec::with_capacity(2);

    threads.push(server.run().expect("server thread run failed"));

    for crawler in &crawlers {
        let crawler_spawn = Arc::clone(crawler);
        threads.push(
            thread::Builder::new()
                .name("crawler_thread".into())
                .spawn(move || crawler_spawn.write().unwrap().run())
                .unwrap(),
        );
    }

    ctrlc::set_handler(move || {
        server.close();
        for crawler in &crawlers {
            crawler.write().unwrap().close();
        }
    })
    .expect("setting Ctrl-C handler failed");

    for t in threads {
        // no matter what we need to wait all of the thread stopped
        let _ = t.join();
    }
}
