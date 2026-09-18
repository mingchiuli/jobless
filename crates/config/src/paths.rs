use std::path::{Path, PathBuf};

use directories::BaseDirs;
use thiserror::Error;

pub const BUNDLE_ID: &str = "com.jobless.desktop";

#[derive(Debug, Error)]
pub enum PathError {
    #[error("the current user has no platform config/data directory")]
    NoBaseDirectories,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub config_file: PathBuf,
    pub data_dir: PathBuf,
    pub database_file: PathBuf,
    pub browser_profiles_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub runtime_dir: PathBuf,
}

impl AppPaths {
    pub fn discover() -> Result<Self, PathError> {
        Self::with_overrides(
            std::env::var_os("JOBLESS_CONFIG_FILE").map(PathBuf::from),
            std::env::var_os("JOBLESS_DATA_DIR").map(PathBuf::from),
            std::env::var_os("JOBLESS_RUNTIME_DIR").map(PathBuf::from),
        )
    }

    pub fn with_overrides(
        config_file: Option<PathBuf>,
        data_dir: Option<PathBuf>,
        runtime_dir: Option<PathBuf>,
    ) -> Result<Self, PathError> {
        let base = BaseDirs::new().ok_or(PathError::NoBaseDirectories)?;

        let config_file =
            config_file.unwrap_or_else(|| base.config_dir().join(BUNDLE_ID).join("config.toml"));
        let config_dir = config_file
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| base.config_dir().join(BUNDLE_ID));
        let data_dir = data_dir.unwrap_or_else(|| base.data_local_dir().join(BUNDLE_ID));
        let runtime_dir = runtime_dir.unwrap_or_else(default_runtime_dir);

        Ok(Self::from_normalized(
            config_dir,
            config_file,
            data_dir,
            runtime_dir,
        ))
    }

    pub fn set_data_dir(&mut self, data_dir: PathBuf) {
        let runtime_dir = self.runtime_dir.clone();
        let config_dir = self.config_dir.clone();
        let config_file = self.config_file.clone();
        *self = Self::from_normalized(config_dir, config_file, data_dir, runtime_dir);
    }

    pub fn set_runtime_dir(&mut self, runtime_dir: PathBuf) {
        self.runtime_dir = runtime_dir;
    }

    fn from_normalized(
        config_dir: PathBuf,
        config_file: PathBuf,
        data_dir: PathBuf,
        runtime_dir: PathBuf,
    ) -> Self {
        let database_file = data_dir.join("db").join("jobless.sqlite3");
        let browser_profiles_dir = data_dir.join("browser-profiles");
        let logs_dir = data_dir.join("logs");

        Self {
            config_dir,
            config_file,
            data_dir,
            database_file,
            browser_profiles_dir,
            logs_dir,
            runtime_dir,
        }
    }
}

fn default_runtime_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe()
        && let Some(exe_dir) = exe.parent()
    {
        let packaged = exe_dir.join("runtime");
        if packaged.exists() {
            return packaged;
        }

        if let Some(contents_dir) = exe_dir.parent() {
            let macos_bundle = contents_dir.join("Resources").join("runtime");
            if macos_bundle.exists() {
                return macos_bundle;
            }
        }
    }

    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("runtime")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_overrides_are_normalized() {
        let root = tempfile::tempdir().unwrap();
        let paths = AppPaths::with_overrides(
            Some(root.path().join("config/config.toml")),
            Some(root.path().join("data")),
            Some(root.path().join("runtime")),
        )
        .unwrap();

        assert_eq!(paths.config_file, root.path().join("config/config.toml"));
        assert_eq!(paths.config_dir, root.path().join("config"));
        assert_eq!(
            paths.database_file,
            root.path().join("data/db/jobless.sqlite3")
        );
        assert_eq!(
            paths.browser_profiles_dir,
            root.path().join("data/browser-profiles")
        );
    }
}
