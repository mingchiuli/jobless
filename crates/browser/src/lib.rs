mod manager;
mod protocol;
mod transport;

pub use manager::{
    BrowserError, BrowserManager, BrowserManagerConfig, BrowserPage, BrowserSession,
    RuntimeNotification, RuntimePaths, resolve_runtime_paths,
};
pub use protocol::{
    BrowserEngineInfo, BrowserInfo, PageActivateParams, PageActivateResult, PageCloseParams,
    PageCloseResult, PageEnsureParams, PageEnsureResult, PageEvent, PageEventParams, PageId,
    PageListParams, PageListResult, PageNavigateParams, PageNavigateResult, PageSnapshot,
    PageState, ProtocolErrorParams, RpcErrorPayload, RpcMethod, RuntimeHealthParams,
    RuntimeHealthResult, RuntimeInfo, RuntimeReadyParams, RuntimeShutdownParams,
    RuntimeShutdownResult, SessionCloseParams, SessionCloseResult, SessionEvent,
    SessionEventParams, SessionId, SessionStartParams, SessionStartResult, method,
};
pub use transport::{IncomingNotification, RpcConnection, TransportError};
