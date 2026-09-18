use jobless_browser::{
    BrowserManager, BrowserManagerConfig, BrowserSession, RuntimeHealthResult, RuntimeNotification,
};
use jobless_config::{AppConfig, AppPaths};
use thiserror::Error;

pub struct RuntimeService {
    config: BrowserManagerConfig,
    manager: Option<BrowserManager>,
    session: Option<BrowserSession>,
    health: Option<RuntimeHealthResult>,
}

impl RuntimeService {
    pub fn new(config: BrowserManagerConfig) -> Self {
        Self {
            config,
            manager: None,
            session: None,
            health: None,
        }
    }

    pub fn from_config(config: &AppConfig, paths: &AppPaths) -> Result<Self, RuntimeServiceError> {
        Ok(Self::new(BrowserManagerConfig::from_app(config, paths)?))
    }

    pub fn is_running(&self) -> bool {
        self.manager.is_some()
    }

    pub fn is_browser_open(&self) -> bool {
        self.session.is_some()
    }

    pub fn health(&self) -> Option<&RuntimeHealthResult> {
        self.health.as_ref()
    }

    pub fn start(&mut self) -> Result<(), RuntimeServiceError> {
        if self.manager.is_some() {
            return Ok(());
        }

        let manager = BrowserManager::start(self.config.clone())?;
        let health = manager.health()?;
        self.manager = Some(manager);
        self.health = Some(health);
        Ok(())
    }

    pub fn open(
        &mut self,
        url: impl Into<String>,
        profile_id: impl Into<String>,
    ) -> Result<BrowserSession, RuntimeServiceError> {
        let manager = self
            .manager
            .as_ref()
            .ok_or(RuntimeServiceError::NotRunning)?;
        let session = manager.open(url, profile_id)?;
        self.session = Some(session.clone());
        Ok(session)
    }

    pub fn close(&mut self) -> Result<u32, RuntimeServiceError> {
        let manager = self
            .manager
            .as_ref()
            .ok_or(RuntimeServiceError::NotRunning)?;
        let session_id = self.session.take().map(|session| session.session_id);
        Ok(manager.close(session_id)?)
    }

    pub fn stop(&mut self) -> Result<(), RuntimeServiceError> {
        self.session = None;
        self.health = None;
        if let Some(mut manager) = self.manager.take() {
            manager.shutdown()?;
        }
        Ok(())
    }

    pub fn drain_notifications(&mut self) -> Vec<RuntimeNotification> {
        self.manager
            .as_mut()
            .map(BrowserManager::drain_notifications)
            .unwrap_or_default()
    }
}

#[derive(Debug, Error)]
pub enum RuntimeServiceError {
    #[error("browser runtime is not running")]
    NotRunning,
    #[error(transparent)]
    Browser(#[from] jobless_browser::BrowserError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use jobless_browser::RuntimePaths;
    use std::path::PathBuf;
    use std::time::Duration;

    fn service() -> RuntimeService {
        RuntimeService::new(BrowserManagerConfig {
            runtime: RuntimePaths {
                runtime_dir: PathBuf::from("/missing"),
                runtime_bin: PathBuf::from("/missing/bun"),
                worker_entry: PathBuf::from("/missing/main.ts"),
                chromium_dir: PathBuf::from("/missing/chromium"),
            },
            data_dir: PathBuf::from("/missing/data"),
            engine: jobless_config::BrowserEngine::Patchright,
            headless: true,
            request_timeout: Duration::from_millis(1),
            shutdown_timeout: Duration::from_millis(1),
            max_frame_bytes: 1024,
        })
    }

    #[test]
    fn operations_require_a_running_runtime() {
        let mut service = service();
        assert!(!service.is_running());
        assert!(matches!(
            service.open("about:blank", "default"),
            Err(RuntimeServiceError::NotRunning)
        ));
    }
}
