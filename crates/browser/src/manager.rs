use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use jobless_config::{AppConfig, AppPaths, BrowserEngine};
use serde::Serialize;
use serde::de::DeserializeOwned;
use thiserror::Error;

use crate::protocol::{
    BrowserCloseParams, BrowserCloseResult, BrowserEventParams, BrowserOpenParams,
    BrowserOpenResult, ProtocolErrorParams, RuntimeHealthParams, RuntimeHealthResult,
    RuntimeShutdownParams, RuntimeShutdownResult, method,
};
use crate::transport::{IncomingNotification, RpcConnection, TransportError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePaths {
    pub runtime_dir: PathBuf,
    pub runtime_bin: PathBuf,
    pub worker_entry: PathBuf,
    pub chromium_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct BrowserManagerConfig {
    pub runtime: RuntimePaths,
    pub data_dir: PathBuf,
    pub engine: BrowserEngine,
    pub headless: bool,
    pub request_timeout: Duration,
    pub shutdown_timeout: Duration,
    pub max_frame_bytes: usize,
}

impl BrowserManagerConfig {
    pub fn from_app(config: &AppConfig, paths: &AppPaths) -> Result<Self, BrowserError> {
        let runtime = resolve_runtime_paths(config, paths)?;
        Ok(Self {
            runtime,
            data_dir: paths.data_dir.clone(),
            engine: config.browser.engine,
            headless: config.browser.headless,
            request_timeout: Duration::from_millis(config.rpc.request_timeout_ms),
            shutdown_timeout: Duration::from_millis(config.rpc.shutdown_timeout_ms),
            max_frame_bytes: config.rpc.max_frame_bytes,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserSession {
    pub session_id: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeNotification {
    BrowserEvent(BrowserEventParams),
    ProtocolError(ProtocolErrorParams),
    Unknown {
        method: String,
        params: serde_json::Value,
    },
}

pub struct BrowserManager {
    connection: RpcConnection,
    child: Child,
    config: BrowserManagerConfig,
    stopped: bool,
    buffered_notifications: VecDeque<RuntimeNotification>,
}

impl BrowserManager {
    pub fn start(config: BrowserManagerConfig) -> Result<Self, BrowserError> {
        let mut command = Command::new(&config.runtime.runtime_bin);
        command
            .arg(&config.runtime.worker_entry)
            .arg("--data-dir")
            .arg(&config.data_dir)
            .arg("--engine")
            .arg(browser_engine_name(config.engine))
            .arg("--headless")
            .arg(if config.headless { "true" } else { "false" })
            .env("PLAYWRIGHT_BROWSERS_PATH", &config.runtime.chromium_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|source| BrowserError::Spawn {
            path: config.runtime.runtime_bin.clone(),
            source,
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or(BrowserError::MissingPipe("stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or(BrowserError::MissingPipe("stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or(BrowserError::MissingPipe("stderr"))?;

        std::thread::Builder::new()
            .name("jobless-browser-stderr".to_string())
            .spawn(move || {
                use std::io::{BufRead, BufReader};
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    tracing::info!(target: "browser_worker", "{line}");
                }
            })
            .map_err(|source| BrowserError::ThreadSpawn(source.to_string()))?;

        let connection = RpcConnection::new(stdout, stdin, config.max_frame_bytes);
        let mut manager = Self {
            connection,
            child,
            config,
            stopped: false,
            buffered_notifications: VecDeque::new(),
        };
        manager.wait_for_ready()?;
        Ok(manager)
    }

    pub fn health(&self) -> Result<RuntimeHealthResult, BrowserError> {
        self.call(method::RUNTIME_HEALTH, &RuntimeHealthParams::default())
    }

    pub fn open(
        &self,
        url: impl Into<String>,
        profile_id: impl Into<String>,
    ) -> Result<BrowserSession, BrowserError> {
        let result: BrowserOpenResult = self.call(
            method::BROWSER_OPEN,
            &BrowserOpenParams {
                url: url.into(),
                profile_id: profile_id.into(),
            },
        )?;
        Ok(BrowserSession {
            session_id: result.session_id,
            url: result.url,
        })
    }

    pub fn close(&self, session_id: Option<String>) -> Result<u32, BrowserError> {
        let result: BrowserCloseResult =
            self.call(method::BROWSER_CLOSE, &BrowserCloseParams { session_id })?;
        Ok(result.closed)
    }

    pub fn drain_notifications(&mut self) -> Vec<RuntimeNotification> {
        let mut notifications = self.buffered_notifications.drain(..).collect::<Vec<_>>();
        while let Some(notification) = self.connection.try_recv_notification() {
            notifications.push(RuntimeNotification::from_incoming(notification));
        }
        notifications
    }

    pub fn shutdown(&mut self) -> Result<(), BrowserError> {
        if self.stopped {
            return Ok(());
        }

        let _ = self.call_with_timeout::<RuntimeShutdownParams, RuntimeShutdownResult>(
            method::RUNTIME_SHUTDOWN,
            &RuntimeShutdownParams::default(),
            self.config.shutdown_timeout,
        );

        let deadline = Instant::now() + self.config.shutdown_timeout;
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => {
                    self.stopped = true;
                    return Ok(());
                }
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(50));
                }
                Ok(None) => {
                    self.child
                        .kill()
                        .map_err(|source| BrowserError::Kill(source.to_string()))?;
                    let _ = self.child.wait();
                    self.stopped = true;
                    return Ok(());
                }
                Err(source) => {
                    self.stopped = true;
                    return Err(BrowserError::Wait(source.to_string()));
                }
            }
        }
    }

    pub fn is_running(&mut self) -> bool {
        !self.stopped && matches!(self.child.try_wait(), Ok(None))
    }

    fn call<P, R>(&self, method_name: &str, params: &P) -> Result<R, BrowserError>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        self.call_with_timeout(method_name, params, self.config.request_timeout)
    }

    fn call_with_timeout<P, R>(
        &self,
        method_name: &str,
        params: &P,
        timeout: Duration,
    ) -> Result<R, BrowserError>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        let value = self.connection.call(method_name, params, timeout)?;
        serde_json::from_value(value).map_err(BrowserError::Decode)
    }

    fn wait_for_ready(&mut self) -> Result<(), BrowserError> {
        let deadline = Instant::now() + self.config.request_timeout;
        loop {
            let now = Instant::now();
            if now >= deadline {
                return Err(BrowserError::ReadyTimeout(self.config.request_timeout));
            }

            match self.connection.recv_notification(deadline - now) {
                Ok(notification) if notification.method == method::RUNTIME_READY => return Ok(()),
                Ok(notification) => self
                    .buffered_notifications
                    .push_back(RuntimeNotification::from_incoming(notification)),
                Err(error) => return Err(BrowserError::Transport(error)),
            }
        }
    }
}

impl RuntimeNotification {
    fn from_incoming(notification: IncomingNotification) -> Self {
        match notification.method.as_str() {
            method::BROWSER_EVENT => match serde_json::from_value(notification.params) {
                Ok(params) => Self::BrowserEvent(params),
                Err(error) => Self::ProtocolError(ProtocolErrorParams {
                    message: format!("invalid browser.event payload: {error}"),
                }),
            },
            method::RUNTIME_PROTOCOL_ERROR => match serde_json::from_value(notification.params) {
                Ok(params) => Self::ProtocolError(params),
                Err(error) => Self::ProtocolError(ProtocolErrorParams {
                    message: format!("invalid protocol error payload: {error}"),
                }),
            },
            _ => Self::Unknown {
                method: notification.method,
                params: notification.params,
            },
        }
    }
}

impl Drop for BrowserManager {
    fn drop(&mut self) {
        if self.stopped {
            return;
        }

        let _ = self
            .connection
            .notify(method::RUNTIME_SHUTDOWN, &RuntimeShutdownParams::default());
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.stopped = true;
    }
}

#[derive(Debug, Error)]
pub enum BrowserError {
    #[error(transparent)]
    Config(#[from] jobless_config::ConfigError),
    #[error("Bun runtime was not found at `{path}`")]
    MissingRuntime { path: PathBuf },
    #[error("browser worker entry was not found at `{path}`")]
    MissingWorker { path: PathBuf },
    #[error("failed to spawn browser worker with `{path}`: {source}")]
    Spawn {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("worker process did not expose {0}")]
    MissingPipe(&'static str),
    #[error("failed to start background thread: {0}")]
    ThreadSpawn(String),
    #[error("timed out waiting for runtime.ready after {0:?}")]
    ReadyTimeout(Duration),
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error("failed to decode worker response: {0}")]
    Decode(#[source] serde_json::Error),
    #[error("failed to stop worker: {0}")]
    Kill(String),
    #[error("failed while waiting for worker: {0}")]
    Wait(String),
}

pub fn resolve_runtime_paths(
    config: &AppConfig,
    paths: &AppPaths,
) -> Result<RuntimePaths, BrowserError> {
    let runtime_dir = config
        .runtime
        .dir
        .clone()
        .unwrap_or_else(|| paths.runtime_dir.clone());

    let runtime_bin = config
        .runtime
        .bin
        .clone()
        .or_else(|| first_existing(&runtime_candidates(&runtime_dir)))
        .unwrap_or_else(|| PathBuf::from(if cfg!(windows) { "bun.exe" } else { "bun" }));

    let worker_entry = config
        .runtime
        .worker_entry
        .clone()
        .or_else(|| first_existing(&worker_candidates(&runtime_dir)))
        .ok_or_else(|| BrowserError::MissingWorker {
            path: runtime_dir.join("browser-worker/src/main.ts"),
        })?;

    if runtime_bin.is_absolute() && !runtime_bin.is_file() {
        return Err(BrowserError::MissingRuntime { path: runtime_bin });
    }

    Ok(RuntimePaths {
        runtime_dir: runtime_dir.clone(),
        runtime_bin,
        worker_entry,
        chromium_dir: runtime_dir.join("chromium"),
    })
}

fn worker_candidates(runtime_dir: &Path) -> Vec<PathBuf> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    vec![
        runtime_dir.join("browser-worker/src/main.ts"),
        cwd.join("runtime/browser-worker/src/main.ts"),
        cwd.join("browser-worker/src/main.ts"),
    ]
}

fn runtime_candidates(runtime_dir: &Path) -> Vec<PathBuf> {
    vec![runtime_dir.join("bun/bun.exe"), runtime_dir.join("bun/bun")]
}

fn first_existing(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|path| path.is_file()).cloned()
}

fn browser_engine_name(engine: BrowserEngine) -> &'static str {
    match engine {
        BrowserEngine::Patchright => "patchright",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::IncomingNotification;

    #[test]
    fn resolves_development_runtime_from_repo_layout() {
        let root = tempfile::tempdir().unwrap();
        let runtime_dir = root.path().join("runtime");
        let worker = runtime_dir.join("browser-worker/src/main.ts");
        std::fs::create_dir_all(worker.parent().unwrap()).unwrap();
        std::fs::write(&worker, "// worker").unwrap();
        let bun = runtime_dir.join("bun/bun");
        std::fs::create_dir_all(bun.parent().unwrap()).unwrap();
        std::fs::write(&bun, "// bun").unwrap();

        let paths = AppPaths::with_overrides(
            Some(root.path().join("config/config.toml")),
            Some(root.path().join("data")),
            Some(runtime_dir.clone()),
        )
        .unwrap();
        let resolved = resolve_runtime_paths(&AppConfig::default(), &paths).unwrap();

        assert_eq!(resolved.runtime_bin, bun);
        assert_eq!(resolved.worker_entry, worker);
    }

    #[test]
    fn parses_browser_event_notifications() {
        let notification = RuntimeNotification::from_incoming(IncomingNotification {
            method: method::BROWSER_EVENT.to_string(),
            params: serde_json::json!({
                "session_id": "session-1",
                "event": "crashed"
            }),
        });

        assert_eq!(
            notification,
            RuntimeNotification::BrowserEvent(BrowserEventParams {
                session_id: "session-1".to_string(),
                event: crate::protocol::BrowserEvent::Crashed,
            })
        );
    }
}
