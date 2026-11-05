//! Scan speed readings and the gauge's range.

use std::time::{Duration, Instant};

/// Bytes read at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpeedSample {
    pub at: Instant,
    pub bytes: i64,
}

/// Returns the bytes per second read in the window ending at `now`.
pub fn rate(samples: &[SpeedSample], now: Instant, window: Duration) -> f64 {
    if window.is_zero() || samples.is_empty() {
        return 0.0;
    }
    let cutoff = now.checked_sub(window).unwrap_or(now);
    let total: i64 = samples
        .iter()
        .filter(|sample| sample.at >= cutoff)
        .map(|sample| sample.bytes)
        .sum();
    total as f64 / window.as_secs_f64()
}

const GAUGE_STEPS: [f64; 8] = [40.0, 80.0, 200.0, 400.0, 1000.0, 2000.0, 4000.0, 8000.0];

/// Returns the smallest ceiling at or above `current` that leaves room for
/// `speed`, or the largest ceiling for a faster scan. It never lowers a
/// ceiling.
pub fn gauge_max(current: f64, speed: f64) -> f64 {
    let mut current = current;
    if current < GAUGE_STEPS[0] {
        current = GAUGE_STEPS[0];
    }
    for step in GAUGE_STEPS {
        if step < current {
            continue;
        }
        if speed <= step * 0.85 {
            return step;
        }
    }
    GAUGE_STEPS[GAUGE_STEPS.len() - 1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_uses_the_window() {
        let now = Instant::now();
        let samples = vec![
            SpeedSample {
                at: now,
                bytes: 100,
            },
            SpeedSample {
                at: now - Duration::from_millis(500),
                bytes: 100,
            },
            SpeedSample {
                at: now - Duration::from_secs(3),
                bytes: 500,
            },
        ];
        assert_eq!(rate(&samples, now, Duration::from_secs(1)), 200.0);
    }

    #[test]
    fn no_samples_is_zero() {
        assert_eq!(rate(&[], Instant::now(), Duration::from_secs(1)), 0.0);
    }

    #[test]
    fn gauge_starts_at_forty() {
        assert_eq!(gauge_max(0.0, 0.0), 40.0);
    }

    #[test]
    fn gauge_grows_before_the_needle_leaves_the_dial() {
        assert_eq!(gauge_max(40.0, 39.0), 80.0);
        assert_eq!(gauge_max(40.0, 2000.0), 4000.0);
        assert_eq!(gauge_max(8000.0, 9000.0), 8000.0);
    }

    #[test]
    fn gauge_never_lowers() {
        assert_eq!(gauge_max(400.0, 1.0), 400.0);
    }
}
