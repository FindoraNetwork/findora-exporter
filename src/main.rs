use lazy_static::lazy_static;
use log::{error, info};
use prometheus::register_histogram_vec;
use prometheus::HistogramVec;

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

lazy_static! {
    static ref VALIDATOR_ADDRESSES_HISTOGRAM: HistogramVec = register_histogram_vec!(
        "validator_addresses",
        "Total consensus validator addresses.",
        &["env"],
    )
    .expect("validator metrics create failed");
}

const DEFAULT_SERVER_ADDR: &str = "0.0.0.0:9090";
const DEFAULT_WEBSOCKET_ADDR: &str = "ws://127.0.0.1:26657/websocket";

fn main() {
    simple_logger::init().expect("simple logger init failed");

    let mut threads = Vec::with_capacity(2);
    let done = Arc::new(AtomicBool::new(false));
    let server = Server::new(DEFAULT_SERVER_ADDR, done.clone());
    let mut socket = Socket::new(DEFAULT_WEBSOCKET_ADDR, done.clone());

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

struct Server {
    done: Arc<AtomicBool>,
    server: tiny_http::Server,
}

impl Server {
    fn new(addr: &str, done: Arc<AtomicBool>) -> Self {
        Server {
            done,
            server: tiny_http::Server::http(addr).expect("server binding failed"),
        }
    }

    fn run(&self) {
        while !self.done.load(Ordering::SeqCst) {
            match self.server.recv() {
                Ok(_request) => {}
                Err(e) => error!("server receiving request failed: {}", e),
            }
        }
    }
}

struct Socket {
    done: Arc<AtomicBool>,
    stream:
        tungstenite::protocol::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
}

impl Socket {
    fn new(addr: &str, done: Arc<AtomicBool>) -> Self {
        let (stream, _) = tungstenite::connect(addr).expect("listener connect failed");
        Socket { done, stream }
    }

    fn run(&mut self) {
        while !self.done.load(Ordering::SeqCst) {
            match self.stream.read_message() {
                Ok(_message) => {}
                Err(e) => error!("socket read message failed: {}", e),
            }
        }
    }
}
