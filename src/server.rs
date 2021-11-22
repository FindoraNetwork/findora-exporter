use log::error;
use prometheus::TextEncoder;

pub(crate) struct Server {
    server: tiny_http::Server,
}

impl Server {
    pub(crate) fn new(addr: &str) -> Self {
        Server {
            server: tiny_http::Server::http(addr).expect("server binding failed"),
        }
    }

    pub(crate) fn close(&self) {
        self.server.unblock()
    }

    pub(crate) fn run(&self) {
        // consume every prometheus scrape request one by one
        for request in self.server.incoming_requests() {
            // for prometheus usage, only handle
            // 1. method == GET
            // 2. url path == /metrics
            if request.method().as_str() != "GET" || request.url() != "/metrics" {
                let response = tiny_http::Response::empty(403);
                if let Err(e) = request.respond(response) {
                    error!("respond failed: {}", e);
                }
                continue;
            }

            let encoder = TextEncoder::new();
            let response = match encoder.encode_to_string(&prometheus::gather()) {
                Ok(v) => tiny_http::Response::from_string(v).boxed(),
                Err(e) => {
                    error!("encode to string failed: {}", e);
                    tiny_http::Response::empty(500).boxed()
                }
            };
            if let Err(e) = request.respond(response) {
                error!("respond failed: {}", e);
            }
        }
    }
}
