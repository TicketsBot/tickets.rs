use super::PledgeResponse;
use super::Tier;
use crate::config::Config;
use crate::database::Tokens;

use std::collections::HashMap;
use std::sync::Arc;

use crate::error::PatreonError;
use log::{debug, error};

pub struct Poller {
    config: Arc<Config>,
    pub tokens: Arc<Tokens>,
}

impl Poller {
    pub fn new(config: Arc<Config>, tokens: Arc<Tokens>) -> Poller {
        Poller { config, tokens }
    }

    pub async fn poll(&self) -> Result<HashMap<String, Tier>, PatreonError> {
        let mut patrons: HashMap<String, Tier> = HashMap::new();
        let first = format!("https://www.patreon.com/api/oauth2/api/campaigns/{}/pledges?page%5Bcount%5D=25&sort=created", self.config.patreon_campaign_id);

        let mut res = self.poll_page(first).await?;
        while let Some(next) = &res.links.next {
            patrons.extend(res.convert().into_iter());
            res = self.poll_page(next.to_string()).await?;
        }

        patrons.extend(res.convert().into_iter());

        Ok(patrons)
    }

    async fn poll_page(&self, uri: String) -> Result<PledgeResponse, PatreonError> {
        debug!("Polling {}", uri);

        let client = reqwest::ClientBuilder::new()
            .use_rustls_tls()
            .build()
            .unwrap();

        let res = client
            .get(&uri)
            .header(
                "Authorization",
                format!("Bearer {}", self.tokens.access_token),
            )
            .send()
            .await?
            .bytes()
            .await?;

        let pledges =
            serde_json::from_slice::<PledgeResponse>(&res[..]).map_err(PatreonError::JsonError);
        let pledges = match pledges {
            Ok(v) => v,
            Err(e) => {
                error!(
                    "Error deserialising pledges: {}\nFull body: {:?}",
                    e,
                    std::str::from_utf8(&res[..])
                );
                return Err(e);
            }
        };

        Ok(pledges)
    }
}
