use std::fmt;
use std::str::FromStr;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ts_rs::TS;

use crate::catalog::PlatformError;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, TS)]
#[ts(type = "string")]
pub struct PlatformId(String);

impl PlatformId {
    pub fn new(value: impl Into<String>) -> Result<Self, PlatformError> {
        let value = value.into();
        let valid = (1..=64).contains(&value.len())
            && value.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
            });

        if !valid {
            return Err(PlatformError::InvalidId(value));
        }

        Ok(Self(value))
    }

    pub fn compatibility() -> Self {
        Self("default".to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PlatformId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for PlatformId {
    type Err = PlatformError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl Serialize for PlatformId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for PlatformId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_platform_ids() {
        assert!(PlatformId::new("boss").is_ok());
        assert!(PlatformId::new("boss-main_1").is_ok());
        assert!(PlatformId::new("BOSS").is_err());
        assert!(PlatformId::new("../boss").is_err());
        assert!(PlatformId::new("").is_err());
    }
}
