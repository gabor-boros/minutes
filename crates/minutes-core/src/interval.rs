//! Work intervals: start time, elapsed length, and billable flag.

/// A tracked stretch of work: when it started, how long it ran, and whether
/// that time is billable.
///
/// This is distinct from [`std::time::Duration`], which only represents a
/// length of time. Defaults to a billable interval that starts at the Unix
/// epoch and has zero elapsed length.
///
/// # Examples
///
/// ```
/// use minutes_core::WorkInterval;
/// use std::time::{Duration, UNIX_EPOCH};
///
/// let zero = WorkInterval::default();
/// assert_eq!(zero.start(), UNIX_EPOCH);
/// assert_eq!(zero.to_string(), "0h 0m 0s (billable)");
///
/// let tracked = WorkInterval::new(UNIX_EPOCH, Duration::from_secs(3661), false);
/// assert_eq!(tracked.to_string(), "1h 1m 1s (non-billable)");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkInterval {
    start: std::time::SystemTime,
    elapsed: std::time::Duration,
    billable: bool,
}

impl Default for WorkInterval {
    fn default() -> Self {
        Self::new(
            std::time::SystemTime::UNIX_EPOCH,
            std::time::Duration::ZERO,
            true,
        )
    }
}

impl std::fmt::Display for WorkInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let secs = self.elapsed.as_secs();
        let billable = if self.billable {
            "billable"
        } else {
            "non-billable"
        };
        write!(
            f,
            "{}h {}m {}s ({})",
            secs / 3600,
            (secs % 3600) / 60,
            secs % 60,
            billable
        )
    }
}

impl WorkInterval {
    /// Creates an interval with an explicit start, length, and billable flag.
    ///
    /// # Examples
    ///
    /// ```
    /// use minutes_core::WorkInterval;
    /// use std::time::{Duration, UNIX_EPOCH};
    ///
    /// let interval = WorkInterval::new(UNIX_EPOCH, Duration::from_secs(90), false);
    /// assert_eq!(interval.start(), UNIX_EPOCH);
    /// assert!(!interval.is_billable());
    /// assert_eq!(interval.elapsed(), Duration::from_secs(90));
    /// ```
    #[must_use]
    pub const fn new(
        start: std::time::SystemTime,
        elapsed: std::time::Duration,
        billable: bool,
    ) -> Self {
        Self {
            start,
            elapsed,
            billable,
        }
    }

    /// Instant when the tracked interval started.
    #[must_use]
    pub const fn start(self) -> std::time::SystemTime {
        self.start
    }

    /// Wall-clock length of the tracked interval.
    #[must_use]
    pub const fn elapsed(self) -> std::time::Duration {
        self.elapsed
    }

    /// Whether the interval counts as billable time.
    #[must_use]
    pub const fn is_billable(self) -> bool {
        self.billable
    }

    /// Instant when the interval ended, if `start + elapsed` does not overflow.
    #[must_use]
    pub fn end(self) -> Option<std::time::SystemTime> {
        self.start.checked_add(self.elapsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration as StdDuration, UNIX_EPOCH};

    #[test]
    fn work_interval_default_is_epoch_zero_and_billable() {
        let interval = WorkInterval::default();
        assert_eq!(interval.start(), UNIX_EPOCH);
        assert_eq!(interval.elapsed(), StdDuration::ZERO);
        assert!(interval.is_billable());
        assert_eq!(interval.end(), Some(UNIX_EPOCH));
    }

    #[test]
    fn work_interval_new_preserves_fields() {
        let start = UNIX_EPOCH + StdDuration::from_secs(100);
        let billable = WorkInterval::new(start, StdDuration::from_secs(90), true);
        let non_billable = WorkInterval::new(start, StdDuration::from_secs(90), false);

        assert_eq!(billable.start(), start);
        assert!(billable.is_billable());
        assert!(!non_billable.is_billable());
        assert_eq!(billable.elapsed(), StdDuration::from_secs(90));
        assert_eq!(billable.end(), Some(start + StdDuration::from_secs(90)));
    }

    #[test]
    fn work_interval_display_formats_hms_and_billable_flag() {
        let billable = WorkInterval::new(UNIX_EPOCH, StdDuration::from_secs(3661), true);
        assert_eq!(billable.to_string(), "1h 1m 1s (billable)");

        let non_billable = WorkInterval::new(UNIX_EPOCH, StdDuration::from_secs(59), false);
        assert_eq!(non_billable.to_string(), "0h 0m 59s (non-billable)");

        let zero = WorkInterval::default();
        assert_eq!(zero.to_string(), "0h 0m 0s (billable)");
    }

    #[test]
    fn work_interval_is_copy() {
        let original = WorkInterval::new(UNIX_EPOCH, StdDuration::from_secs(1), true);
        let copied = original;
        assert_eq!(original, copied);
    }
}
