use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(
    Serialize_repr, Deserialize_repr, Copy, Clone, Debug, Eq, FromPrimitive, PartialEq, Hash,
)]
#[repr(u8)]
pub enum ActivityType {
    Game = 0,
    Streaming = 1,
    Listening = 2,
    Watching = 3,
    Custom = 4,
    Competing = 5,
}

impl ActivityType {
    pub fn from_u8(value: u8) -> Option<ActivityType> {
        FromPrimitive::from_u8(value)
    }

    pub fn from_u16(value: u16) -> Option<ActivityType> {
        FromPrimitive::from_u16(value)
    }

    pub fn from_i16(value: i16) -> Option<ActivityType> {
        FromPrimitive::from_i16(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert() {
        assert_eq!(FromPrimitive::from_i16(1), Some(ActivityType::Streaming));
        assert_eq!(FromPrimitive::from_i16(4), Some(ActivityType::Custom));
        // assert_eq!(FromPrimitive::from_i16(1000), None);
    }
}
