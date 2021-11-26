use lazy_static::lazy_static;
use prometheus::register_histogram_vec;
use prometheus::HistogramVec;

use std::sync::{Arc, RwLock};
use std::thread;

mod crawler;
mod server;
mod websocket;

lazy_static! {
    static ref VALIDATOR_ADDRESSES_HISTOGRAM: HistogramVec = register_histogram_vec!(
        "validator_addresses",
        "Total consensus validator addresses.",
        &["env"],
    )
    .expect("validator metrics create failed");
}

const DEFAULT_CRAWLER_ADDR: &str = "https://prod-testnet.prod.findora.org:26657";
const DEFAULT_SERVER_ADDR: &str = "0.0.0.0:9090";
const DEFAULT_WEBSOCKET_ADDR: &str = "ws://127.0.0.1:26657/websocket";

fn main() {
    simple_logger::init().expect("simple logger init failed");

    let server = Arc::new(server::Server::new(DEFAULT_SERVER_ADDR));
    // let socket = Arc::new(RwLock::new(websocket::Socket::new(DEFAULT_WEBSOCKET_ADDR)));
    let crawler = Arc::new(RwLock::new(crawler::Crawler::new(DEFAULT_CRAWLER_ADDR)));

    let mut threads = Vec::with_capacity(2);

    let server_spawn = Arc::clone(&server);
    // let socket_spawn = Arc::clone(&socket);
    let crawler_spawn = Arc::clone(&crawler);
    threads.push(
        thread::Builder::new()
            .name("server_thread".into())
            .spawn(move || server_spawn.run())
            .unwrap(),
    );

    threads.push(
        thread::Builder::new()
            .name("crawler_thread".into())
            .spawn(move || crawler_spawn.write().unwrap().run())
            .unwrap(),
    );

    // threads.push(
    //     thread::Builder::new()
    //         .name("websocket_thread".into())
    //         .spawn(move || socket_spawn.write().unwrap().run())
    //         .unwrap(),
    // );

    ctrlc::set_handler(move || {
        server.close();
        crawler.write().unwrap().close();
        // socket.write().unwrap().close();
    })
    .expect("setting Ctrl-C handler failed");

    for t in threads {
        // no matter what we need to wait all of the thread stopped
        let _ = t.join();
    }
}
