//! Asynchronous typed notification delivery.
//!
//! [`EventStream`] wraps a Tokio broadcast subscription in the standard
//! `futures_core::Stream` shape while also providing the convenient
//! [`EventStream::recv`] method. It is intentionally independent per caller:
//! two subscriptions can progress at different speeds, and each observes its
//! own lag state.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::{Stream, StreamExt};
use thiserror::Error;
use tokio::sync::broadcast;

use super::Event;

/// Asynchronous stream of [`Event`] values produced by [`crate::Client::subscribe`].
///
/// The stream implements `futures_util::Stream<Item = Result<Event,
/// EventStreamError>>`, so callers can use stream combinators or call
/// [`Self::recv`] directly. Each stream has a bounded broadcast receiver; if
/// it cannot keep up, it yields [`EventStreamError::Lagged`] once and can then
/// continue receiving newer events.
///
/// When the client closes, events already accepted by the receiver are drained
/// first. The stream then ends, and [`Self::recv`] reports
/// [`EventStreamError::Closed`].
pub struct EventStream {
    inner: Pin<Box<dyn Stream<Item = Result<Event, EventStreamError>> + Send>>,
}

impl std::fmt::Debug for EventStream {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EventStream")
            .finish_non_exhaustive()
    }
}

impl EventStream {
    pub(crate) fn new(
        receiver: broadcast::Receiver<Event>,
        shutdown: tokio::sync::watch::Receiver<bool>,
    ) -> Self {
        struct State {
            receiver: broadcast::Receiver<Event>,
            shutdown: tokio::sync::watch::Receiver<bool>,
        }

        let stream =
            futures_util::stream::unfold(State { receiver, shutdown }, |mut state| async move {
                loop {
                    // A session can end immediately after receiving a notification. Drain
                    // events already accepted by the broadcast receiver before observing
                    // shutdown so callers never lose the final notifications of a session.
                    match state.receiver.try_recv() {
                        Ok(event) => return Some((Ok(event), state)),
                        Err(broadcast::error::TryRecvError::Lagged(dropped)) => {
                            return Some((Err(EventStreamError::Lagged { dropped }), state));
                        }
                        Err(broadcast::error::TryRecvError::Closed) => return None,
                        Err(broadcast::error::TryRecvError::Empty) => {}
                    }

                    if *state.shutdown.borrow() {
                        return None;
                    }

                    tokio::select! {
                        biased;

                        received = state.receiver.recv() => match received {
                            Ok(event) => return Some((Ok(event), state)),
                            Err(broadcast::error::RecvError::Lagged(dropped)) => {
                                return Some((Err(EventStreamError::Lagged { dropped }), state));
                            }
                            Err(broadcast::error::RecvError::Closed) => return None,
                        },
                        changed = state.shutdown.changed() => {
                            if changed.is_err() {
                                return None;
                            }
                            // Re-enter the loop to drain any notification that was received
                            // before the terminal session state was observed.
                        }
                    }
                }
            });
        Self {
            inner: Box::pin(stream),
        }
    }

    /// Waits asynchronously for the next event or delivery error.
    ///
    /// This is equivalent to awaiting the next `Stream` item, but converts the
    /// end of the stream into [`EventStreamError::Closed`]. A
    /// [`EventStreamError::Lagged`] result means this subscriber missed one or
    /// more notifications; query the relevant official API (`players`, `bans`,
    /// `server`, and so on) before treating later events as a complete local
    /// state history.
    pub async fn recv(&mut self) -> Result<Event, EventStreamError> {
        match self.next().await {
            Some(Ok(event)) => Ok(event),
            Some(Err(error)) => Err(error),
            None => Err(EventStreamError::Closed),
        }
    }
}

impl Stream for EventStream {
    type Item = Result<Event, EventStreamError>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().inner.as_mut().poll_next(context)
    }
}

/// Delivery condition emitted while consuming an [`EventStream`].
///
/// These errors describe the local subscription rather than a JSON-RPC
/// failure returned by Minecraft. The enum is `non_exhaustive`, so downstream
/// code should include a fallback arm.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum EventStreamError {
    /// The subscriber did not consume events quickly enough for the broadcast buffer.
    ///
    /// The stream remains usable after this error, but its history has a gap.
    /// Re-query authoritative server state before relying on later events.
    #[error("event subscriber lagged behind and missed {dropped} events")]
    Lagged {
        /// Number of notifications discarded for this individual subscriber.
        dropped: u64,
    },
    /// The client closed its event broadcaster and no queued events remain.
    #[error("event stream is closed")]
    Closed,
}

#[cfg(test)]
mod tests {
    use tokio::sync::{broadcast, watch};

    use super::*;

    #[tokio::test]
    async fn drains_queued_events_after_shutdown() {
        let (events, receiver) = broadcast::channel(2);
        let (shutdown_tx, shutdown) = watch::channel(false);
        let mut stream = EventStream::new(receiver, shutdown);

        events.send(Event::ServerStarted).unwrap();
        shutdown_tx.send(true).unwrap();

        assert!(matches!(stream.recv().await, Ok(Event::ServerStarted)));
        assert!(matches!(stream.recv().await, Err(EventStreamError::Closed)));
    }
}
