use super::Entitlement;
use super::PledgeResponse;
use crate::database::Tokens;
use std::collections::HashMap;

use std::sync::Arc;

use crate::error::Error;
use std::time::Duration;
use tracing::log::{debug, error};

pub struct Poller {
    client: reqwest::Client,
    campaign_id: String,
    pub tokens: Arc<Tokens>,
}

impl Poller {
    pub fn new(campaign_id: String, tokens: Arc<Tokens>) -> Poller {
        let client = reqwest::ClientBuilder::new()
            .use_rustls_tls()
            .timeout(Duration::from_secs(30))
            .user_agent("tickets-patreon-proxy/1.0")
            .build()
            .expect("failed to build http client");

        Poller {
            client,
            campaign_id,
            tokens,
        }
    }

    pub async fn poll(&self) -> Result<HashMap<String, Vec<Entitlement>>, Error> {
        let mut patrons: HashMap<String, Vec<Entitlement>> = HashMap::new();

        let campaign_id = self.campaign_id.as_str();
        let first = format!("https://www.patreon.com/api/oauth2/v2/campaigns/{campaign_id}/members?include=currently_entitled_tiers,user&fields%5Bmember%5D=last_charge_date,last_charge_status,patron_status,pledge_cadence,next_charge_date&fields%5Buser%5D=social_connections");

        let mut res = self.poll_page(first.as_str()).await?;
        while let Some(next) = res.links.as_ref().and_then(|l| l.next.as_ref()) {
            patrons.extend(res.convert());
            res = self.poll_page(next.as_str()).await?;
        }

        patrons.extend(res.convert());

        Ok(patrons)
    }

    async fn poll_page(&self, uri: &str) -> Result<PledgeResponse, Error> {
        debug!("Polling {}", uri);

        let res = self
            .client
            .get(uri)
            .header(
                "Authorization",
                format!("Bearer {}", self.tokens.access_token),
            )
            .send()
            .await?
            .bytes()
            .await?;

        debug!("Poll complete");

        let pledges = serde_json::from_slice::<PledgeResponse>(&res[..]).map_err(Error::JsonError);
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
