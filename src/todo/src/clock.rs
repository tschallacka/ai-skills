// MODE: DEV
// PACKAGE: PROD
//! UTC timestamps, without a dependency and without a shell.
//!
//! `date -u +%Y-%m-%dT%H:%M:%SZ` would be shorter, and it is what the shell
//! tools this replaces did — but calling out to coreutils reintroduces exactly
//! the environment dependency this binary exists to remove. So the civil date
//! is computed here, the same way plan-crypt computes its own SHA-256 rather
//! than shelling out to a digest tool.
//!
//! Second precision, always UTC, always the same 20 characters. The register is
//! a tracked file, so a timestamp whose width or zone varied by machine would
//! show up as noise in every diff.

use std::time::{SystemTime, UNIX_EPOCH};

/// `YYYY-MM-DDTHH:MM:SSZ` for now.
pub fn now() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    from_epoch(seconds)
}

/// `YYYY-MM-DDTHH:MM:SSZ` for a Unix timestamp. Separated from `now()` so the
/// conversion is testable against known values rather than against the clock.
pub fn from_epoch(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let rem = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year,
        month,
        day,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Howard Hinnant's days-to-civil algorithm, which is the standard way to do
/// this without a calendar library: shift the epoch to 0000-03-01 so leap days
/// land at the end of the cycle, then unwind the 400/100/4-year eras.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11], March = 0
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::from_epoch;

    #[test]
    fn known_instants_convert_exactly() {
        // Each of these is checkable by hand or with `date -u -d @<n>`, which is
        // the point: the arithmetic is pinned to values, not to the clock.
        assert_eq!(from_epoch(0), "1970-01-01T00:00:00Z");
        assert_eq!(from_epoch(1), "1970-01-01T00:00:01Z");
        assert_eq!(from_epoch(86_399), "1970-01-01T23:59:59Z");
        assert_eq!(from_epoch(86_400), "1970-01-02T00:00:00Z");
        assert_eq!(from_epoch(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(from_epoch(1_709_164_800), "2024-02-29T00:00:00Z");
        assert_eq!(from_epoch(1_735_689_599), "2024-12-31T23:59:59Z");
        assert_eq!(from_epoch(1_735_689_600), "2025-01-01T00:00:00Z");
    }

    #[test]
    fn a_century_boundary_is_not_a_leap_year() {
        // 1900 and 2100 are not leap years; 2000 is. The 100/400 rules are
        // where a hand-rolled calendar usually goes wrong.
        assert_eq!(from_epoch(4_107_542_400), "2100-03-01T00:00:00Z");
        assert_eq!(from_epoch(4_107_456_000), "2100-02-28T00:00:00Z");
    }

    #[test]
    fn every_timestamp_is_the_same_width() {
        for seconds in [0_i64, 1_000_000_000, 1_735_689_600, 4_107_542_400] {
            assert_eq!(from_epoch(seconds).len(), 20, "at {seconds}");
        }
    }
}
