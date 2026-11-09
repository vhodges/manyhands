//! The clock seam (D6 design A1, R-5).
//!
//! The schedule model never reads the time itself. Whoever drives it reads a
//! [`Clock`] and passes the reading ([`Now`]) in, so tests advance both
//! clocks by hand and nothing sleeps. [`SystemClock`] is the one place the
//! process clocks are read.

use std::time::{Duration, Instant};

use time::OffsetDateTime;

/// The two clocks the schedule uses: wall time (UTC) for the deadlines the
/// library stores, and the monotonic clock for waits inside the process.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 5.
pub(crate) trait Clock {
    fn utc(&self) -> OffsetDateTime;
    fn mono(&self) -> Instant;

    /// Both readings, taken together.
    fn now(&self) -> Now {
        Now {
            utc: self.utc(),
            mono: self.mono(),
        }
    }
}

/// One reading of both clocks.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Now {
    pub utc: OffsetDateTime,
    pub mono: Instant,
}

/// The longest wait the model converts, so a far-future deadline cannot
/// overflow an `Instant`. The library clamps a deadline long before this.
const LONGEST_WAIT: Duration = Duration::from_secs(10 * 365 * 24 * 60 * 60);

impl Now {
    /// The monotonic instant `after` from now.
    pub fn after(&self, after: Duration) -> Instant {
        self.mono
            .checked_add(after.min(LONGEST_WAIT))
            .unwrap_or(self.mono)
    }

    /// A stored UTC deadline as a monotonic instant, converted at this
    /// reading. A deadline that has passed is now.
    pub fn mono_at(&self, deadline: OffsetDateTime) -> Instant {
        let left = deadline - self.utc;
        match Duration::try_from(left) {
            Ok(left) => self.after(left),
            Err(_) => self.mono,
        }
    }
}

/// The production clock: the only code in the model that reads the process
/// clocks.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 5.
pub(crate) struct SystemClock;

impl Clock for SystemClock {
    fn utc(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }

    fn mono(&self) -> Instant {
        Instant::now()
    }
}

/// A clock moved by hand. Not `Copy`; the two clocks start together and are
/// moved separately to make a suspend or a jump.
#[cfg(test)]
pub(crate) struct TestClock {
    readings: std::sync::Mutex<(OffsetDateTime, Instant)>,
}

#[cfg(test)]
impl TestClock {
    pub fn new() -> Self {
        let start = OffsetDateTime::from_unix_timestamp(1_800_000_000).expect("a valid time");
        Self {
            readings: std::sync::Mutex::new((start, SystemClock.mono())),
        }
    }

    /// Both clocks move together: time passing.
    pub fn advance(&self, by: Duration) {
        self.skew(time::Duration::try_from(by).expect("a small step"), by);
    }

    /// The two clocks move apart: a suspend (UTC moves, the monotonic clock
    /// does not) or a jump.
    pub fn skew(&self, utc: time::Duration, mono: Duration) {
        let mut readings = self.readings.lock().expect("the clock");
        readings.0 += utc;
        readings.1 += mono;
    }
}

#[cfg(test)]
impl Clock for TestClock {
    fn utc(&self) -> OffsetDateTime {
        self.readings.lock().expect("the clock").0
    }

    fn mono(&self) -> Instant {
        self.readings.lock().expect("the clock").1
    }
}
