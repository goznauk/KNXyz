use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconnectPolicy {
    pub max_attempts: usize,
    pub initial_delay: Duration,
    pub max_delay: Duration,
}

impl ReconnectPolicy {
    pub fn bounded(max_attempts: usize, initial_delay: Duration, max_delay: Duration) -> Self {
        Self {
            max_attempts: max_attempts.max(1),
            initial_delay,
            max_delay,
        }
    }

    pub(crate) fn next_delay(self, current: Duration) -> Duration {
        current.saturating_mul(2).min(self.max_delay)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionEvent {
    Connected { channel_id: u8 },
    Disconnected,
    Reconnecting { attempt: usize, delay: Duration },
    Reconnected { attempt: usize, channel_id: u8 },
}
