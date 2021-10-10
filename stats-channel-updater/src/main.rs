mod config;
use config::Config;

mod client;
use client::Client;

mod error;
pub use error::{AppError, Result};

use log::{debug, error};
use std::thread::sleep;
use std::time::Duration;

fn main() {
    env_logger::init();

    let conf = Config::load();
    let client = Client::new(conf);

    loop {
        do_update(&client);
        sleep(Duration::from_secs(60 * 10));
    }
}

fn do_update(client: &Client) {
    let count = match client.fetch_server_count() {
        Ok(v) => v,
        Err(e) => {
            error!("Error fetching server count: {}", e);
            return;
        }
    };

    debug!("Server count: {}", count);

    if let Err(e) = client.update_channel(count) {
        error!("Error updating channel: {}", e);
        return;
    }

    debug!("Updated channel successfully");
}
