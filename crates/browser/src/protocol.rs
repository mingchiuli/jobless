use std::fmt;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub mod method {
    pub const RUNTIME_READY: &str = "runtime.ready";
    pub const RUNTIME_HEALTH: &str = "runtime.health";
    pub const RUNTIME_SHUTDOWN: &str = "runtime.shutdown";
    pub const SESSION_START: &str = "browser.session.start";
    pub const SESSION_CLOSE: &str = "browser.session.close";
    pub const SESSION_EVENT: &str = "browser.session.event";
    pub const PAGE_ENSURE: &str = "browser.page.ensure";
    pub const PAGE_NAVIGATE: &str = "browser.page.navigate";
    pub const PAGE_ACTIVATE: &str = "browser.page.activate";
    pub const PAGE_CLOSE: &str = "browser.page.close";
    pub const PAGE_LIST: &str = "browser.page.list";
    pub const PAGE_EVENT: &str = "browser.page.event";
    pub const RUNTIME_PROTOCOL_ERROR: &str = "runtime.protocol_error";
    pub const CANCEL_REQUEST: &str = "$/cancelRequest";
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct RuntimeReadyParams {}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct RuntimeHealthParams {}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct RuntimeHealthResult {
    pub runtime: RuntimeInfo,
    pub engine: BrowserEngineInfo,
    pub browser: BrowserInfo,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct RuntimeInfo {
    pub name: String,
    pub version: String,
    pub node_compat_version: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct BrowserEngineInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct BrowserInfo {
    pub available: bool,
    pub executable_path: Option<String>,
    pub revision: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, TS)]
#[ts(export, export_to = "protocol.ts")]
pub struct SessionId(String);

impl SessionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, TS)]
#[ts(export, export_to = "protocol.ts")]
pub struct PageId(String);

impl PageId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PageId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct SessionStartParams {
    pub profile_id: String,
}

impl Default for SessionStartParams {
    fn default() -> Self {
        Self {
            profile_id: "default".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct SessionStartResult {
    pub session_id: SessionId,
}

impl Default for SessionStartResult {
    fn default() -> Self {
        Self {
            session_id: SessionId::new(String::new()),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct SessionCloseParams {
    pub session_id: Option<SessionId>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct SessionCloseResult {
    pub closed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "protocol.ts")]
pub enum SessionEvent {
    Opened,
    Closed,
    Crashed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct SessionEventParams {
    pub session_id: SessionId,
    pub event: SessionEvent,
}

impl Default for SessionEventParams {
    fn default() -> Self {
        Self {
            session_id: SessionId::new(String::new()),
            event: SessionEvent::Closed,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct PageEnsureParams {
    pub session_id: SessionId,
    pub platform_id: String,
    pub url: String,
}

impl Default for PageEnsureParams {
    fn default() -> Self {
        Self {
            session_id: SessionId::new(String::new()),
            platform_id: "default".to_string(),
            url: "about:blank".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct PageEnsureResult {
    pub page_id: PageId,
    pub platform_id: String,
    pub url: String,
    pub created: bool,
}

impl Default for PageEnsureResult {
    fn default() -> Self {
        Self {
            page_id: PageId::new(String::new()),
            platform_id: "default".to_string(),
            url: "about:blank".to_string(),
            created: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct PageNavigateParams {
    pub session_id: SessionId,
    pub platform_id: String,
    pub url: String,
}

impl Default for PageNavigateParams {
    fn default() -> Self {
        Self {
            session_id: SessionId::new(String::new()),
            platform_id: "default".to_string(),
            url: "about:blank".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct PageNavigateResult {
    pub page_id: PageId,
    pub url: String,
}

impl Default for PageNavigateResult {
    fn default() -> Self {
        Self {
            page_id: PageId::new(String::new()),
            url: "about:blank".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct PageActivateParams {
    pub session_id: SessionId,
    pub platform_id: String,
}

impl Default for PageActivateParams {
    fn default() -> Self {
        Self {
            session_id: SessionId::new(String::new()),
            platform_id: "default".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct PageActivateResult {
    pub activated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct PageCloseParams {
    pub session_id: SessionId,
    pub platform_id: String,
}

impl Default for PageCloseParams {
    fn default() -> Self {
        Self {
            session_id: SessionId::new(String::new()),
            platform_id: "default".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct PageCloseResult {
    pub closed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct PageListParams {
    pub session_id: SessionId,
}

impl Default for PageListParams {
    fn default() -> Self {
        Self {
            session_id: SessionId::new(String::new()),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct PageListResult {
    pub pages: Vec<PageSnapshot>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct PageSnapshot {
    pub page_id: PageId,
    pub platform_id: String,
    pub url: String,
    pub state: PageState,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "protocol.ts")]
pub enum PageState {
    Opening,
    #[default]
    Open,
    Navigating,
    Crashed,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "protocol.ts")]
pub enum PageEvent {
    Opened,
    Navigated,
    #[default]
    Closed,
    Crashed,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct PageEventParams {
    pub session_id: SessionId,
    pub page_id: Option<PageId>,
    pub platform_id: Option<String>,
    pub url: Option<String>,
    pub event: PageEvent,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct RuntimeShutdownParams {}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct RuntimeShutdownResult {
    pub ok: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct ProtocolErrorParams {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "protocol.ts")]
pub enum RpcMethod {
    #[serde(rename = "runtime.ready")]
    RuntimeReady,
    #[serde(rename = "runtime.health")]
    RuntimeHealth,
    #[serde(rename = "runtime.shutdown")]
    RuntimeShutdown,
    #[serde(rename = "browser.session.start")]
    SessionStart,
    #[serde(rename = "browser.session.close")]
    SessionClose,
    #[serde(rename = "browser.session.event")]
    SessionEvent,
    #[serde(rename = "browser.page.ensure")]
    PageEnsure,
    #[serde(rename = "browser.page.navigate")]
    PageNavigate,
    #[serde(rename = "browser.page.activate")]
    PageActivate,
    #[serde(rename = "browser.page.close")]
    PageClose,
    #[serde(rename = "browser.page.list")]
    PageList,
    #[serde(rename = "browser.page.event")]
    PageEvent,
    #[serde(rename = "$/cancelRequest")]
    CancelRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct RpcErrorPayload {
    pub code: i32,
    pub message: String,
    pub data: Option<String>,
}

impl Default for RpcErrorPayload {
    fn default() -> Self {
        Self {
            code: -32_000,
            message: "runtime error".to_string(),
            data: None,
        }
    }
}
