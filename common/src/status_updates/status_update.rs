use serde::{Serialize, Deserialize};
use model::Snowflake;

pub const KEY: &str = "tickets:statusupdates";

#[derive(Serialize, Deserialize, Debug)]
pub struct Payload(pub Snowflake);
