//! Lifecycle management for one multiplexed WebSocket session.

use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::{broadcast, mpsc, watch};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::{Error as WebSocketError, Message};

use crate::capability::{Capabilities, CompatibilityMode};
use crate::client::ConnectionState;
use crate::events::{decode_event, normalize_notification, Event, RawNotification};
use crate::transport::jsonrpc::{parse_inbound, serialize_request, Inbound, OutboundRequest};
use crate::transport::reconnect::{self, ReconnectPolicy};
use crate::transport::request::PendingRequests;
use crate::transport::websocket::{Socket, WebSocketConfig};
use crate::{Error, Result};

const NOTIFICATION_BUFFER_CAPACITY: usize = 256;

/// Immutable options captured when a [`crate::Client`] connects.
pub(crate) struct SessionConfig {
    pub(crate) request_timeout: Duration,
    pub(crate) channel_capacity: usize,
    pub(crate) compatibility_mode: CompatibilityMode,
    pub(crate) reconnect_policy: ReconnectPolicy,
    pub(crate) websocket: WebSocketConfig,
}

/// Shared mutable state for all clones of one client.
pub(crate) struct SessionController {
    outbound_tx: RwLock<Option<mpsc::Sender<Outbound>>>,
    pending: PendingRequests,
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
    websocket: WebSocketConfig,
    session_tasks: Mutex<Vec<JoinHandle<()>>>,
    pub(crate) reconnect_task: Mutex<Option<JoinHandle<()>>>,
}

impl fmt::Debug for SessionController {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SessionController")
            .field("state", &self.state.read().ok())
            .field("request_timeout", &self.request_timeout)
            .field("compatibility_mode", &self.compatibility_mode)
            .field("reconnect_policy", &self.reconnect_policy)
            .finish_non_exhaustive()
    }
}

impl SessionController {
    pub(crate) fn new(config: SessionConfig) -> Self {
        let (raw_notifications, _) = broadcast::channel(NOTIFICATION_BUFFER_CAPACITY);
        let (events, _) = broadcast::channel(NOTIFICATION_BUFFER_CAPACITY);
        let (shutdown_tx, _) = watch::channel(false);
        let (event_shutdown_tx, _) = watch::channel(false);
        Self {
            outbound_tx: RwLock::new(None),
            pending: PendingRequests::new(),
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
            websocket: config.websocket,
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
    pub(crate) fn websocket_config(&self) -> &WebSocketConfig {
        &self.websocket
    }
    pub(crate) fn shutdown_requested(&self) -> bool {
        self.shutdown_requested.load(Ordering::Acquire)
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
    pub(crate) fn clear_capabilities(&self) {
        *self
            .capabilities
            .write()
            .expect("capabilities lock poisoned") = None;
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
        self.pending.fail_all(Error::Closed);
        let reconnect_task = self
            .reconnect_task
            .lock()
            .expect("reconnect task lock poisoned")
            .take();
        if let Some(task) = reconnect_task {
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
        let (id, receiver) = self.pending.register();
        let text = match serialize_request(id, method, params) {
            Ok(text) => text,
            Err(error) => {
                self.pending.remove(id);
                return Err(error);
            }
        };
        let Some(outbound) = self
            .outbound_tx
            .read()
            .expect("outbound lock poisoned")
            .clone()
        else {
            self.pending.remove(id);
            return Err(self.unavailable_error());
        };
        if outbound
            .send(Outbound::Request(OutboundRequest { text }))
            .await
            .is_err()
        {
            self.pending.remove(id);
            return Err(self.unavailable_error());
        }
        match timeout(self.request_timeout, receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(self.unavailable_error()),
            Err(_) => {
                self.pending.remove(id);
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
    fn stop_current_session(&self) {
        if let Some(stop) = self
            .session_stop
            .lock()
            .expect("session-stop lock poisoned")
            .as_ref()
        {
            let _ = stop.send(true);
        }
    }

    fn publish_notification(&self, raw: RawNotification) -> Result<()> {
        let raw = normalize_notification(raw, self.compatibility_mode).map_err(Error::protocol)?;
        let _ = self.raw_notifications.send(raw.clone());
        let capabilities = self.capabilities();
        let _ = self.events.send(decode_event(raw, capabilities.as_ref()));
        Ok(())
    }

    fn queue_control(&self, generation: u64, message: Message) -> bool {
        if self.current_generation.load(Ordering::Acquire) != generation {
            return false;
        }
        self.outbound_tx
            .read()
            .expect("outbound lock poisoned")
            .clone()
            .is_some_and(|sender| sender.try_send(Outbound::Control(message)).is_ok())
    }

    fn session_ended(self: &Arc<Self>, generation: u64, error: Error, reconnectable: bool) {
        if self.current_generation.load(Ordering::Acquire) != generation
            || self.shutdown_requested()
        {
            return;
        }
        *self.outbound_tx.write().expect("outbound lock poisoned") = None;
        self.stop_current_session();
        self.pending.fail_all(error.clone());
        self.clear_capabilities();
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
            reconnect::schedule(self);
        } else {
            let _ = self.event_shutdown_tx.send(true);
        }
    }

    pub(crate) fn finish_reconnect_failure(&self) {
        *self.state.write().expect("state lock poisoned") = ConnectionState::Failed;
        let _ = self.event_shutdown_tx.send(true);
    }
}

#[derive(Debug)]
enum Outbound {
    Request(OutboundRequest),
    Control(Message),
}

/// Starts reader and writer tasks for one successfully opened WebSocket session.
pub(crate) fn start_session(controller: &Arc<SessionController>, socket: Socket) {
    if controller.shutdown_requested() {
        return;
    }
    controller.stop_current_session();
    let generation = controller.current_generation.fetch_add(1, Ordering::AcqRel) + 1;
    let (outbound_tx, outbound_rx) = mpsc::channel(controller.channel_capacity);
    let (stop_tx, stop_rx) = watch::channel(false);
    *controller
        .outbound_tx
        .write()
        .expect("outbound lock poisoned") = Some(outbound_tx);
    *controller
        .session_stop
        .lock()
        .expect("session-stop lock poisoned") = Some(stop_tx);
    *controller.state.write().expect("state lock poisoned") = ConnectionState::Connected;
    let (sink, stream) = socket.split();
    let reader = tokio::spawn(reader_task(
        Arc::clone(controller),
        generation,
        stream,
        stop_rx.clone(),
        controller.shutdown_tx.subscribe(),
    ));
    let writer = tokio::spawn(writer_task(
        Arc::clone(controller),
        generation,
        sink,
        outbound_rx,
        stop_rx,
        controller.shutdown_tx.subscribe(),
    ));
    controller
        .session_tasks
        .lock()
        .expect("session tasks lock poisoned")
        .extend([reader, writer]);
}

async fn reader_task<S>(
    controller: Arc<SessionController>,
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
                    controller.session_ended(generation, Error::Closed, true);
                    break;
                };

                match message {
                    Ok(Message::Text(text)) => match parse_inbound(text.as_ref()) {
                        Ok(Inbound::Response { id, result }) => {
                            let _ = controller.pending.resolve(id, result);
                        }
                        Ok(Inbound::Notification { method, params }) => {
                            if let Err(error) =
                                controller.publish_notification(RawNotification { method, params })
                            {
                                controller.session_ended(generation, error, false);
                                break;
                            }
                        }
                        Err(error) => {
                            controller.session_ended(generation, error, false);
                            break;
                        }
                    },
                    Ok(Message::Ping(payload)) => {
                        if !controller.queue_control(generation, Message::Pong(payload)) {
                            controller.session_ended(
                                generation,
                                Error::transport("unable to queue WebSocket pong"),
                                true,
                            );
                            break;
                        }
                    }
                    Ok(Message::Pong(_)) => {}
                    Ok(Message::Close(_)) => {
                        controller.session_ended(generation, Error::Closed, true);
                        break;
                    }
                    Ok(Message::Binary(_)) => {
                        controller.session_ended(
                            generation,
                            Error::protocol(
                                "binary WebSocket frames are not valid MCSMP JSON-RPC text",
                            ),
                            false,
                        );
                        break;
                    }
                    Ok(_) => {
                        controller.session_ended(
                            generation,
                            Error::protocol("unsupported WebSocket frame received from peer"),
                            false,
                        );
                        break;
                    }
                    Err(error) => {
                        controller.session_ended(
                            generation,
                            Error::transport(error.to_string()),
                            true,
                        );
                        break;
                    }
                }
            }
        }
    }
}

async fn writer_task<S>(
    controller: Arc<SessionController>,
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
                    controller.session_ended(
                        generation,
                        Error::transport(error.to_string()),
                        true,
                    );
                    break;
                }
            }
        }
    }
}
