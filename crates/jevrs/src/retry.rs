use std::hash::{BuildHasher, Hasher, RandomState};

use core::time::Duration;

/// Controls retry count and exponential-backoff timing.
///
/// Attempts are zero-based: attempt zero waits for [`Self::base`], attempt one
/// waits for twice the base, and so on. Set [`Self::jitter`] to `false` when a
/// fixed schedule is required. Jitter draws fresh per-call entropy from
/// [`RandomState`]. A server-provided `Retry-After` always takes precedence
/// and is never jittered.
///
/// ```
/// use std::time::Duration;
/// use jevrs::RetryPolicy;
///
/// let policy = RetryPolicy { jitter: false, ..RetryPolicy::default() };
/// assert_eq!(policy.delay_for(1, None), Duration::from_secs(1));
/// assert_eq!(
///     policy.delay_for(0, Some(Duration::from_secs(3))),
///     Duration::from_secs(3),
/// );
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetryPolicy {
    /// Set this to cap attempts after the initial request.
    pub max_retries: u32,
    /// Set this to control the delay before the first exponential retry.
    pub base: Duration,
    /// Set this to cap both local backoff and server `Retry-After` delays.
    pub max: Duration,
    /// Keep this enabled to spread concurrent retries, or disable it for tests.
    pub jitter: bool,
}

impl RetryPolicy {
    /// Computes a delay when previewing or testing the configured schedule.
    #[must_use]
    pub fn delay_for(&self, attempt: u32, retry_after: Option<Duration>) -> Duration {
        if let Some(delay) = retry_after {
            return delay.min(self.max);
        }

        let multiplier = 1_u32.checked_shl(attempt).unwrap_or(u32::MAX);
        let delay = self.base.saturating_mul(multiplier).min(self.max);
        if !self.jitter || delay.is_zero() {
            return delay;
        }

        let random = RandomState::new().build_hasher().finish();
        let fraction = u32::try_from(random >> 32).unwrap_or(u32::MAX);
        let nanos = delay.as_nanos().saturating_mul(u128::from(fraction)) / u128::from(u32::MAX);
        let seconds = u64::try_from(nanos / 1_000_000_000).unwrap_or(u64::MAX);
        let subsec_nanos = u32::try_from(nanos % 1_000_000_000).unwrap_or(999_999_999);
        Duration::new(seconds, subsec_nanos)
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 2,
            base: Duration::from_millis(500),
            max: Duration::from_secs(8),
            jitter: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RetryPolicy;
    use core::time::Duration;

    fn fixed() -> RetryPolicy {
        RetryPolicy {
            jitter: false,
            ..RetryPolicy::default()
        }
    }

    #[test]
    fn exponential_sequence_without_jitter() {
        let policy = fixed();
        assert_eq!(policy.delay_for(0, None), Duration::from_millis(500));
        assert_eq!(policy.delay_for(1, None), Duration::from_secs(1));
        assert_eq!(policy.delay_for(2, None), Duration::from_secs(2));
        assert_eq!(policy.delay_for(3, None), Duration::from_secs(4));
    }

    #[test]
    fn exponential_delay_is_capped() {
        let policy = fixed();
        assert_eq!(policy.delay_for(4, None), Duration::from_secs(8));
        assert_eq!(policy.delay_for(100, None), Duration::from_secs(8));
    }

    #[test]
    fn jitter_draws_vary_within_the_unjittered_delay() {
        let policy = RetryPolicy::default();
        let unjittered = policy.base.saturating_mul(4);
        let draws: Vec<_> = (0..32).map(|_| policy.delay_for(2, None)).collect();

        assert!(
            draws
                .iter()
                .all(|delay| (Duration::ZERO..=unjittered).contains(delay))
        );
        assert!(draws.windows(2).any(|pair| pair[0] != pair[1]));
    }

    #[test]
    fn retry_after_takes_precedence_is_capped_and_is_not_jittered() {
        let policy = RetryPolicy::default();
        assert_eq!(
            policy.delay_for(3, Some(Duration::from_secs(2))),
            Duration::from_secs(2)
        );
        assert_eq!(
            policy.delay_for(0, Some(Duration::from_secs(30))),
            Duration::from_secs(8)
        );
    }
}
