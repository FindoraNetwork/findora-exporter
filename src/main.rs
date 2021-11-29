use lazy_static::lazy_static;
use prometheus::{opts, register_gauge, register_histogram_vec};
use prometheus::{Gauge, HistogramVec};

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
    let crawler = crawler::Crawler::new(&cfg.crawler);

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
