use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

use atomic_write_file::AtomicWriteFile;
use config::{Config, Environment, File as ConfigFile, FileFormat, Map};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::paths::{AppPaths, PathError};

pub const APP_LOCALE_ZH_CN: &str = "zh-CN";
pub const APP_LOCALE_EN: &str = "en";

const DEFAULT_CONFIG: &str = include_str!("../default.toml");
const MAX_REQUEST_TIMEOUT_MS: u64 = 10 * 60 * 1000;
const MIN_FRAME_BYTES: usize = 1_024;
const MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AppConfig {
    pub app: AppSection,
    pub storage: StorageSection,
    pub runtime: RuntimeSection,
    pub browser: BrowserSection,
    pub rpc: RpcSection,
    pub logging: LoggingSection,
}

impl AppConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if !matches!(self.app.locale.as_str(), APP_LOCALE_ZH_CN | APP_LOCALE_EN) {
            return Err(ConfigError::Validation(format!(
                "app.locale must be `{APP_LOCALE_ZH_CN}` or `{APP_LOCALE_EN}`"
            )));
        }

        if self.rpc.request_timeout_ms == 0 || self.rpc.request_timeout_ms > MAX_REQUEST_TIMEOUT_MS
        {
            return Err(ConfigError::Validation(format!(
                "rpc.request_timeout_ms must be between 1 and {MAX_REQUEST_TIMEOUT_MS}"
            )));
        }

        if self.rpc.shutdown_timeout_ms == 0
            || self.rpc.shutdown_timeout_ms > MAX_REQUEST_TIMEOUT_MS
        {
            return Err(ConfigError::Validation(format!(
                "rpc.shutdown_timeout_ms must be between 1 and {MAX_REQUEST_TIMEOUT_MS}"
            )));
        }

        if !(MIN_FRAME_BYTES..=MAX_FRAME_BYTES).contains(&self.rpc.max_frame_bytes) {
            return Err(ConfigError::Validation(format!(
                "rpc.max_frame_bytes must be between {MIN_FRAME_BYTES} and {MAX_FRAME_BYTES}"
            )));
        }

        if self.browser.smoke_url.trim().is_empty() {
            return Err(ConfigError::Validation(
                "browser.smoke_url must not be empty".to_string(),
            ));
        }

        if self.logging.level.trim().is_empty() {
            return Err(ConfigError::Validation(
                "logging.level must not be empty".to_string(),
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AppSection {
    pub locale: String,
}

impl Default for AppSection {
    fn default() -> Self {
        Self {
            locale: APP_LOCALE_ZH_CN.to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct StorageSection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct RuntimeSection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bin: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker_entry: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct BrowserSection {
    pub engine: BrowserEngine,
    pub headless: bool,
    pub smoke_url: String,
}

impl Default for BrowserSection {
    fn default() -> Self {
        Self {
            engine: BrowserEngine::Patchright,
            headless: false,
            smoke_url: "about:blank".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BrowserEngine {
    #[default]
    Patchright,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct RpcSection {
    pub request_timeout_ms: u64,
    pub shutdown_timeout_ms: u64,
    pub max_frame_bytes: usize,
}

impl Default for RpcSection {
    fn default() -> Self {
        Self {
            request_timeout_ms: 10_000,
            shutdown_timeout_ms: 3_000,
            max_frame_bytes: 4 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct LoggingSection {
    pub level: String,
}

impl Default for LoggingSection {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSourceKind {
    Defaults,
    UserFile,
    Environment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigSource {
    pub kind: ConfigSourceKind,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct LoadedConfig {
    pub value: AppConfig,
    pub paths: AppPaths,
    pub sources: Vec<ConfigSource>,
}

#[derive(Debug, Clone)]
pub struct ConfigStore {
    paths: AppPaths,
}

impl ConfigStore {
    pub fn new(paths: AppPaths) -> Self {
        Self { paths }
    }

    pub fn paths(&self) -> &AppPaths {
        &self.paths
    }

    pub fn ensure_user_file(&self) -> Result<bool, ConfigError> {
        if self.paths.config_file.exists() {
            return Ok(false);
        }

        if let Some(parent) = self.paths.config_file.parent() {
            std::fs::create_dir_all(parent).map_err(|source| ConfigError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        write_atomic(&self.paths.config_file, DEFAULT_CONFIG.as_bytes())?;
        Ok(true)
    }

    pub fn load(&self) -> Result<LoadedConfig, ConfigError> {
        self.ensure_user_file()?;
        self.load_from_environment(None)
    }

    pub fn load_with_environment(
        &self,
        environment: HashMap<String, String>,
    ) -> Result<LoadedConfig, ConfigError> {
        self.ensure_user_file()?;
        let mut values = Map::new();
        values.extend(environment);
        self.load_from_environment(Some(values))
    }

    pub fn save(&self, value: &AppConfig) -> Result<(), ConfigError> {
        value.validate()?;
        if let Some(parent) = self.paths.config_file.parent() {
            std::fs::create_dir_all(parent).map_err(|source| ConfigError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let serialized = toml::to_string_pretty(value)?;
        write_atomic(&self.paths.config_file, serialized.as_bytes())?;
        Ok(())
    }

    fn load_from_environment(
        &self,
        environment: Option<Map<String, String>>,
    ) -> Result<LoadedConfig, ConfigError> {
        let environment = Environment::with_prefix("JOBLESS")
            .prefix_separator("__")
            .separator("__")
            .try_parsing(true)
            .source(environment);

        let settings = Config::builder()
            .add_source(ConfigFile::from_str(DEFAULT_CONFIG, FileFormat::Toml).required(false))
            .add_source(
                ConfigFile::from(self.paths.config_file.clone())
                    .format(FileFormat::Toml)
                    .required(false),
            )
            .add_source(environment)
            .build()?;

        let value: AppConfig = settings.try_deserialize()?;
        value.validate()?;

        let mut effective_paths = self.paths.clone();
        if let Some(data_dir) = &value.storage.data_dir {
            effective_paths.set_data_dir(data_dir.clone());
        }
        if let Some(runtime_dir) = &value.runtime.dir {
            effective_paths.set_runtime_dir(runtime_dir.clone());
        }

        Ok(LoadedConfig {
            value,
            paths: effective_paths,
            sources: vec![
                ConfigSource {
                    kind: ConfigSourceKind::Defaults,
                    path: None,
                },
                ConfigSource {
                    kind: ConfigSourceKind::UserFile,
                    path: Some(self.paths.config_file.clone()),
                },
                ConfigSource {
                    kind: ConfigSourceKind::Environment,
                    path: None,
                },
            ],
        })
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error(transparent)]
    Paths(#[from] PathError),
    #[error("failed to read or write `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("configuration error: {0}")]
    Config(#[from] config::ConfigError),
    #[error("invalid configuration: {0}")]
    Validation(String),
    #[error("failed to serialize configuration: {0}")]
    Serialize(#[from] toml::ser::Error),
}

fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> Result<(), ConfigError> {
    let mut file = AtomicWriteFile::open(path).map_err(|source| ConfigError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    file.write_all(bytes).map_err(|source| ConfigError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    file.commit().map_err(|source| ConfigError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> (tempfile::TempDir, ConfigStore) {
        let root = tempfile::tempdir().unwrap();
        let paths = AppPaths::with_overrides(
            Some(root.path().join("config/config.toml")),
            Some(root.path().join("data")),
            Some(root.path().join("runtime")),
        )
        .unwrap();
        let store = ConfigStore::new(paths);
        (root, store)
    }

    #[test]
    fn creates_and_loads_user_config() {
        let (root, store) = test_store();
        let loaded = store.load().unwrap();

        assert!(store.paths().config_file.exists());
        assert_eq!(loaded.value.browser.engine, BrowserEngine::Patchright);
        assert_eq!(loaded.paths.data_dir, root.path().join("data"));
    }

    #[test]
    fn environment_overrides_user_config() {
        let (_root, store) = test_store();
        store
            .save(&AppConfig {
                logging: LoggingSection {
                    level: "warn".to_string(),
                },
                ..AppConfig::default()
            })
            .unwrap();

        let loaded = store
            .load_with_environment(HashMap::from([(
                "JOBLESS__LOGGING__LEVEL".to_string(),
                "debug".to_string(),
            )]))
            .unwrap();

        assert_eq!(loaded.value.logging.level, "debug");
    }

    #[test]
    fn rejects_invalid_timeout() {
        let (_root, store) = test_store();
        let value = AppConfig {
            rpc: RpcSection {
                request_timeout_ms: 0,
                ..RpcSection::default()
            },
            ..AppConfig::default()
        };

        assert!(value.validate().is_err());
        assert!(store.save(&value).is_err());
    }

    #[test]
    fn config_data_dir_override_updates_effective_paths() {
        let (root, store) = test_store();
        let custom = root.path().join("custom-data");
        let mut value = AppConfig::default();
        value.storage.data_dir = Some(custom.clone());
        store.save(&value).unwrap();

        let loaded = store.load().unwrap();
        assert_eq!(loaded.paths.data_dir, custom);
        assert_eq!(
            loaded.paths.database_file,
            root.path().join("custom-data/db/jobless.sqlite3")
        );
    }
}
