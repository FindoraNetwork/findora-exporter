use log::error;
use tungstenite::{connect, protocol::WebSocket, stream::MaybeTlsStream};

use std::net::TcpStream;

pub(crate) struct Socket {
    stream: WebSocket<MaybeTlsStream<TcpStream>>,
}

impl Socket {
    pub(crate) fn new(addr: &str) -> Self {
        let (stream, _) = connect(addr).expect("listener connect failed");
        Socket { stream }
    }

    pub(crate) fn close(&mut self) {
        let _ = self.stream.close(None);
    }

    pub(crate) fn run(&mut self) {
        match self.stream.read_message() {
            Ok(_message) => {}
            Err(e) => error!("socket read message failed: {}", e),
        }
    }
}
