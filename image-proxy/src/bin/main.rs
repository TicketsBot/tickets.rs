use image_proxy::http::Server;
use image_proxy::Config;

#[tokio::main]
async fn main() {
    env_logger::init();

    let config = Config::from_envvar();
    let server = Server::new(config);
    server.start().await;
}
