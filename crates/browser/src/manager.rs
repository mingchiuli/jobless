mod notification;
mod runtime_paths;
mod session;

use std::collections::VecDeque;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use jobless_config::{AppConfig, AppPaths, BrowserEngine};
use serde::Serialize;
use serde::de::DeserializeOwned;
use thiserror::Error;

use crate::protocol::{
    RuntimeHealthParams, RuntimeHealthResult, RuntimeShutdownParams, RuntimeShutdownResult, method,
};
use crate::transport::{RpcConnection, TransportError};

pub use notification::RuntimeNotification;
pub use runtime_paths::{RuntimePaths, resolve_runtime_paths};
pub use session::{BrowserPage, BrowserSession};

#[derive(Debug, Clone)]
pub struct BrowserManagerConfig {
    pub runtime: RuntimePaths,
    pub data_dir: std::path::PathBuf,
    pub engine: BrowserEngine,
    pub headless: bool,
    pub profile_id: String,
    pub request_timeout: Duration,
    pub shutdown_timeout: Duration,
    pub max_frame_bytes: usize,
}

impl BrowserManagerConfig {
    pub fn from_app(config: &AppConfig, paths: &AppPaths) -> Result<Self, BrowserError> {
        Ok(Self {
            runtime: resolve_runtime_paths(config, paths)?,
            data_dir: paths.data_dir.clone(),
            engine: config.browser.engine,
            headless: config.browser.headless,
            profile_id: config.browser.profile_id.clone(),
            request_timeout: Duration::from_millis(config.rpc.request_timeout_ms),
            shutdown_timeout: Duration::from_millis(config.rpc.shutdown_timeout_ms),
            max_frame_bytes: config.rpc.max_frame_bytes,
        })
    }
}

pub struct BrowserManager {
    pub(super) connection: RpcConnection,
    child: Child,
    pub(super) config: BrowserManagerConfig,
    stopped: bool,
    pub(super) buffered_notifications: VecDeque<RuntimeNotification>,
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

    pub(super) fn call<P, R>(&self, method_name: &str, params: &P) -> Result<R, BrowserError>
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
    #[error(transparent)]
    RuntimePaths(#[from] runtime_paths::RuntimePathError),
    #[error("failed to spawn browser worker with `{path}`: {source}")]
    Spawn {
        path: std::path::PathBuf,
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
    #[error("browser session was not found: {0}")]
    SessionNotFound(crate::protocol::SessionId),
    #[error("browser page was not found for platform: {0}")]
    PageNotFound(jobless_platform::PlatformId),
    #[error("failed to stop worker: {0}")]
    Kill(String),
    #[error("failed while waiting for worker: {0}")]
    Wait(String),
}

fn browser_engine_name(engine: BrowserEngine) -> &'static str {
    match engine {
        BrowserEngine::Patchright => "patchright",
    }
}
