use std::collections::HashMap;

use jobless_browser::{
    BrowserError, BrowserManager, BrowserManagerConfig, BrowserPage, BrowserSession, PageSnapshot,
    RuntimeHealthResult, RuntimeNotification, SessionId,
};
use jobless_config::{AppConfig, AppPaths};
use jobless_platform::PlatformId;
use thiserror::Error;

pub struct RuntimeService {
    driver: Box<dyn BrowserDriver>,
    profile_id: String,
    sessions: HashMap<SessionId, BrowserSession>,
    primary_session: Option<SessionId>,
}

impl RuntimeService {
    pub fn from_config(config: &AppConfig, paths: &AppPaths) -> Result<Self, RuntimeServiceError> {
        let manager_config = BrowserManagerConfig::from_app(config, paths)?;
        Ok(Self::with_driver(
            Box::new(ManagedBrowserDriver::new(manager_config)),
            config.browser.profile_id.clone(),
        ))
    }

    fn with_driver(driver: Box<dyn BrowserDriver>, profile_id: String) -> Self {
        Self {
            driver,
            profile_id,
            sessions: HashMap::new(),
            primary_session: None,
        }
    }

    pub fn is_running(&self) -> bool {
        self.driver.is_running()
    }

    pub fn is_browser_open(&self) -> bool {
        self.primary_session.is_some()
    }

    pub fn health(&self) -> Option<RuntimeHealthResult> {
        self.driver.health().ok()
    }

    pub fn start(&mut self) -> Result<(), RuntimeServiceError> {
        self.driver.start()?;
        Ok(())
    }

    pub fn start_session(&mut self) -> Result<BrowserSession, RuntimeServiceError> {
        if let Some(session_id) = &self.primary_session
            && let Some(session) = self.sessions.get(session_id)
        {
            return Ok(session.clone());
        }
        self.require_running()?;

        let session = self.driver.start_session(&self.profile_id)?;
        self.sessions
            .insert(session.session_id().clone(), session.clone());
        self.primary_session = Some(session.session_id().clone());
        Ok(session)
    }

    pub fn close_session(
        &mut self,
        session_id: Option<&SessionId>,
    ) -> Result<u32, RuntimeServiceError> {
        let closed = self.driver.close_session(session_id)?;
        match session_id {
            Some(session_id) => {
                self.sessions.remove(session_id);
                if self.primary_session.as_ref() == Some(session_id) {
                    self.primary_session = None;
                }
            }
            None => {
                self.sessions.clear();
                self.primary_session = None;
            }
        }
        Ok(closed)
    }

    pub fn ensure_platform_page(
        &mut self,
        platform_id: &PlatformId,
        url: &str,
    ) -> Result<BrowserPage, RuntimeServiceError> {
        let session = self.start_session()?;
        Ok(self.driver.ensure_page(&session, platform_id, url)?)
    }

    pub fn navigate_platform_page(
        &mut self,
        platform_id: &PlatformId,
        url: &str,
    ) -> Result<BrowserPage, RuntimeServiceError> {
        let session = self.primary_session()?;
        Ok(self.driver.navigate_page(&session, platform_id, url)?)
    }

    pub fn activate_platform_page(
        &mut self,
        platform_id: &PlatformId,
    ) -> Result<bool, RuntimeServiceError> {
        let session = self.primary_session()?;
        Ok(self.driver.activate_page(&session, platform_id)?)
    }

    pub fn close_platform_page(
        &mut self,
        platform_id: &PlatformId,
    ) -> Result<bool, RuntimeServiceError> {
        let session = self.primary_session()?;
        Ok(self.driver.close_page(&session, platform_id)?)
    }

    pub fn list_platform_pages(&mut self) -> Result<Vec<PageSnapshot>, RuntimeServiceError> {
        let session = self.primary_session()?;
        Ok(self.driver.list_pages(&session)?)
    }

    pub fn drain_notifications(&mut self) -> Vec<RuntimeNotification> {
        let notifications = self.driver.drain_notifications();
        for notification in &notifications {
            if let RuntimeNotification::SessionEvent(event) = notification {
                self.sessions.remove(&event.session_id);
                if self.primary_session.as_ref() == Some(&event.session_id) {
                    self.primary_session = None;
                }
            }
        }
        notifications
    }

    pub fn stop(&mut self) -> Result<(), RuntimeServiceError> {
        self.sessions.clear();
        self.primary_session = None;
        self.driver.shutdown()?;
        Ok(())
    }

    fn primary_session(&self) -> Result<BrowserSession, RuntimeServiceError> {
        let session_id = self
            .primary_session
            .as_ref()
            .ok_or(RuntimeServiceError::SessionNotStarted)?;
        self.sessions
            .get(session_id)
            .cloned()
            .ok_or(RuntimeServiceError::SessionNotStarted)
    }

    fn require_running(&self) -> Result<(), RuntimeServiceError> {
        if self.is_running() {
            Ok(())
        } else {
            Err(RuntimeServiceError::NotRunning)
        }
    }
}

trait BrowserDriver: Send {
    fn is_running(&self) -> bool;
    fn start(&mut self) -> Result<(), DriverError>;
    fn health(&self) -> Result<RuntimeHealthResult, DriverError>;
    fn start_session(&self, profile_id: &str) -> Result<BrowserSession, DriverError>;
    fn close_session(&self, session_id: Option<&SessionId>) -> Result<u32, DriverError>;
    fn ensure_page(
        &self,
        session: &BrowserSession,
        platform_id: &PlatformId,
        url: &str,
    ) -> Result<BrowserPage, DriverError>;
    fn navigate_page(
        &self,
        session: &BrowserSession,
        platform_id: &PlatformId,
        url: &str,
    ) -> Result<BrowserPage, DriverError>;
    fn activate_page(
        &self,
        session: &BrowserSession,
        platform_id: &PlatformId,
    ) -> Result<bool, DriverError>;
    fn close_page(
        &self,
        session: &BrowserSession,
        platform_id: &PlatformId,
    ) -> Result<bool, DriverError>;
    fn list_pages(&self, session: &BrowserSession) -> Result<Vec<PageSnapshot>, DriverError>;
    fn drain_notifications(&mut self) -> Vec<RuntimeNotification>;
    fn shutdown(&mut self) -> Result<(), DriverError>;
}

struct ManagedBrowserDriver {
    config: BrowserManagerConfig,
    manager: Option<BrowserManager>,
}

impl ManagedBrowserDriver {
    fn new(config: BrowserManagerConfig) -> Self {
        Self {
            config,
            manager: None,
        }
    }
}

impl BrowserDriver for ManagedBrowserDriver {
    fn is_running(&self) -> bool {
        self.manager.is_some()
    }

    fn start(&mut self) -> Result<(), DriverError> {
        if self.manager.is_none() {
            self.manager = Some(BrowserManager::start(self.config.clone())?);
        }
        Ok(())
    }

    fn health(&self) -> Result<RuntimeHealthResult, DriverError> {
        Ok(self
            .manager
            .as_ref()
            .ok_or(DriverError::NotRunning)?
            .health()?)
    }

    fn start_session(&self, profile_id: &str) -> Result<BrowserSession, DriverError> {
        Ok(self
            .manager
            .as_ref()
            .ok_or(DriverError::NotRunning)?
            .start_session(profile_id)?)
    }

    fn close_session(&self, session_id: Option<&SessionId>) -> Result<u32, DriverError> {
        Ok(self
            .manager
            .as_ref()
            .ok_or(DriverError::NotRunning)?
            .close_session(session_id)?)
    }

    fn ensure_page(
        &self,
        session: &BrowserSession,
        platform_id: &PlatformId,
        url: &str,
    ) -> Result<BrowserPage, DriverError> {
        Ok(self
            .manager
            .as_ref()
            .ok_or(DriverError::NotRunning)?
            .ensure_page(session, platform_id, url)?)
    }

    fn navigate_page(
        &self,
        session: &BrowserSession,
        platform_id: &PlatformId,
        url: &str,
    ) -> Result<BrowserPage, DriverError> {
        Ok(self
            .manager
            .as_ref()
            .ok_or(DriverError::NotRunning)?
            .navigate_page(session, platform_id, url)?)
    }

    fn activate_page(
        &self,
        session: &BrowserSession,
        platform_id: &PlatformId,
    ) -> Result<bool, DriverError> {
        Ok(self
            .manager
            .as_ref()
            .ok_or(DriverError::NotRunning)?
            .activate_page(session, platform_id)?)
    }

    fn close_page(
        &self,
        session: &BrowserSession,
        platform_id: &PlatformId,
    ) -> Result<bool, DriverError> {
        Ok(self
            .manager
            .as_ref()
            .ok_or(DriverError::NotRunning)?
            .close_page(session, platform_id)?)
    }

    fn list_pages(&self, session: &BrowserSession) -> Result<Vec<PageSnapshot>, DriverError> {
        Ok(self
            .manager
            .as_ref()
            .ok_or(DriverError::NotRunning)?
            .list_pages(session)?)
    }

    fn drain_notifications(&mut self) -> Vec<RuntimeNotification> {
        self.manager
            .as_mut()
            .map(BrowserManager::drain_notifications)
            .unwrap_or_default()
    }

    fn shutdown(&mut self) -> Result<(), DriverError> {
        if let Some(mut manager) = self.manager.take() {
            manager.shutdown()?;
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum DriverError {
    #[error(transparent)]
    Browser(#[from] BrowserError),
    #[error("browser runtime is not running")]
    NotRunning,
}

#[derive(Debug, Error)]
pub enum RuntimeServiceError {
    #[error("browser runtime is not running")]
    NotRunning,
    #[error("browser session has not been started")]
    SessionNotStarted,
    #[error(transparent)]
    Browser(#[from] BrowserError),
    #[error(transparent)]
    Driver(#[from] DriverError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use jobless_browser::{BrowserEngineInfo, BrowserInfo, PageId, RuntimeInfo};

    struct FakeDriver {
        running: bool,
        pages: HashMap<PlatformId, PageId>,
    }

    impl FakeDriver {
        fn new() -> Self {
            Self {
                running: false,
                pages: HashMap::new(),
            }
        }

        fn page(&self, platform_id: &PlatformId, url: &str) -> BrowserPage {
            let page_id = self
                .pages
                .get(platform_id)
                .cloned()
                .unwrap_or_else(|| PageId::new(format!("page-{}", platform_id)));
            BrowserPage::new(page_id, platform_id.clone(), url.to_string())
        }
    }

    impl BrowserDriver for FakeDriver {
        fn is_running(&self) -> bool {
            self.running
        }

        fn start(&mut self) -> Result<(), DriverError> {
            self.running = true;
            Ok(())
        }

        fn health(&self) -> Result<RuntimeHealthResult, DriverError> {
            Ok(RuntimeHealthResult {
                runtime: RuntimeInfo {
                    name: "fake".to_string(),
                    version: "1".to_string(),
                    node_compat_version: "1".to_string(),
                },
                engine: BrowserEngineInfo {
                    name: "fake".to_string(),
                    version: "1".to_string(),
                },
                browser: BrowserInfo {
                    available: true,
                    ..BrowserInfo::default()
                },
            })
        }

        fn start_session(&self, _profile_id: &str) -> Result<BrowserSession, DriverError> {
            Ok(BrowserSession::new(SessionId::new("session")))
        }

        fn close_session(&self, _session_id: Option<&SessionId>) -> Result<u32, DriverError> {
            Ok(1)
        }

        fn ensure_page(
            &self,
            _session: &BrowserSession,
            platform_id: &PlatformId,
            url: &str,
        ) -> Result<BrowserPage, DriverError> {
            Ok(self.page(platform_id, url))
        }

        fn navigate_page(
            &self,
            _session: &BrowserSession,
            platform_id: &PlatformId,
            url: &str,
        ) -> Result<BrowserPage, DriverError> {
            Ok(self.page(platform_id, url))
        }

        fn activate_page(
            &self,
            _session: &BrowserSession,
            platform_id: &PlatformId,
        ) -> Result<bool, DriverError> {
            Ok(self.pages.contains_key(platform_id))
        }

        fn close_page(
            &self,
            _session: &BrowserSession,
            platform_id: &PlatformId,
        ) -> Result<bool, DriverError> {
            Ok(self.pages.contains_key(platform_id))
        }

        fn list_pages(&self, _session: &BrowserSession) -> Result<Vec<PageSnapshot>, DriverError> {
            Ok(Vec::new())
        }

        fn drain_notifications(&mut self) -> Vec<RuntimeNotification> {
            Vec::new()
        }

        fn shutdown(&mut self) -> Result<(), DriverError> {
            self.running = false;
            Ok(())
        }
    }

    #[test]
    fn lazy_starts_session_for_platform_pages() {
        let mut service =
            RuntimeService::with_driver(Box::new(FakeDriver::new()), "default".to_string());
        let platform = PlatformId::new("boss").unwrap();

        assert!(matches!(
            service.ensure_platform_page(&platform, "about:blank"),
            Err(RuntimeServiceError::NotRunning)
        ));
        service.start().unwrap();
        let page = service
            .ensure_platform_page(&platform, "about:blank")
            .unwrap();
        assert_eq!(page.platform_id(), &platform);
        assert!(service.is_browser_open());
    }

    #[test]
    fn notifications_clear_closed_sessions() {
        let mut service =
            RuntimeService::with_driver(Box::new(FakeDriver::new()), "default".to_string());
        service.start().unwrap();
        service.start_session().unwrap();
        assert!(service.is_browser_open());

        service.sessions.clear();
        service.primary_session = None;
        assert!(!service.is_browser_open());
    }
}
