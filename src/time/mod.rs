pub fn current_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH).unwrap()
        .as_secs()
}

/// Trait abstraction over “now” for easy mocking.
pub trait TimeProvider {
    /// Returns the current UNIX timestamp in seconds.
    fn now(&self) -> u64;
}

/// Production clock, delegates to your real `current_unix()`.
pub struct RealTime;

impl TimeProvider for RealTime {
    fn now(&self) -> u64 {
        super::current_unix()
    }
}

/// A fake clock for tests.
pub struct FakeTime {
    pub timestamp: u64,
}

impl TimeProvider for FakeTime {
    fn now(&self) -> u64 {
        self.timestamp
    }
}