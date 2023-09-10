use crate::error::Error;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PatreonResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    scope: String,
    token_type: String,
}

pub async fn refresh_tokens(
    refresh_token: String,
    client_id: String,
    client_secret: String,
) -> Result<PatreonResponse, Error> {
    let uri = format!("https://www.patreon.com/api/oauth2/token?grant_type=refresh_token&refresh_token={}&client_id={}&client_secret={}", refresh_token, client_id, client_secret);

    let client = reqwest::ClientBuilder::new()
        .use_rustls_tls()
        .build()
        .unwrap();
    let res: PatreonResponse = client.post(&uri).send().await?.json().await?;

    Ok(res)
}
