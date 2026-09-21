use jobless_platform::PlatformId;

use super::{BrowserError, BrowserManager};
use crate::protocol::{
    PageActivateParams, PageActivateResult, PageCloseParams, PageCloseResult, PageEnsureParams,
    PageEnsureResult, PageId, PageListParams, PageListResult, PageNavigateParams,
    PageNavigateResult, PageSnapshot, SessionCloseParams, SessionCloseResult, SessionId,
    SessionStartParams, SessionStartResult, method,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserSession {
    session_id: SessionId,
}

impl BrowserSession {
    pub fn new(session_id: SessionId) -> Self {
        Self { session_id }
    }

    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserPage {
    page_id: PageId,
    platform_id: PlatformId,
    url: String,
}

impl BrowserPage {
    pub fn new(page_id: PageId, platform_id: PlatformId, url: String) -> Self {
        Self {
            page_id,
            platform_id,
            url,
        }
    }

    pub fn page_id(&self) -> &PageId {
        &self.page_id
    }

    pub fn platform_id(&self) -> &PlatformId {
        &self.platform_id
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

impl BrowserManager {
    pub fn start_session(&self, profile_id: &str) -> Result<BrowserSession, BrowserError> {
        let result: SessionStartResult = self.call(
            method::SESSION_START,
            &SessionStartParams {
                profile_id: profile_id.to_string(),
            },
        )?;
        Ok(BrowserSession {
            session_id: result.session_id,
        })
    }

    pub fn close_session(&self, session_id: Option<&SessionId>) -> Result<u32, BrowserError> {
        let result: SessionCloseResult = self.call(
            method::SESSION_CLOSE,
            &SessionCloseParams {
                session_id: session_id.cloned(),
            },
        )?;
        Ok(result.closed)
    }

    pub fn ensure_page(
        &self,
        session: &BrowserSession,
        platform_id: &PlatformId,
        url: &str,
    ) -> Result<BrowserPage, BrowserError> {
        let result: PageEnsureResult = self.call(
            method::PAGE_ENSURE,
            &PageEnsureParams {
                session_id: session.session_id.clone(),
                platform_id: platform_id.as_str().to_string(),
                url: url.to_string(),
            },
        )?;
        Ok(BrowserPage {
            page_id: result.page_id,
            platform_id: platform_id.clone(),
            url: result.url,
        })
    }

    pub fn navigate_page(
        &self,
        session: &BrowserSession,
        platform_id: &PlatformId,
        url: &str,
    ) -> Result<BrowserPage, BrowserError> {
        let result: PageNavigateResult = self.call(
            method::PAGE_NAVIGATE,
            &PageNavigateParams {
                session_id: session.session_id.clone(),
                platform_id: platform_id.as_str().to_string(),
                url: url.to_string(),
            },
        )?;
        Ok(BrowserPage {
            page_id: result.page_id,
            platform_id: platform_id.clone(),
            url: result.url,
        })
    }

    pub fn activate_page(
        &self,
        session: &BrowserSession,
        platform_id: &PlatformId,
    ) -> Result<bool, BrowserError> {
        let result: PageActivateResult = self.call(
            method::PAGE_ACTIVATE,
            &PageActivateParams {
                session_id: session.session_id.clone(),
                platform_id: platform_id.as_str().to_string(),
            },
        )?;
        Ok(result.activated)
    }

    pub fn close_page(
        &self,
        session: &BrowserSession,
        platform_id: &PlatformId,
    ) -> Result<bool, BrowserError> {
        let result: PageCloseResult = self.call(
            method::PAGE_CLOSE,
            &PageCloseParams {
                session_id: session.session_id.clone(),
                platform_id: platform_id.as_str().to_string(),
            },
        )?;
        Ok(result.closed)
    }

    pub fn list_pages(&self, session: &BrowserSession) -> Result<Vec<PageSnapshot>, BrowserError> {
        let result: PageListResult = self.call(
            method::PAGE_LIST,
            &PageListParams {
                session_id: session.session_id.clone(),
            },
        )?;
        Ok(result.pages)
    }
}
