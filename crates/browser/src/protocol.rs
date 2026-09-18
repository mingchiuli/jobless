use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub mod method {
    pub const RUNTIME_READY: &str = "runtime.ready";
    pub const RUNTIME_HEALTH: &str = "runtime.health";
    pub const RUNTIME_SHUTDOWN: &str = "runtime.shutdown";
    pub const BROWSER_OPEN: &str = "browser.open";
    pub const BROWSER_CLOSE: &str = "browser.close";
    pub const BROWSER_EVENT: &str = "browser.event";
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct BrowserOpenParams {
    pub url: String,
    pub profile_id: String,
}

impl Default for BrowserOpenParams {
    fn default() -> Self {
        Self {
            url: "about:blank".to_string(),
            profile_id: "default".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct BrowserOpenResult {
    pub session_id: String,
    pub url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct BrowserCloseParams {
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct BrowserCloseResult {
    pub closed: u32,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "protocol.ts")]
pub enum BrowserEvent {
    Opened,
    #[default]
    Closed,
    Crashed,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(default)]
#[ts(export, export_to = "protocol.ts")]
pub struct BrowserEventParams {
    pub session_id: String,
    pub event: BrowserEvent,
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
    #[serde(rename = "browser.open")]
    BrowserOpen,
    #[serde(rename = "browser.close")]
    BrowserClose,
    #[serde(rename = "browser.event")]
    BrowserEvent,
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
