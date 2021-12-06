use anyhow::{Context, Result};
use log::error;
use prometheus::TextEncoder;
use std::{sync::Arc, thread, thread::JoinHandle};

/// A server instance to listen to an IPv4 address and only serve the /metrics path for Prometheus usage.
pub(crate) struct Server {
    metrics: Arc<crate::metrics::Metrics>,
    server: Arc<tiny_http::Server>,
}

impl Server {
    /// Returns a Server instance.
    ///
    /// This new will not execute anything but only returns a Server instance.
    /// The server only serves http protocol,
    /// and will Panics on server binding if any error occurs.
    pub(crate) fn new(cfg: &crate::config::Server, metrics: Arc<crate::metrics::Metrics>) -> Self {
        Server {
            metrics,
            server: Arc::new(
                tiny_http::Server::http(&cfg.listen_addr).expect("server binding failed"),
            ),
        }
    }

    /// This method allows graceful shutdown of server.
    pub(crate) fn close(&self) {
        self.server.unblock()
    }

    /// Spawned a new thread to listen to a specific address and port.
    /// Serving
    /// 1. GET method
    /// 2. /metrics path
    /// only
    ///
    /// returns 403 status code on other requests.
    /// returns 500 status code on encoding JSON failure.
    pub(crate) fn run(&self) -> Result<JoinHandle<()>> {
        let server = self.server.clone();
        let metrics = self.metrics.clone();

        thread::Builder::new()
            .name("server_thread".into())
            .spawn(move || {
                // consume every prometheus scrape request one by one
                for request in server.incoming_requests() {
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
                    let response = match encoder.encode_to_string(&metrics.gather()) {
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
            })
            .context("server thread run failed")
    }
}
