use lazy_static::lazy_static;
use log::info;
use prometheus::register_histogram_vec;
use prometheus::HistogramVec;

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

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

const DEFAULT_SERVER_ADDR: &str = "127.0.0.1:9090";
const DEFAULT_WEBSOCKET_ADDR: &str = "ws://127.0.0.1:26657/websocket";

fn main() {
    simple_logger::init().expect("simple logger init failed");

    let mut threads = Vec::with_capacity(2);
    let done = Arc::new(AtomicBool::new(false));
    let server = server::Server::new(DEFAULT_SERVER_ADDR, done.clone());
    let mut socket = websocket::Socket::new(DEFAULT_WEBSOCKET_ADDR, done.clone());

    ctrlc::set_handler(move || {
        done.store(true, Ordering::SeqCst);
    })
    .expect("setting Ctrl-C handler failed");

    threads.push(thread::spawn(move || server.run()));
    threads.push(thread::spawn(move || socket.run()));

    info!("server listening at: {}", DEFAULT_SERVER_ADDR);
    for t in threads {
        // no matter what we need to wait all of the thread stopped
        let _ = t.join();
    }
}
