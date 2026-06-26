//! Reconnection configuration for [`crate::Client`].

use std::time::Duration;

/// Controls whether and how a client reconnects after an unexpected WebSocket
/// close or transport failure.
///
/// Reconnection never retries an in-flight JSON-RPC request. Calls that were
/// pending when the connection failed complete with the original terminal
/// error; callers must decide whether a particular operation is safe to issue
/// again. This is especially important for non-idempotent operations such as
/// kicking, banning, or stopping a server.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReconnectPolicy {
    /// Do not reconnect automatically. This is the default.
    #[default]
    Never,

    /// Retry connections after a constant delay.
    ///
    /// `max_attempts: None` means retry forever. A configured value must be
    /// greater than zero.
    Fixed {
        /// Delay before every reconnect attempt.
        delay: Duration,
        /// Maximum reconnect attempts after one disconnection.
        max_attempts: Option<usize>,
    },

    /// Retry connections using exponential backoff capped at `max_delay`.
    ///
    /// The first retry waits `initial_delay`; each subsequent retry doubles
    /// the delay until it reaches `max_delay`. `max_attempts: None` means
    /// retry forever. A configured value must be greater than zero.
    Exponential {
        /// Delay before the first reconnect attempt.
        initial_delay: Duration,
        /// Largest delay used by the backoff schedule.
        max_delay: Duration,
        /// Maximum reconnect attempts after one disconnection.
        max_attempts: Option<usize>,
    },
}

impl ReconnectPolicy {
    /// Returns a fixed-delay reconnect policy.
    pub const fn fixed(delay: Duration, max_attempts: Option<usize>) -> Self {
        Self::Fixed {
            delay,
            max_attempts,
        }
    }

    /// Returns an exponential-backoff reconnect policy.
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

    /// Returns whether the client should attempt automatic reconnection.
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
