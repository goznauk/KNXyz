use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeartbeatOptions {
    pub interval: Duration,
    pub timeout: Duration,
    pub max_missed: usize,
}

impl HeartbeatOptions {
    pub fn new(interval: Duration, timeout: Duration, max_missed: usize) -> Self {
        Self {
            interval,
            timeout,
            max_missed: max_missed.max(1),
        }
    }
}
