use warp::reply::Response;
use std::convert::Infallible;
use crate::http::Server;
use std::sync::Arc;

pub async fn handle(server: Arc<Server>) -> Result<Response, Infallible> {
    panic!("abort")
}