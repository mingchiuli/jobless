mod manager;
mod protocol;
mod transport;

pub use manager::{
    BrowserError, BrowserManager, BrowserManagerConfig, BrowserSession, RuntimeNotification,
    RuntimePaths, resolve_runtime_paths,
};
pub use protocol::{
    BrowserCloseParams, BrowserCloseResult, BrowserEngineInfo, BrowserEvent, BrowserEventParams,
    BrowserInfo, BrowserOpenParams, BrowserOpenResult, ProtocolErrorParams, RpcErrorPayload,
    RpcMethod, RuntimeHealthParams, RuntimeHealthResult, RuntimeInfo, RuntimeReadyParams,
    RuntimeShutdownParams, RuntimeShutdownResult, method,
};
pub use transport::{IncomingNotification, RpcConnection, TransportError};
