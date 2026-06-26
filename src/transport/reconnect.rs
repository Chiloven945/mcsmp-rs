//! Reconnection policy and supervisor for transport sessions.

use std::sync::Arc;
use std::time::Duration;

use tokio::time::sleep;

use crate::client::{Client, ConnectionState};
use crate::transport::session::{SessionController, start_session};
use crate::transport::websocket::open_socket;

/// Controls whether and how a client reconnects after an unexpected interruption.
///
/// Configure this through [`crate::ClientBuilder::reconnect_policy`]. The
/// policy applies only to unexpected transport failures or peer closes; it
/// does not reconnect after an explicit [`crate::Client::shutdown`].
///
/// Reconnection never retries an in-flight JSON-RPC request. Calls pending
/// when the connection failed complete with their terminal error, and new
/// calls made while reconnecting return [`crate::Error::Reconnecting`]. This
/// is essential for non-idempotent administration operations such as kicking,
/// banning, changing settings, saving, or stopping a server. After a new
/// session is established, the client clears stale capabilities and performs a
/// fresh discovery attempt without replaying any prior management operation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReconnectPolicy {
    /// Never reconnect automatically after an unexpected interruption.
    ///
    /// This is the default and leaves the client in a terminal state after the
    /// active session fails.
    #[default]
    Never,
    /// Retry connections after the same delay for every attempt.
    Fixed {
        /// Positive delay before every reconnect attempt.
        delay: Duration,
        /// Optional positive maximum number of attempts after one disconnection.
        ///
        /// `None` means retry indefinitely until an explicit shutdown.
        max_attempts: Option<usize>,
    },
    /// Retry connections using exponential backoff capped at `max_delay`.
    ///
    /// Delays double from `initial_delay` until they reach `max_delay`.
    Exponential {
        /// Positive delay before the first reconnect attempt.
        initial_delay: Duration,
        /// Positive upper bound for delays produced by the backoff schedule.
        max_delay: Duration,
        /// Optional positive maximum number of attempts after one disconnection.
        ///
        /// `None` means retry indefinitely until an explicit shutdown.
        max_attempts: Option<usize>,
    },
}

impl ReconnectPolicy {
    /// Creates a fixed-delay reconnect policy.
    ///
    /// The policy is validated when a client connects. `delay` must be
    /// non-zero; when supplied, `max_attempts` must be greater than zero.
    pub const fn fixed(delay: Duration, max_attempts: Option<usize>) -> Self {
        Self::Fixed {
            delay,
            max_attempts,
        }
    }

    /// Creates an exponential-backoff reconnect policy.
    ///
    /// Attempt one waits `initial_delay`, attempt two waits twice that value,
    /// and later attempts continue doubling until `max_delay` is reached. The
    /// policy is validated when a client connects: both durations must be
    /// non-zero and `initial_delay` cannot exceed `max_delay`.
    pub const fn exponential(
        initial_delay: Duration,
        max_delay: Duration,
        max_attempts: Option<usize>,
    ) -> Self {
        Self::Exponential {
            initial_delay,
            max_delay,
            max_attempts,
        }
    }

    /// Returns whether this policy enables automatic reconnect attempts.
    pub const fn is_enabled(&self) -> bool {
        !matches!(self, Self::Never)
    }

    /// Validates user-supplied timing and attempt limits.
    pub(crate) fn validate(&self) -> std::result::Result<(), String> {
        match self {
            Self::Never => Ok(()),
            Self::Fixed {
                delay,
                max_attempts,
            } => {
                if delay.is_zero() {
                    return Err("reconnect delay must be greater than zero".into());
                }
                if matches!(max_attempts, Some(0)) {
                    return Err("reconnect max_attempts must be greater than zero when set".into());
                }
                Ok(())
            }
            Self::Exponential {
                initial_delay,
                max_delay,
                max_attempts,
            } => {
                if initial_delay.is_zero() {
                    return Err("reconnect initial_delay must be greater than zero".into());
                }
                if max_delay.is_zero() {
                    return Err("reconnect max_delay must be greater than zero".into());
                }
                if initial_delay > max_delay {
                    return Err("reconnect initial_delay must not exceed max_delay".into());
                }
                if matches!(max_attempts, Some(0)) {
                    return Err("reconnect max_attempts must be greater than zero when set".into());
                }
                Ok(())
            }
        }
    }

    /// Returns the delay before `attempt`, where attempts begin at one.
    pub(crate) fn delay_for_attempt(&self, attempt: usize) -> Option<Duration> {
        match self {
            Self::Never => None,
            Self::Fixed {
                delay,
                max_attempts,
            } => allowed(*max_attempts, attempt).then_some(*delay),
            Self::Exponential {
                initial_delay,
                max_delay,
                max_attempts,
            } => {
                if !allowed(*max_attempts, attempt) {
                    return None;
                }
                let exponent = attempt.saturating_sub(1).min(63) as u32;
                let factor = 1_u32.checked_shl(exponent).unwrap_or(u32::MAX);
                Some(initial_delay.saturating_mul(factor).min(*max_delay))
            }
        }
    }
}

fn allowed(max_attempts: Option<usize>, attempt: usize) -> bool {
    max_attempts.is_none_or(|maximum| attempt <= maximum)
}

/// Starts at most one reconnect supervisor for `controller`.
pub(crate) fn schedule(controller: &Arc<SessionController>) {
    let mut task = controller
        .reconnect_task
        .lock()
        .expect("reconnect task lock poisoned");
    if task.as_ref().is_some_and(|task| !task.is_finished()) {
        return;
    }
    let controller = Arc::clone(controller);
    *task = Some(tokio::spawn(async move {
        reconnect_loop(controller).await;
    }));
}

async fn reconnect_loop(controller: Arc<SessionController>) {
    let mut attempt = 1_usize;
    loop {
        if controller.shutdown_requested() {
            return;
        }
        let Some(delay) = controller.reconnect_policy().delay_for_attempt(attempt) else {
            controller.finish_reconnect_failure();
            return;
        };
        sleep(delay).await;
        if controller.shutdown_requested() {
            return;
        }

        match open_socket(controller.websocket_config()).await {
            Ok(socket) => {
                if controller.shutdown_requested() {
                    return;
                }
                controller.clear_capabilities();
                start_session(&controller, socket);

                // Discovery refreshes the cache for the new server session. It is
                // intentionally never a replay of calls that failed before reconnect.
                let client = Client::from_controller(Arc::clone(&controller));
                if client.discover().await.is_ok() || client.state() == ConnectionState::Connected {
                    return;
                }
                attempt = attempt.saturating_add(1);
            }
            Err(_) => attempt = attempt.saturating_add(1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_and_caps_backoff() {
        let policy = ReconnectPolicy::exponential(
            Duration::from_millis(10),
            Duration::from_millis(40),
            Some(3),
        );
        assert!(policy.validate().is_ok());
        assert_eq!(policy.delay_for_attempt(1), Some(Duration::from_millis(10)));
        assert_eq!(policy.delay_for_attempt(2), Some(Duration::from_millis(20)));
        assert_eq!(policy.delay_for_attempt(3), Some(Duration::from_millis(40)));
        assert_eq!(policy.delay_for_attempt(4), None);
    }
}
