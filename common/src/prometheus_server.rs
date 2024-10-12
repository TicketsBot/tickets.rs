use std::error::Error;
use std::fmt::Debug;

use http_body_util::Full;
use hyper::body::{Body, Bytes};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::{TokioIo, TokioTimer};
use tokio::net::{TcpListener, ToSocketAddrs};

use prometheus::{Encoder, TextEncoder};
use tracing::{debug, info, trace};

pub async fn start_server<T: ToSocketAddrs + Debug>(server_addr: T) -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(&server_addr).await?;

    info!("Starting metrics server on {server_addr:?}");

    tokio::spawn(async move {
        loop {
            let (socket, _) = match listener.accept().await {
                Ok(s) => s,
                Err(e) => {
                    debug!(error = %e, "Failed to accept connection");
                    continue;
                }
            };

            let io = TokioIo::new(socket);

            tokio::spawn(async move {
                if let Err(e) = http1::Builder::new()
                    .timer(TokioTimer::new())
                    .serve_connection(io, service_fn(handle))
                    .await
                {
                    debug!(error = %e, "Failed to serve connection");
                }
            });
        }
    });

    Ok(())
}

async fn handle(_: Request<impl Body>) -> Result<Response<Full<Bytes>>, prometheus::Error> {
    trace!("Received metrics request");

    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();

    let metric_families = prometheus::gather();
    encoder.encode(&metric_families, &mut buffer)?;

    Ok(Response::new(Full::new(buffer.into())))
}
