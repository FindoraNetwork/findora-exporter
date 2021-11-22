use log::error;
use tungstenite::{connect, protocol::WebSocket, stream::MaybeTlsStream};

use std::{
    net::TcpStream,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub(crate) struct Socket {
    done: Arc<AtomicBool>,
    stream: WebSocket<MaybeTlsStream<TcpStream>>,
}

impl Socket {
    pub(crate) fn new(addr: &str, done: Arc<AtomicBool>) -> Self {
        let (stream, _) = connect(addr).expect("listener connect failed");
        Socket { done, stream }
    }

    pub(crate) fn run(&mut self) {
        while !self.done.load(Ordering::SeqCst) {
            match self.stream.read_message() {
                Ok(_message) => {}
                Err(e) => error!("socket read message failed: {}", e),
            }
        }
    }
}
