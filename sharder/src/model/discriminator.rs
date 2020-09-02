use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::de::Error;
use std::fmt;
use std::fmt::Formatter;

#[derive(Debug)]
pub struct Discriminator(pub u16);

impl Serialize for Discriminator {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for Discriminator {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Discriminator(String::deserialize(deserializer)?.parse().map_err(Error::custom)?))
    }
}

impl fmt::Display for Discriminator {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}