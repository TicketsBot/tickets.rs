use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::de::Error;

#[derive(Debug)]
pub struct Snowflake(pub u64);

impl Serialize for Snowflake {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for Snowflake {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Snowflake(String::deserialize(deserializer)?.parse().map_err(Error::custom)?))
    }
}