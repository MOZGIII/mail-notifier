//! Exponential backoff calculator.

#![no_std]

use core::time::Duration;

/// An exponential backoff state.
#[derive(Debug, Clone)]
pub struct State {
    /// Factor to multiply the current delay to calculate the next one.
    pub factor: u32,

    /// Delay clamp.
    pub max: Duration,

    /// Precomputed delay value to return.
    ///
    /// Initialize with the first delay value to expect.
    pub value: Duration,
}

impl State {
    /// Obtain the stored delay value and precompute next one.
    pub fn advance(&mut self) -> Duration {
        let current = self.value;
        self.value = current.saturating_mul(self.factor).min(self.max);
        current
    }

    /// Peek the stored delay value.
    pub const fn peek(&self) -> Duration {
        self.value
    }
}
