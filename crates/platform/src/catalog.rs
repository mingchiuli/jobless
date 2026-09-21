use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::PlatformId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlatformConfig {
    pub enabled: bool,
    pub display_name: String,
    pub home_url: String,
    pub sort_order: u32,
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            display_name: String::new(),
            home_url: "about:blank".to_string(),
            sort_order: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformDescriptor {
    pub id: PlatformId,
    pub enabled: bool,
    pub display_name: String,
    pub home_url: String,
    pub sort_order: u32,
}

#[derive(Debug, Clone, Default)]
pub struct PlatformCatalog {
    descriptors: Vec<PlatformDescriptor>,
}

impl PlatformCatalog {
    pub fn from_config(
        platforms: &BTreeMap<PlatformId, PlatformConfig>,
    ) -> Result<Self, PlatformError> {
        let mut descriptors = platforms
            .iter()
            .map(|(id, config)| {
                validate_home_url(&config.home_url)?;
                if config.display_name.trim().is_empty() {
                    return Err(PlatformError::EmptyDisplayName(id.clone()));
                }
                Ok(PlatformDescriptor {
                    id: id.clone(),
                    enabled: config.enabled,
                    display_name: config.display_name.trim().to_string(),
                    home_url: config.home_url.clone(),
                    sort_order: config.sort_order,
                })
            })
            .collect::<Result<Vec<_>, PlatformError>>()?;
        descriptors.sort_by(|left, right| {
            left.sort_order
                .cmp(&right.sort_order)
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(Self { descriptors })
    }

    pub fn all(&self) -> &[PlatformDescriptor] {
        &self.descriptors
    }

    pub fn enabled(&self) -> impl Iterator<Item = &PlatformDescriptor> {
        self.descriptors
            .iter()
            .filter(|descriptor| descriptor.enabled)
    }

    pub fn get(&self, id: &PlatformId) -> Option<&PlatformDescriptor> {
        self.descriptors
            .iter()
            .find(|descriptor| &descriptor.id == id)
    }

    pub fn first_enabled(&self) -> Option<&PlatformDescriptor> {
        self.enabled().next()
    }
}

pub fn compatibility_platform() -> PlatformDescriptor {
    PlatformDescriptor {
        id: PlatformId::compatibility(),
        enabled: true,
        display_name: "Test page".to_string(),
        home_url: "about:blank".to_string(),
        sort_order: u32::MAX,
    }
}

fn validate_home_url(url: &str) -> Result<(), PlatformError> {
    if url == "about:blank" || url.starts_with("http://") || url.starts_with("https://") {
        return Ok(());
    }
    Err(PlatformError::InvalidHomeUrl(url.to_string()))
}

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("invalid platform id `{0}`")]
    InvalidId(String),
    #[error("platform `{0}` has an empty display name")]
    EmptyDisplayName(PlatformId),
    #[error("platform home URL must be http, https, or about:blank: `{0}`")]
    InvalidHomeUrl(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_enabled_platforms() {
        let mut configs = BTreeMap::new();
        configs.insert(
            PlatformId::new("boss").unwrap(),
            PlatformConfig {
                display_name: "BOSS".to_string(),
                home_url: "https://www.zhipin.com".to_string(),
                sort_order: 20,
                ..PlatformConfig::default()
            },
        );
        configs.insert(
            PlatformId::new("liepin").unwrap(),
            PlatformConfig {
                display_name: "猎聘".to_string(),
                home_url: "https://www.liepin.com".to_string(),
                sort_order: 10,
                ..PlatformConfig::default()
            },
        );

        let catalog = PlatformCatalog::from_config(&configs).unwrap();
        let ids = catalog
            .enabled()
            .map(|platform| platform.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, ["liepin", "boss"]);
    }
}
