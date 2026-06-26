//! Connection-session runtime for [`crate::Client`].
//!
//! The public API lives in `client`; this module owns mutable session state,
//! request multiplexing, notification dispatch, and reconnection. Keeping those
//! responsibilities here prevents the public facade from becoming a second
//! transport implementation.

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::{broadcast, mpsc, oneshot, watch};
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout};
use tokio_tungstenite::tungstenite::{Error as WebSocketError, Message};

use crate::client::{Client, ConnectionState, RequestId};
use crate::connection::{open_socket, ReconnectConfig, Socket};
use crate::event::{decode_event, normalize_notification, Event, RawNotification};
use crate::transport::{parse_inbound, serialize_request, Inbound, OutboundRequest};
use crate::{Capabilities, CompatibilityMode, Error, ReconnectPolicy, Result};

const NOTIFICATION_BUFFER_CAPACITY: usize = 256;

/// Immutable runtime options captured when a client connects.
pub(crate) struct RuntimeConfig {
    pub(crate) request_timeout: Duration,
    pub(crate) channel_capacity: usize,
    pub(crate) compatibility_mode: CompatibilityMode,
    pub(crate) reconnect_policy: ReconnectPolicy,
    pub(crate) reconnect_config: ReconnectConfig,
}

/// Shared mutable state behind every clone of [`Client`].
pub(crate) struct ClientInner {
    outbound_tx: RwLock<Option<mpsc::Sender<Outbound>>>,
    pending: Mutex<HashMap<RequestId, oneshot::Sender<Result<Value>>>>,
    next_id: AtomicU64,
    current_generation: AtomicU64,
    raw_notifications: broadcast::Sender<RawNotification>,
    events: broadcast::Sender<Event>,
    shutdown_tx: watch::Sender<bool>,
    shutdown_requested: AtomicBool,
    event_shutdown_tx: watch::Sender<bool>,
    session_stop: Mutex<Option<watch::Sender<bool>>>,
    state: RwLock<ConnectionState>,
    request_timeout: Duration,
    channel_capacity: usize,
    compatibility_mode: CompatibilityMode,
    capabilities: RwLock<Option<Capabilities>>,
    reconnect_policy: ReconnectPolicy,
    reconnect_config: ReconnectConfig,
    session_tasks: Mutex<Vec<JoinHandle<()>>>,
    reconnect_task: Mutex<Option<JoinHandle<()>>>,
}

impl fmt::Debug for ClientInner {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientInner")
            .field("state", &self.state.read().ok())
            .field("request_timeout", &self.request_timeout)
            .field("compatibility_mode", &self.compatibility_mode)
            .field("reconnect_policy", &self.reconnect_policy)
            .finish_non_exhaustive()
    }
}

impl ClientInner {
    pub(crate) fn new(config: RuntimeConfig) -> Self {
        let (raw_notifications, _) = broadcast::channel(NOTIFICATION_BUFFER_CAPACITY);
        let (events, _) = broadcast::channel(NOTIFICATION_BUFFER_CAPACITY);
        let (shutdown_tx, _) = watch::channel(false);
        let (event_shutdown_tx, _) = watch::channel(false);

        Self {
            outbound_tx: RwLock::new(None),
            pending: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            current_generation: AtomicU64::new(0),
            raw_notifications,
            events,
            shutdown_tx,
            shutdown_requested: AtomicBool::new(false),
            event_shutdown_tx,
            session_stop: Mutex::new(None),
            state: RwLock::new(ConnectionState::Connected),
            request_timeout: config.request_timeout,
            channel_capacity: config.channel_capacity,
            compatibility_mode: config.compatibility_mode,
            capabilities: RwLock::new(None),
            reconnect_policy: config.reconnect_policy,
            reconnect_config: config.reconnect_config,
            session_tasks: Mutex::new(Vec::new()),
            reconnect_task: Mutex::new(None),
        }
    }

    pub(crate) fn state(&self) -> ConnectionState {
        *self.state.read().expect("state lock poisoned")
    }

    pub(crate) fn compatibility_mode(&self) -> CompatibilityMode {
        self.compatibility_mode
    }

    pub(crate) fn reconnect_policy(&self) -> &ReconnectPolicy {
        &self.reconnect_policy
    }

    pub(crate) fn capabilities(&self) -> Option<Capabilities> {
        self.capabilities
            .read()
            .expect("capabilities lock poisoned")
            .clone()
    }

    pub(crate) fn replace_capabilities(&self, capabilities: Capabilities) {
        *self
            .capabilities
            .write()
            .expect("capabilities lock poisoned") = Some(capabilities);
    }

    pub(crate) fn subscribe_events(&self) -> broadcast::Receiver<Event> {
        self.events.subscribe()
    }

    pub(crate) fn subscribe_raw_notifications(&self) -> broadcast::Receiver<RawNotification> {
        self.raw_notifications.subscribe()
    }

    pub(crate) fn subscribe_event_shutdown(&self) -> watch::Receiver<bool> {
        self.event_shutdown_tx.subscribe()
    }

    pub(crate) async fn shutdown(&self) -> Result<()> {
        let preserve_failed_state = {
            let mut state = self.state.write().expect("state lock poisoned");
            let failed = *state == ConnectionState::Failed;
            if !matches!(*state, ConnectionState::Closed | ConnectionState::Failed) {
                *state = ConnectionState::Closing;
            }
            failed
        };

        self.shutdown_requested.store(true, Ordering::Release);
        let _ = self.shutdown_tx.send(true);
        let _ = self.event_shutdown_tx.send(true);
        self.stop_current_session();
        *self.outbound_tx.write().expect("outbound lock poisoned") = None;
        self.fail_all(Error::Closed);

        if let Some(task) = self
            .reconnect_task
            .lock()
            .expect("reconnect task lock poisoned")
            .take()
        {
            let _ = task.await;
        }

        let tasks = std::mem::take(
            &mut *self
                .session_tasks
                .lock()
                .expect("session tasks lock poisoned"),
        );
        for task in tasks {
            let _ = task.await;
        }

        if !preserve_failed_state {
            *self.state.write().expect("state lock poisoned") = ConnectionState::Closed;
        }
        Ok(())
    }

    pub(crate) fn ensure_method_allowed(&self, method: &str) -> Result<()> {
        if self.compatibility_mode != CompatibilityMode::Strict {
            return Ok(());
        }

        let capabilities = self
            .capabilities
            .read()
            .expect("capabilities lock poisoned");
        let capabilities = capabilities.as_ref().ok_or(Error::DiscoveryRequired)?;
        if capabilities.supports_method(method) {
            Ok(())
        } else {
            Err(Error::UnsupportedMethod {
                method: method.to_owned(),
            })
        }
    }

    pub(crate) async fn call_value(&self, method: &str, params: Option<Value>) -> Result<Value> {
        match self.state() {
            ConnectionState::Connected => {}
            ConnectionState::Reconnecting => return Err(Error::Reconnecting),
            ConnectionState::Closing | ConnectionState::Closed | ConnectionState::Failed => {
                return Err(Error::Closed);
            }
        }

        let id = self.next_request_id();
        let text = serialize_request(id, method, params)?;
        let (sender, receiver) = oneshot::channel();
        self.pending
            .lock()
            .expect("pending lock poisoned")
            .insert(id, sender);

        let outbound_tx = self
            .outbound_tx
            .read()
            .expect("outbound lock poisoned")
            .clone();
        let Some(outbound_tx) = outbound_tx else {
            self.remove_pending(id);
            return Err(self.unavailable_error());
        };

        if outbound_tx
            .send(Outbound::Request(OutboundRequest { text }))
            .await
            .is_err()
        {
            self.remove_pending(id);
            return Err(self.unavailable_error());
        }

        match timeout(self.request_timeout, receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(self.unavailable_error()),
            Err(_) => {
                self.remove_pending(id);
                Err(Error::Timeout {
                    id,
                    method: method.to_owned(),
                })
            }
        }
    }

    fn unavailable_error(&self) -> Error {
        if self.state() == ConnectionState::Reconnecting {
            Error::Reconnecting
        } else {
            Error::Closed
        }
    }

    fn next_request_id(&self) -> RequestId {
        loop {
            let id = self.next_id.fetch_add(1, Ordering::Relaxed);
            if id != 0 {
                return RequestId::new(id);
            }
        }
    }

    fn resolve_pending(&self, id: RequestId, result: Result<Value>) -> bool {
        let sender = self
            .pending
            .lock()
            .expect("pending lock poisoned")
            .remove(&id);
        sender.is_some_and(|sender| sender.send(result).is_ok())
    }

    fn remove_pending(&self, id: RequestId) {
        let _ = self
            .pending
            .lock()
            .expect("pending lock poisoned")
            .remove(&id);
    }

    fn fail_all(&self, error: Error) {
        let pending = std::mem::take(&mut *self.pending.lock().expect("pending lock poisoned"));
        for (_, sender) in pending {
            let _ = sender.send(Err(error.clone()));
        }
    }

    fn stop_current_session(&self) {
        if let Some(stop) = self
            .session_stop
            .lock()
            .expect("session stop lock poisoned")
            .as_ref()
        {
            let _ = stop.send(true);
        }
    }

    fn publish_notification(&self, raw: RawNotification) -> Result<()> {
        let raw = normalize_notification(raw, self.compatibility_mode).map_err(Error::protocol)?;
        let _ = self.raw_notifications.send(raw.clone());
        let capabilities = self
            .capabilities
            .read()
            .expect("capabilities lock poisoned")
            .clone();
        let _ = self.events.send(decode_event(raw, capabilities.as_ref()));
        Ok(())
    }

    fn queue_control(&self, generation: u64, message: Message) -> bool {
        if self.current_generation.load(Ordering::Acquire) != generation {
            return false;
        }
        let outbound = self
            .outbound_tx
            .read()
            .expect("outbound lock poisoned")
            .clone();
        outbound.is_some_and(|sender| sender.try_send(Outbound::Control(message)).is_ok())
    }

    fn session_ended(self: &Arc<Self>, generation: u64, error: Error, reconnectable: bool) {
        if self.current_generation.load(Ordering::Acquire) != generation {
            return;
        }
        if self.shutdown_requested.load(Ordering::Acquire) {
            return;
        }

        *self.outbound_tx.write().expect("outbound lock poisoned") = None;
        self.stop_current_session();
        self.fail_all(error.clone());
        // A replacement connection can expose a different MCSMP version or
        // extension schema. Do not report stale capabilities while reconnecting.
        *self
            .capabilities
            .write()
            .expect("capabilities lock poisoned") = None;

        let should_reconnect = reconnectable && self.reconnect_policy.is_enabled();
        {
            let mut state = self.state.write().expect("state lock poisoned");
            if matches!(*state, ConnectionState::Closing | ConnectionState::Closed) {
                return;
            }
            *state = if should_reconnect {
                ConnectionState::Reconnecting
            } else if matches!(error, Error::Closed) {
                ConnectionState::Closed
            } else {
                ConnectionState::Failed
            };
        }

        if should_reconnect {
            self.start_reconnect_task();
        } else {
            let _ = self.event_shutdown_tx.send(true);
        }
    }

    fn start_reconnect_task(self: &Arc<Self>) {
        let mut task = self
            .reconnect_task
            .lock()
            .expect("reconnect task lock poisoned");
        if task.as_ref().is_some_and(|task| !task.is_finished()) {
            return;
        }
        let inner = Arc::clone(self);
        *task = Some(tokio::spawn(async move {
            reconnect_loop(inner).await;
        }));
    }
}

/// Messages owned by the single WebSocket writer task for one connection.
#[derive(Debug)]
enum Outbound {
    Request(OutboundRequest),
    Control(Message),
}

/// Starts reader and writer tasks for one successful WebSocket session.
pub(crate) fn start_session(inner: &Arc<ClientInner>, socket: Socket) {
    if inner.shutdown_requested.load(Ordering::Acquire) {
        return;
    }

    inner.stop_current_session();
    let generation = inner.current_generation.fetch_add(1, Ordering::AcqRel) + 1;
    let (outbound_tx, outbound_rx) = mpsc::channel(inner.channel_capacity);
    let (session_stop_tx, session_stop_rx) = watch::channel(false);
    *inner.outbound_tx.write().expect("outbound lock poisoned") = Some(outbound_tx);
    *inner
        .session_stop
        .lock()
        .expect("session stop lock poisoned") = Some(session_stop_tx);
    *inner.state.write().expect("state lock poisoned") = ConnectionState::Connected;

    let (sink, stream) = socket.split();
    let reader = tokio::spawn(reader_task(
        Arc::clone(inner),
        generation,
        stream,
        session_stop_rx.clone(),
        inner.shutdown_tx.subscribe(),
    ));
    let writer = tokio::spawn(writer_task(
        Arc::clone(inner),
        generation,
        sink,
        outbound_rx,
        session_stop_rx,
        inner.shutdown_tx.subscribe(),
    ));
    inner
        .session_tasks
        .lock()
        .expect("session tasks lock poisoned")
        .extend([reader, writer]);
}

async fn reader_task<S>(
    inner: Arc<ClientInner>,
    generation: u64,
    mut stream: S,
    mut session_stop: watch::Receiver<bool>,
    mut client_shutdown: watch::Receiver<bool>,
) where
    S: futures_util::Stream<Item = std::result::Result<Message, WebSocketError>> + Unpin,
{
    loop {
        tokio::select! {
            changed = client_shutdown.changed() => {
                if changed.is_err() || *client_shutdown.borrow() {
                    break;
                }
            }
            changed = session_stop.changed() => {
                if changed.is_err() || *session_stop.borrow() {
                    break;
                }
            }
            message = stream.next() => {
                let Some(message) = message else {
                    inner.session_ended(generation, Error::Closed, true);
                    break;
                };

                match message {
                    Ok(Message::Text(text)) => match parse_inbound(text.as_ref()) {
                        Ok(Inbound::Response { id, result }) => {
                            let _ = inner.resolve_pending(id, result);
                        }
                        Ok(Inbound::Notification { method, params }) => {
                            if let Err(error) = inner.publish_notification(RawNotification { method, params }) {
                                inner.session_ended(generation, error, false);
                                break;
                            }
                        }
                        Err(error) => {
                            inner.session_ended(generation, error, false);
                            break;
                        }
                    },
                    Ok(Message::Ping(payload)) => {
                        if !inner.queue_control(generation, Message::Pong(payload)) {
                            inner.session_ended(generation, Error::transport("unable to queue WebSocket pong"), true);
                            break;
                        }
                    }
                    Ok(Message::Pong(_)) => {}
                    Ok(Message::Close(_)) => {
                        inner.session_ended(generation, Error::Closed, true);
                        break;
                    }
                    Ok(Message::Binary(_)) => {
                        inner.session_ended(generation, Error::protocol("binary WebSocket frames are not valid MCSMP JSON-RPC text"), false);
                        break;
                    }
                    Ok(_) => {
                        inner.session_ended(generation, Error::protocol("unsupported WebSocket frame received from peer"), false);
                        break;
                    }
                    Err(error) => {
                        inner.session_ended(generation, Error::transport(error.to_string()), true);
                        break;
                    }
                }
            }
        }
    }
}

async fn writer_task<S>(
    inner: Arc<ClientInner>,
    generation: u64,
    mut sink: S,
    mut outbound: mpsc::Receiver<Outbound>,
    mut session_stop: watch::Receiver<bool>,
    mut client_shutdown: watch::Receiver<bool>,
) where
    S: futures_util::Sink<Message, Error = WebSocketError> + Unpin,
{
    loop {
        tokio::select! {
            changed = client_shutdown.changed() => {
                if changed.is_err() || *client_shutdown.borrow() {
                    let _ = sink.send(Message::Close(None)).await;
                    break;
                }
            }
            changed = session_stop.changed() => {
                if changed.is_err() || *session_stop.borrow() {
                    let _ = sink.send(Message::Close(None)).await;
                    break;
                }
            }
            outbound = outbound.recv() => {
                let Some(outbound) = outbound else {
                    let _ = sink.send(Message::Close(None)).await;
                    break;
                };
                let message = match outbound {
                    Outbound::Request(request) => Message::Text(request.text.into()),
                    Outbound::Control(message) => message,
                };
                if let Err(error) = sink.send(message).await {
                    inner.session_ended(generation, Error::transport(error.to_string()), true);
                    break;
                }
            }
        }
    }
}

async fn reconnect_loop(inner: Arc<ClientInner>) {
    let mut attempt = 1_usize;
    loop {
        if inner.shutdown_requested.load(Ordering::Acquire) {
            return;
        }
        let Some(delay) = inner.reconnect_policy.delay_for_attempt(attempt) else {
            *inner.state.write().expect("state lock poisoned") = ConnectionState::Failed;
            let _ = inner.event_shutdown_tx.send(true);
            return;
        };
        sleep(delay).await;
        if inner.shutdown_requested.load(Ordering::Acquire) {
            return;
        }

        match open_socket(&inner.reconnect_config).await {
            Ok(socket) => {
                if inner.shutdown_requested.load(Ordering::Acquire) {
                    return;
                }
                *inner
                    .capabilities
                    .write()
                    .expect("capabilities lock poisoned") = None;
                start_session(&inner, socket);

                // A new TCP/WebSocket session may advertise a different server
                // version or extension set. Refresh the cache but do not treat a
                // failed optional discovery call as a reason to replay requests.
                let client = Client::from_inner(Arc::clone(&inner));
                let discovery = client.discover().await;
                if discovery.is_ok() || client.state() == ConnectionState::Connected {
                    return;
                }
                // The replacement socket failed while discovery was in flight.
                // Its reader/writer already moved state to Reconnecting; retry.
                attempt = attempt.saturating_add(1);
            }
            Err(_) => {
                attempt = attempt.saturating_add(1);
            }
        }
    }
}
