//! Asynchronous delivery for typed MCSMP notifications.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::{Stream, StreamExt};
use thiserror::Error;
use tokio::sync::broadcast;

use super::Event;

/// A stream of [`Event`] values produced by [`crate::Client::subscribe`].
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
                if *state.shutdown.borrow() {
                    return None;
                }
                let item = tokio::select! {
                    changed = state.shutdown.changed() => {
                        if changed.is_err() || *state.shutdown.borrow() {
                            return None;
                        }
                        return None;
                    }
                    received = state.receiver.recv() => match received {
                        Ok(event) => Ok(event),
                        Err(broadcast::error::RecvError::Lagged(dropped)) => {
                            Err(EventStreamError::Lagged { dropped })
                        }
                        Err(broadcast::error::RecvError::Closed) => return None,
                    },
                };
                Some((item, state))
            });
        Self {
            inner: Box::pin(stream),
        }
    }

    /// Waits for the next event.
    ///
    /// [`EventStreamError::Lagged`] means this subscriber missed events and
    /// should re-synchronize state through the appropriate query API. A closed
    /// stream returns [`EventStreamError::Closed`].
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

/// A recoverable error emitted while consuming an [`EventStream`].
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum EventStreamError {
    /// The subscriber did not consume events quickly enough for the bounded
    /// broadcast buffer.
    #[error("event subscriber lagged behind and missed {dropped} events")]
    Lagged {
        /// Number of events discarded for this subscriber.
        dropped: u64,
    },
    /// The client closed its event broadcaster.
    #[error("event stream is closed")]
    Closed,
}
