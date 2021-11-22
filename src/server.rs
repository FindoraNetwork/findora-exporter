use log::error;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub(crate) struct Server {
    done: Arc<AtomicBool>,
    server: tiny_http::Server,
}

impl Server {
    pub(crate) fn new(addr: &str, done: Arc<AtomicBool>) -> Self {
        Server {
            done,
            server: tiny_http::Server::http(addr).expect("server binding failed"),
        }
    }

    pub(crate) fn run(&self) {
        while !self.done.load(Ordering::SeqCst) {
            match self.server.recv() {
                Ok(_request) => {}
                Err(e) => error!("server receiving request failed: {}", e),
            }
        }
    }
}
