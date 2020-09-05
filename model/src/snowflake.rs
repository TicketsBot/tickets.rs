use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::de::Error;
use std::fmt;
use serde_json::Value;
use super::util;
use sqlx::error::BoxDynError;
use sqlx::Postgres;
use sqlx::postgres::{PgValueRef, PgArgumentBuffer};
use sqlx::encode::IsNull;

#[derive(Debug, Copy, Clone)]
pub struct Snowflake(pub u64);

impl Serialize for Snowflake {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for Snowflake {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value: Value = Deserialize::deserialize(deserializer)?;

        if let Some(i) = value.as_u64() {
            return Ok(Snowflake(i));
        }

        if let Some(s) = value.as_str() {
            return Ok(Snowflake(s.parse().map_err(Error::custom)?))
        }

        Err(Error::invalid_type(util::to_unexpected(value), &"a string or u64"))
    }
}

impl fmt::Display for Snowflake {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'r> sqlx::Decode<'r, Postgres> for Snowflake {
    fn decode(value: PgValueRef<'_>) -> Result<Self, BoxDynError> {
        let i = i64::decode(value)?;
        Ok(Snowflake(i as u64))
    }
}

impl<'q> sqlx::Encode<'q, Postgres> for Snowflake {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> IsNull {
        buf.extend(&(self.0 as i64).to_le_bytes());
        IsNull::No
    }
}
