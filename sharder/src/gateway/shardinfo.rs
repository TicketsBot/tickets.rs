use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug)]
pub struct ShardInfo {
    pub shard_id: u16,
    pub num_shards: u16,
}

impl ShardInfo {
    pub fn new(shard_id: u16, num_shards: u16) -> ShardInfo {
        ShardInfo {
            shard_id,
            num_shards,
        }
    }
}

impl Serialize for ShardInfo {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(2))?;

        seq.serialize_element(&self.shard_id)?;
        seq.serialize_element(&self.num_shards)?;

        seq.end()
    }
}

impl<'de> Deserialize<'de> for ShardInfo {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let seq: [u16; 2] = Deserialize::deserialize(deserializer)?;

        Ok(ShardInfo {
            shard_id: seq[0],
            num_shards: seq[1],
        })
    }
}
