use bot_list_updater::retriever::Retriever;
use bot_list_updater::updater::{DblUpdater, DiscordBoatsUpdater, Updater};
use bot_list_updater::Config;
use log::{debug, error, info};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    env_logger::init();
    let conf = Config::from_envvar();

    // Internally arc'd
    let http_client = reqwest::Client::new();

    let retriever = Retriever::new_with_client(conf.base_url, http_client.clone());
    let dbl_updater = DblUpdater::new_with_client(conf.dbl_token, conf.bot_id, http_client.clone());
    let discord_boats_updater =
        DiscordBoatsUpdater::new_with_client(conf.dboats_token, conf.bot_id, http_client);

    loop {
        sleep(Duration::from_secs(conf.delay)).await;

        let count: usize;
        match retriever.get_count().await {
            Ok(v) => count = v,
            Err(e) => {
                error!("Error while retrieving count: {}", e);
                continue;
            }
        }

        debug!("Retrieved count of {}", count);

        let (dbl_res, dboats_res) = tokio::join!(
            dbl_updater.update(count),
            discord_boats_updater.update(count),
        );

        if let Err(e) = dbl_res {
            error!("Error while updating DBL: {}", e);
        }

        if let Err(e) = dboats_res {
            error!("Error while updating Discord Boats: {}", e);
        }

        info!("Success: Updated count to {}", count);
    }
}
