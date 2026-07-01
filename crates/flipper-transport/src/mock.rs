//! In-memory mock transport for tests and offline TUI runs.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use bytes::Bytes;

use crate::base::{CommandResult, Transport, TransportError};

type Handler = Arc<dyn Fn(&[&str]) -> CommandResult + Send + Sync>;

/// One entry in [`MockTransport::call_log`]: the command name and the
/// owned arg vector as captured at call time.
pub type CallRecord = (String, Vec<String>);

/// Mock transport that dispatches to per-command handlers registered via
/// [`MockTransport::on`].
#[derive(Default, Clone)]
pub struct MockTransport {
    handlers: Arc<Mutex<HashMap<String, Handler>>>,
    /// Commands registered with [`MockTransport::one_shot`]. The
    /// handler is removed after its first invocation.
    one_shot: Arc<Mutex<HashSet<String>>>,
    connected: Arc<Mutex<bool>>,
    log: Arc<Mutex<Vec<CallRecord>>>,
}

impl MockTransport {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a persistent handler for a command name. Unlike
    /// [`MockTransport::one_shot`], handlers registered with `on`
    /// fire every time the command is sent — which is what the
    /// offline TUI needs (the event loop re-issues `storage list`
    /// and `storage info` on every screen refresh).
    ///
    /// ```ignore
    /// let tx = MockTransport::new();
    /// tx.on("device_info", |_args| CommandResult::ok(b"hello".to_vec()));
    /// ```
    pub fn on<F>(&self, command: &str, handler: F)
    where
        F: Fn(&[&str]) -> CommandResult + Send + Sync + 'static,
    {
        self.handlers
            .lock()
            .expect("mock handlers poisoned")
            .insert(command.to_string(), Arc::new(handler));
    }

    /// Register a one-shot handler. The handler fires on the next
    /// matching `send` and is then removed. Useful in unit tests
    /// that want to assert "the call happened exactly once".
    pub fn one_shot<F>(&self, command: &str, handler: F)
    where
        F: Fn(&[&str]) -> CommandResult + Send + Sync + 'static,
    {
        self.handlers
            .lock()
            .expect("mock handlers poisoned")
            .insert(command.to_string(), Arc::new(handler));
        self.one_shot
            .lock()
            .expect("mock one_shot poisoned")
            .insert(command.to_string());
    }

    /// Returns every command this transport has received, in order.
    pub fn call_log(&self) -> Vec<CallRecord> {
        self.log.lock().expect("mock log poisoned").clone()
    }
}

impl std::fmt::Debug for MockTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockTransport")
            .field("connected", &*self.connected.lock().unwrap())
            .field("handlers", &self.handlers.lock().unwrap().len())
            .field("log_len", &self.log.lock().unwrap().len())
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl Transport for MockTransport {
    async fn connect(&self) -> Result<(), TransportError> {
        *self.connected.lock().expect("connected poisoned") = true;
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), TransportError> {
        *self.connected.lock().expect("connected poisoned") = false;
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        *self.connected.lock().expect("connected poisoned")
    }

    async fn send(&self, command: &str, args: &[&str]) -> Result<CommandResult, TransportError> {
        if !self.is_connected().await {
            return Err(TransportError::NotConnected);
        }
        let owned_args: Vec<String> = args.iter().map(|a| (*a).to_string()).collect();
        self.log
            .lock()
            .expect("mock log poisoned")
            .push((command.to_string(), owned_args));
        // `on` is persistent: clone the handler and leave it in place
        // so subsequent calls to the same command still fire.
        // `one_shot` removes after invocation. This split is what the
        // offline TUI depends on — the event loop re-issues
        // `storage list /ext` and `storage info /ext` on every screen
        // refresh, and the old `remove()` semantics silently broke
        // every refresh after the first.
        let (handler, was_one_shot) = {
            let mut map = self.handlers.lock().expect("mock handlers poisoned");
            let is_one_shot = self
                .one_shot
                .lock()
                .expect("mock one_shot poisoned")
                .contains(command);
            let handler = if is_one_shot {
                map.remove(command)
            } else {
                map.get(command).map(Arc::clone)
            };
            (handler, is_one_shot)
        };
        if was_one_shot {
            self.one_shot
                .lock()
                .expect("mock one_shot poisoned")
                .remove(command);
        }
        match handler {
            Some(h) => Ok(h(args)),
            None => Err(TransportError::MockUnhandled(command.to_string())),
        }
    }
}

/// Convenience: a mock transport with a single canned response for any command.
impl MockTransport {
    pub fn always_replying(reply: &'static [u8]) -> Self {
        let tx = Self::new();
        tx.on("*", move |_args| CommandResult {
            response: Bytes::from_static(reply),
            status: None,
        });
        tx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn not_connected_rejects_send() {
        let tx = MockTransport::new();
        let err = tx.send("device_info", &[]).await.unwrap_err();
        assert!(matches!(err, TransportError::NotConnected));
    }

    #[tokio::test]
    async fn handler_dispatched_and_logged() {
        let tx = MockTransport::new();
        tx.on("device_info", |_args| CommandResult::ok(b"hello".to_vec()));
        tx.connect().await.unwrap();
        let r = tx.send("device_info", &["/ext"]).await.unwrap();
        assert_eq!(r.response, Bytes::from_static(b"hello"));
        assert_eq!(
            tx.call_log(),
            vec![("device_info".into(), vec!["/ext".into()])]
        );
    }

    #[tokio::test]
    async fn unhandled_command_errors() {
        let tx = MockTransport::new();
        tx.connect().await.unwrap();
        let err = tx.send("nope", &[]).await.unwrap_err();
        assert!(matches!(err, TransportError::MockUnhandled(_)));
    }

    #[tokio::test]
    async fn ping_succeeds_when_device_replies_with_bytes() {
        let tx = MockTransport::new();
        tx.on("", |_args| CommandResult::ok(b"any-bytes".to_vec()));
        tx.connect().await.unwrap();
        // Default `Transport::ping` impl sends an empty line and checks
        // the reply is non-empty.
        tx.ping().await.expect("ping should succeed");
    }

    #[tokio::test]
    async fn ping_errors_when_device_returns_empty_reply() {
        let tx = MockTransport::new();
        tx.on("", |_args| CommandResult::ok(b"".to_vec()));
        tx.connect().await.unwrap();
        let err = tx.ping().await.unwrap_err();
        assert!(matches!(err, TransportError::Io(_)));
    }
}
