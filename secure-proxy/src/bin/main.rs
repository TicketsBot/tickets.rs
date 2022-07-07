use secure_proxy::http::Server;
use secure_proxy::Config;

#[tokio::main]
async fn main() {
    env_logger::init();

    let config = Config::from_envvar();
    let server = Server::new(config);
    server.start().await;
}
