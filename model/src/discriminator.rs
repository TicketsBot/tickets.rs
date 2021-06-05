use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::fmt::Formatter;

#[derive(Debug, Copy, Clone)]
pub struct Discriminator(pub u16);

impl Discriminator {
    pub fn serialize_to_int<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u16(self.0)
    }
}

impl Serialize for Discriminator {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{:0>4}", self.0))
    }
}

impl<'de> Deserialize<'de> for Discriminator {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Discriminator(
            String::deserialize(deserializer)?
                .parse()
                .map_err(Error::custom)?,
        ))
    }
}

impl fmt::Display for Discriminator {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
