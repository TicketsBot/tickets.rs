use model::Snowflake;
use serde::{Deserialize, Serialize};

pub const KEY: &str = "tickets:statusupdates";

#[derive(Serialize, Deserialize, Debug)]
pub struct Payload(pub Snowflake);
