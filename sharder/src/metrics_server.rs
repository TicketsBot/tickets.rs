use std::{convert::Infallible, net::SocketAddr, str::FromStr};

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};

use prometheus::{Encoder, TextEncoder};
use tracing::{info, trace};

use crate::Result;

pub async fn start_server(server_addr: &str) -> Result<()> {
    let addr = SocketAddr::from_str(server_addr)?;

    let make_svc = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });

    let server = Server::bind(&addr).serve(make_svc);

    info!("Starting metrics server on {addr}");
    server.await?;

    Ok(())
}

async fn handle(_: Request<Body>) -> Result<Response<Body>, prometheus::Error> {
    trace!("Received metrics request");

    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();

    let metric_families = prometheus::gather();
    encoder.encode(&metric_families, &mut buffer)?;

    Ok(Response::new(Body::from(buffer)))
}
