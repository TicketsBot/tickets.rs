use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone)]
pub struct ImageHash {
    pub animated: bool,
    data: u128,
}

impl Serialize for ImageHash {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let s = if self.animated {
            format!("a_{:x}", self.data)
        } else {
            format!("{:x}", self.data)
        };

        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for ImageHash {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;

        let animated = raw.len() == 34;
        let hash = raw.trim_start_matches("a_");
        let data = u128::from_str_radix(hash, 16).map_err(Error::custom)?;

        Ok(ImageHash { animated, data })
    }
}
