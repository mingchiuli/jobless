use crate::protocol::{PageEventParams, ProtocolErrorParams, SessionEventParams, method};
use crate::transport::IncomingNotification;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeNotification {
    SessionEvent(SessionEventParams),
    PageEvent(PageEventParams),
    ProtocolError(ProtocolErrorParams),
    Unknown {
        method: String,
        params: serde_json::Value,
    },
}

impl RuntimeNotification {
    pub(super) fn from_incoming(notification: IncomingNotification) -> Self {
        match notification.method.as_str() {
            method::SESSION_EVENT => match serde_json::from_value(notification.params) {
                Ok(params) => Self::SessionEvent(params),
                Err(error) => Self::protocol_error("invalid browser.session.event payload", error),
            },
            method::PAGE_EVENT => match serde_json::from_value(notification.params) {
                Ok(params) => Self::PageEvent(params),
                Err(error) => Self::protocol_error("invalid browser.page.event payload", error),
            },
            method::RUNTIME_PROTOCOL_ERROR => match serde_json::from_value(notification.params) {
                Ok(params) => Self::ProtocolError(params),
                Err(error) => Self::protocol_error("invalid protocol error payload", error),
            },
            _ => Self::Unknown {
                method: notification.method,
                params: notification.params,
            },
        }
    }

    fn protocol_error(context: &str, error: serde_json::Error) -> Self {
        Self::ProtocolError(ProtocolErrorParams {
            message: format!("{context}: {error}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_page_events() {
        let notification = RuntimeNotification::from_incoming(IncomingNotification {
            method: method::PAGE_EVENT.to_string(),
            params: serde_json::json!({
                "session_id": "session-1",
                "page_id": "page-1",
                "platform_id": "boss",
                "url": "https://example.com",
                "event": "navigated"
            }),
        });

        let RuntimeNotification::PageEvent(event) = notification else {
            panic!("expected page event");
        };
        assert_eq!(event.platform_id.as_deref(), Some("boss"));
        assert_eq!(event.url.as_deref(), Some("https://example.com"));
    }
}
