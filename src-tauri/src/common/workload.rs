//! How many readers the workstation may contact at once, and how many
//! PostgreSQL connections that leaves for enrollment and the window.
//!
//! The registered-device list is a separate limit. Raising this number does
//! not register more doors, and registering more doors does not raise this number.

use std::time::Duration;

/// Safe default. Four readers at a time, with connections left for the window.
pub const RECOMMENDED_CONCURRENCY: usize = 4;
/// Hard ceiling. Forty is allowed only when the operator sets it.
pub const MAX_POLL_CONCURRENCY: usize = 40;
pub const MIN_POLL_CONCURRENCY: usize = 1;
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(30);
pub const MIN_POLL_INTERVAL: Duration = Duration::from_secs(5);
pub const MAX_POLL_INTERVAL: Duration = Duration::from_secs(3_600);
/// Connections kept for enrollment, user management, and the window.
pub const FOREGROUND_POOL_RESERVE: u32 = 3;

/// Parsed `VSMART_POLL_CONCURRENCY` and `VSMART_POLL_INTERVAL_SECS`.
///
/// Out-of-range values are clamped. The window still starts, and `detail`
/// says what was changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkloadConfig {
    pub concurrency: usize,
    pub interval: Duration,
    pub pool_size: u32,
    pub clamped: bool,
    pub detail: String,
}

impl WorkloadConfig {
    pub fn from_env() -> Self {
        Self::from_values(
            std::env::var("VSMART_POLL_CONCURRENCY").ok().as_deref(),
            std::env::var("VSMART_POLL_INTERVAL_SECS").ok().as_deref(),
        )
    }

    pub fn from_values(concurrency: Option<&str>, interval_secs: Option<&str>) -> Self {
        let (concurrency, concurrency_note) = parse_concurrency(concurrency);
        let (interval, interval_note) = parse_interval(interval_secs);
        let mut detail = String::new();
        if let Some(note) = concurrency_note {
            detail.push_str(&note);
        }
        if let Some(note) = interval_note {
            if !detail.is_empty() {
                detail.push(' ');
            }
            detail.push_str(&note);
        }
        Self {
            concurrency,
            interval,
            pool_size: pool_size_for(concurrency),
            clamped: !detail.is_empty(),
            detail,
        }
    }

    pub fn guidance(&self) -> String {
        let mut text = format!(
            "Polling {} readers at a time every {}s. The database pool is {} connections, with {} reserved for enrollment and the window. Recommended concurrency is {}. The maximum is {}.",
            self.concurrency,
            self.interval.as_secs(),
            self.pool_size,
            FOREGROUND_POOL_RESERVE,
            RECOMMENDED_CONCURRENCY,
            MAX_POLL_CONCURRENCY
        );
        if self.concurrency >= 16 {
            text.push_str(
                " This setting is high for a local PostgreSQL install. If enrollment stalls or the database refuses connections, set VSMART_POLL_CONCURRENCY=4.",
            );
        }
        if !self.detail.is_empty() {
            text.push(' ');
            text.push_str(&self.detail);
        }
        text
    }
}

pub fn pool_size_for(concurrency: usize) -> u32 {
    (concurrency as u32)
        .saturating_add(FOREGROUND_POOL_RESERVE)
        .clamp(5, (MAX_POLL_CONCURRENCY as u32) + FOREGROUND_POOL_RESERVE)
}

fn parse_concurrency(raw: Option<&str>) -> (usize, Option<String>) {
    let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return (RECOMMENDED_CONCURRENCY, None);
    };
    let Ok(parsed) = raw.parse::<usize>() else {
        return (
            RECOMMENDED_CONCURRENCY,
            Some(format!(
                "VSMART_POLL_CONCURRENCY={raw} is not a number, so concurrency is {RECOMMENDED_CONCURRENCY}."
            )),
        );
    };
    if parsed < MIN_POLL_CONCURRENCY {
        return (
            RECOMMENDED_CONCURRENCY,
            Some(format!(
                "VSMART_POLL_CONCURRENCY={parsed} is below 1, so concurrency is {RECOMMENDED_CONCURRENCY}."
            )),
        );
    }
    if parsed > MAX_POLL_CONCURRENCY {
        return (
            MAX_POLL_CONCURRENCY,
            Some(format!(
                "VSMART_POLL_CONCURRENCY={parsed} is above {MAX_POLL_CONCURRENCY}, so concurrency is {MAX_POLL_CONCURRENCY}."
            )),
        );
    }
    (parsed, None)
}

fn parse_interval(raw: Option<&str>) -> (Duration, Option<String>) {
    let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return (DEFAULT_POLL_INTERVAL, None);
    };
    let Ok(parsed) = raw.parse::<u64>() else {
        return (
            DEFAULT_POLL_INTERVAL,
            Some(format!(
                "VSMART_POLL_INTERVAL_SECS={raw} is not a number, so the interval is {}s.",
                DEFAULT_POLL_INTERVAL.as_secs()
            )),
        );
    };
    let requested = Duration::from_secs(parsed);
    if requested < MIN_POLL_INTERVAL {
        return (
            MIN_POLL_INTERVAL,
            Some(format!(
                "VSMART_POLL_INTERVAL_SECS={parsed} is below {}s, so the interval is {}s.",
                MIN_POLL_INTERVAL.as_secs(),
                MIN_POLL_INTERVAL.as_secs()
            )),
        );
    }
    if requested > MAX_POLL_INTERVAL {
        return (
            MAX_POLL_INTERVAL,
            Some(format!(
                "VSMART_POLL_INTERVAL_SECS={parsed} is above {}s, so the interval is {}s.",
                MAX_POLL_INTERVAL.as_secs(),
                MAX_POLL_INTERVAL.as_secs()
            )),
        );
    }
    (requested, None)
}

/// Sleep this long before the next cycle. A cycle that already ran longer than
/// the interval starts the next pass immediately instead of adding another wait.
pub fn wait_after_cycle(interval: Duration, elapsed: Duration) -> Duration {
    interval.saturating_sub(elapsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_values_use_the_recommended_default() {
        let config = WorkloadConfig::from_values(None, None);
        assert_eq!(config.concurrency, 4);
        assert_eq!(config.interval, Duration::from_secs(30));
        assert_eq!(config.pool_size, 7);
        assert!(!config.clamped);
    }

    #[test]
    fn concurrency_is_clamped_to_the_documented_ceiling() {
        let config = WorkloadConfig::from_values(Some("80"), Some("1"));
        assert_eq!(config.concurrency, 40);
        assert_eq!(config.pool_size, 43);
        assert_eq!(config.interval, Duration::from_secs(5));
        assert!(config.clamped);
        assert!(config.detail.contains("80"));
        assert!(config.guidance().contains("VSMART_POLL_CONCURRENCY=4"));
    }

    #[test]
    fn forty_is_accepted_when_configured() {
        let config = WorkloadConfig::from_values(Some("40"), Some("30"));
        assert_eq!(config.concurrency, 40);
        assert!(!config.clamped);
        assert!(config.guidance().contains("high"));
    }

    #[test]
    fn a_long_cycle_does_not_add_another_interval() {
        assert_eq!(
            wait_after_cycle(Duration::from_secs(30), Duration::from_secs(45)),
            Duration::ZERO
        );
        assert_eq!(
            wait_after_cycle(Duration::from_secs(30), Duration::from_secs(10)),
            Duration::from_secs(20)
        );
    }
}
