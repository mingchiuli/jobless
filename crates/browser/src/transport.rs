use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, unbounded};
use serde::Serialize;
use serde_json::{Value, json};
use thiserror::Error;

use crate::protocol::{RpcErrorPayload, method};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomingNotification {
    pub method: String,
    pub params: Value,
}

#[derive(Debug, Clone, Error)]
pub enum TransportError {
    #[error("I/O error: {0}")]
    Io(String),
    #[error("worker exited")]
    WorkerExited,
    #[error("worker connection closed")]
    Disconnected,
    #[error("invalid protocol frame: {0}")]
    InvalidFrame(String),
    #[error("protocol frame exceeds the configured limit of {limit} bytes")]
    FrameTooLarge { limit: usize },
    #[error("invalid JSON: {0}")]
    Json(String),
    #[error("request timed out after {0:?}")]
    Timeout(Duration),
    #[error("request lock poisoned")]
    LockPoisoned,
    #[error("worker error {code}: {message}")]
    Rpc {
        code: i32,
        message: String,
        data: Option<String>,
    },
}

impl From<RpcErrorPayload> for TransportError {
    fn from(value: RpcErrorPayload) -> Self {
        Self::Rpc {
            code: value.code,
            message: value.message,
            data: value.data,
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct WireMessage {
    #[serde(default)]
    jsonrpc: Option<String>,
    #[serde(default)]
    id: Option<u64>,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    params: Option<Value>,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<RpcErrorPayload>,
}

type PendingMap = Arc<Mutex<HashMap<u64, Sender<Result<Value, TransportError>>>>>;

pub struct RpcConnection {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    pending: PendingMap,
    notifications: Receiver<IncomingNotification>,
    next_id: AtomicU64,
    max_frame_bytes: usize,
}

impl RpcConnection {
    pub fn new<R, W>(reader: R, writer: W, max_frame_bytes: usize) -> Self
    where
        R: Read + Send + 'static,
        W: Write + Send + 'static,
    {
        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        let (notification_tx, notifications) = unbounded();
        let reader_pending = Arc::clone(&pending);

        std::thread::Builder::new()
            .name("jobless-browser-rpc-reader".to_string())
            .spawn(move || {
                read_loop(
                    BufReader::new(reader),
                    reader_pending,
                    notification_tx,
                    max_frame_bytes,
                );
            })
            .expect("failed to spawn RPC reader thread");

        Self {
            writer: Arc::new(Mutex::new(Box::new(writer))),
            pending,
            notifications,
            next_id: AtomicU64::new(1),
            max_frame_bytes,
        }
    }

    pub fn max_frame_bytes(&self) -> usize {
        self.max_frame_bytes
    }

    pub fn call<P>(
        &self,
        method_name: impl Into<String>,
        params: &P,
        timeout: Duration,
    ) -> Result<Value, TransportError>
    where
        P: Serialize,
    {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = crossbeam_channel::bounded(1);
        self.pending
            .lock()
            .map_err(|_| TransportError::LockPoisoned)?
            .insert(id, sender);

        let method_name = method_name.into();
        if let Err(error) = self.send(&WireMessage {
            jsonrpc: Some("2.0".to_string()),
            id: Some(id),
            method: Some(method_name.clone()),
            params: Some(serde_json::to_value(params).map_err(json_error)?),
            result: None,
            error: None,
        }) {
            self.remove_pending(id);
            return Err(error);
        }

        match receiver.recv_timeout(timeout) {
            Ok(result) => result,
            Err(RecvTimeoutError::Timeout) => {
                self.remove_pending(id);
                self.cancel(id);
                Err(TransportError::Timeout(timeout))
            }
            Err(RecvTimeoutError::Disconnected) => {
                self.remove_pending(id);
                Err(TransportError::Disconnected)
            }
        }
    }

    pub fn notify<P>(
        &self,
        method_name: impl Into<String>,
        params: &P,
    ) -> Result<(), TransportError>
    where
        P: Serialize,
    {
        self.send(&WireMessage {
            jsonrpc: Some("2.0".to_string()),
            id: None,
            method: Some(method_name.into()),
            params: Some(serde_json::to_value(params).map_err(json_error)?),
            result: None,
            error: None,
        })
    }

    pub fn recv_notification(
        &self,
        timeout: Duration,
    ) -> Result<IncomingNotification, TransportError> {
        match self.notifications.recv_timeout(timeout) {
            Ok(notification) => Ok(notification),
            Err(RecvTimeoutError::Timeout) => Err(TransportError::Timeout(timeout)),
            Err(RecvTimeoutError::Disconnected) => Err(TransportError::Disconnected),
        }
    }

    pub fn try_recv_notification(&self) -> Option<IncomingNotification> {
        self.notifications.try_recv().ok()
    }

    fn cancel(&self, id: u64) {
        let _ = self.notify(method::CANCEL_REQUEST, &json!({ "id": id }));
    }

    fn remove_pending(&self, id: u64) {
        if let Ok(mut pending) = self.pending.lock() {
            pending.remove(&id);
        }
    }

    fn send(&self, message: &WireMessage) -> Result<(), TransportError> {
        let payload = serde_json::to_vec(message).map_err(json_error)?;
        if payload.len() > self.max_frame_bytes {
            return Err(TransportError::FrameTooLarge {
                limit: self.max_frame_bytes,
            });
        }

        let mut writer = self
            .writer
            .lock()
            .map_err(|_| TransportError::LockPoisoned)?;
        write!(writer, "Content-Length: {}\r\n\r\n", payload.len()).map_err(io_error)?;
        writer.write_all(&payload).map_err(io_error)?;
        writer.flush().map_err(io_error)
    }
}

fn read_loop<R: BufRead>(
    mut reader: R,
    pending: PendingMap,
    notifications: Sender<IncomingNotification>,
    max_frame_bytes: usize,
) {
    loop {
        match read_frame(&mut reader, max_frame_bytes) {
            Ok(Some(frame)) => {
                let message = match serde_json::from_slice::<WireMessage>(&frame) {
                    Ok(message) => message,
                    Err(error) => {
                        disconnect_all(
                            &pending,
                            TransportError::Json(error.to_string()),
                            "invalid JSON from worker",
                        );
                        let _ = notifications.send(IncomingNotification {
                            method: method::RUNTIME_PROTOCOL_ERROR.to_string(),
                            params: json!({ "message": error.to_string() }),
                        });
                        continue;
                    }
                };

                if let Some(id) = message.id {
                    let result = if let Some(error) = message.error {
                        Err(error.into())
                    } else {
                        Ok(message.result.unwrap_or(Value::Null))
                    };

                    if let Ok(mut pending) = pending.lock()
                        && let Some(sender) = pending.remove(&id)
                    {
                        let _ = sender.send(result);
                    }
                    continue;
                }

                if let Some(method_name) = message.method {
                    let _ = notifications.send(IncomingNotification {
                        method: method_name,
                        params: message.params.unwrap_or(Value::Null),
                    });
                }
            }
            Ok(None) => {
                disconnect_all(
                    &pending,
                    TransportError::WorkerExited,
                    "worker stdout closed",
                );
                break;
            }
            Err(error) => {
                disconnect_all(&pending, error, "worker protocol error");
                break;
            }
        }
    }
}

fn read_frame<R: BufRead>(
    reader: &mut R,
    max_frame_bytes: usize,
) -> Result<Option<Vec<u8>>, TransportError> {
    let mut content_length = None;
    let mut line = Vec::new();

    loop {
        line.clear();
        let read = reader.read_until(b'\n', &mut line).map_err(io_error)?;
        if read == 0 {
            return Ok(None);
        }

        let header = std::str::from_utf8(&line)
            .map_err(|error| TransportError::InvalidFrame(error.to_string()))?
            .trim_end_matches(['\r', '\n']);
        if header.is_empty() {
            break;
        }

        if let Some((name, value)) = header.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|error| TransportError::InvalidFrame(error.to_string()))?,
            );
        }
    }

    let length = content_length
        .ok_or_else(|| TransportError::InvalidFrame("missing Content-Length".to_string()))?;
    if length > max_frame_bytes {
        return Err(TransportError::FrameTooLarge {
            limit: max_frame_bytes,
        });
    }

    let mut payload = vec![0; length];
    reader.read_exact(&mut payload).map_err(|error| {
        if error.kind() == std::io::ErrorKind::UnexpectedEof {
            TransportError::WorkerExited
        } else {
            io_error(error)
        }
    })?;
    Ok(Some(payload))
}

fn disconnect_all(pending: &PendingMap, error: TransportError, label: &str) {
    tracing::debug!(error = %error, "{label}");
    if let Ok(mut pending) = pending.lock() {
        for (_, sender) in pending.drain() {
            let _ = sender.send(Err(error.clone()));
        }
    }
}

fn io_error(error: std::io::Error) -> TransportError {
    TransportError::Io(error.to_string())
}

fn json_error(error: serde_json::Error) -> TransportError {
    TransportError::Json(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn reads_partial_and_multiple_frames() {
        let input = concat!(
            "Content-Length: 8\r\n\r\n",
            "{\"id\":1}",
            "Content-Length: 14\r\n\r\n",
            "{\"method\":\"x\"}"
        );
        let mut reader = BufReader::new(Cursor::new(input.as_bytes()));

        assert_eq!(
            read_frame(&mut reader, 1024).unwrap().unwrap(),
            b"{\"id\":1}"
        );
        assert_eq!(
            read_frame(&mut reader, 1024).unwrap().unwrap(),
            b"{\"method\":\"x\"}"
        );
        assert!(read_frame(&mut reader, 1024).unwrap().is_none());
    }

    #[test]
    fn rejects_oversized_frames() {
        let input = b"Content-Length: 100\r\n\r\n";
        let mut reader = BufReader::new(Cursor::new(input.as_slice()));

        assert!(matches!(
            read_frame(&mut reader, 10),
            Err(TransportError::FrameTooLarge { .. })
        ));
    }
}
