use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Hello {
    #[serde(rename = "d")]
    pub data: HelloData,
}

#[derive(Deserialize, Debug)]
pub struct HelloData {
    pub heartbeat_interval: u32,
}