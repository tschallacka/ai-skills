// MODE: DEV
// PACKAGE: PROD
//! The derived threshold: `ceil(members_total / divisor)`, floored, unless
//! overridden by `--threshold`/`CI_SCOPE_THRESHOLD` with a genuinely usable
//! (non-empty, all-digit) value.
//!
//! Deliberate simplification: bash's own coercion also tracks a
//! `"derived (ignored an unusable override)"` source string for a
//! non-numeric override, but that string never actually reaches any
//! `decide()` reason -- the real script's own `threshold_label` is only ever
//! built from the "given" branch (non-empty threshold) or the plain derived
//! formula, never from that intermediate source string. It is dead
//! bookkeeping in the original; this port drops it rather than reproduce an
//! internal value with no observable effect.
//!
//! Also deliberate: `divisor`'s bash coercion only special-cases the literal
//! string `"0"`; a leading-zero override like `"00"` passes bash's own
//! all-digits check and is then evaluated in `$(( ))` arithmetic, where a
//! leading zero means octal -- `00` is octal `0`, a division-by-zero bash
//! itself does not guard against. This port treats any override whose
//! PARSED numeric value is zero as unusable, which is strictly safer and
//! does not reproduce that latent bash arithmetic hazard.

pub struct Resolved {
    pub value: u64,
    pub label: String,
}

fn is_all_digits(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

/// `raw_env`: `CI_SCOPE_THRESHOLD`'s raw value (empty string if unset).
/// `raw_flag`: `--threshold`'s raw value, if the flag was given.
pub fn resolve(
    raw_env: &str,
    raw_flag: Option<&str>,
    members_total: u64,
    divisor: u64,
    floor: u64,
) -> Resolved {
    let (raw, given_via_flag) = match raw_flag {
        Some(v) => (v.to_string(), true),
        None => (raw_env.to_string(), false),
    };
    if is_all_digits(&raw) {
        if let Ok(value) = raw.parse::<u64>() {
            let source = if given_via_flag {
                "given"
            } else {
                "CI_SCOPE_THRESHOLD"
            };
            return Resolved {
                value,
                label: format!("{value} from {source}"),
            };
        }
    }
    let derived = members_total.div_ceil(divisor).max(floor);
    Resolved {
        value: derived,
        label: format!("{derived} = ceil({members_total}/{divisor}), floor {floor}"),
    }
}

/// `CI_SCOPE_DIVISOR`'s coercion: empty, non-digit, or numerically zero
/// falls back to the default (4).
pub fn coerce_divisor(raw: &str, default: u64) -> u64 {
    if is_all_digits(raw) {
        if let Ok(v) = raw.parse::<u64>() {
            if v != 0 {
                return v;
            }
        }
    }
    default
}

/// `CI_SCOPE_FLOOR`'s coercion: empty or non-digit falls back to the
/// default (5).
pub fn coerce_floor(raw: &str, default: u64) -> u64 {
    if is_all_digits(raw) {
        if let Ok(v) = raw.parse::<u64>() {
            return v;
        }
    }
    default
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_override_derives() {
        let r = resolve("", None, 78, 4, 5);
        assert_eq!(r.value, 20);
        assert_eq!(r.label, "20 = ceil(78/4), floor 5");
    }

    #[test]
    fn a_junk_env_override_derives() {
        let r = resolve("abc", None, 78, 4, 5);
        assert_eq!(r.value, 20);
    }

    #[test]
    fn a_junk_flag_override_derives() {
        let r = resolve("", Some("abc"), 78, 4, 5);
        assert_eq!(r.value, 20);
    }

    #[test]
    fn a_usable_flag_override_wins_and_is_labeled_given() {
        let r = resolve("", Some("7"), 78, 4, 5);
        assert_eq!(r.value, 7);
        assert_eq!(r.label, "7 from given");
    }

    #[test]
    fn a_usable_env_override_is_labeled_by_the_env_var() {
        let r = resolve("7", None, 78, 4, 5);
        assert_eq!(r.value, 7);
        assert_eq!(r.label, "7 from CI_SCOPE_THRESHOLD");
    }

    #[test]
    fn the_flag_takes_precedence_over_the_env_var() {
        let r = resolve("7", Some("12"), 78, 4, 5);
        assert_eq!(r.value, 12);
        assert_eq!(r.label, "12 from given");
    }

    #[test]
    fn an_empty_flag_value_is_treated_as_no_override() {
        // bash: --threshold "" leaves threshold empty, and the coercion's
        // first case arm (`''`) resets threshold_source to "derived"
        // regardless of how it got there.
        let r = resolve("", Some(""), 78, 4, 5);
        assert_eq!(r.value, 20);
        assert_eq!(r.label, "20 = ceil(78/4), floor 5");
    }

    #[test]
    fn derived_never_rounds_below_the_floor() {
        let r = resolve("", None, 8, 4, 5);
        assert_eq!(r.value, 5);
    }

    #[test]
    fn ceil_rounds_up_on_a_remainder() {
        // ceil(10/4) = 3 (2.5 rounds up), floor 1 does not raise it further.
        let r = resolve("", None, 10, 4, 1);
        assert_eq!(r.value, 3);
    }

    #[test]
    fn divisor_coercion_falls_back_on_empty_non_digit_or_zero() {
        assert_eq!(coerce_divisor("", 4), 4);
        assert_eq!(coerce_divisor("abc", 4), 4);
        assert_eq!(coerce_divisor("0", 4), 4);
        assert_eq!(coerce_divisor("00", 4), 4);
        assert_eq!(coerce_divisor("2", 4), 2);
    }

    #[test]
    fn floor_coercion_falls_back_on_empty_or_non_digit() {
        assert_eq!(coerce_floor("", 5), 5);
        assert_eq!(coerce_floor("abc", 5), 5);
        assert_eq!(coerce_floor("3", 5), 3);
        assert_eq!(coerce_floor("0", 5), 0);
    }
}
