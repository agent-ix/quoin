// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Number serialization, ECMAScript `Number::toString` exactly.
//!
//! RFC 8785 §3.2.2.3 does not define its own number format: it *adopts*
//! ECMAScript's, so "canonical" here means whatever `JSON.stringify` would
//! print. That format is not Rust's:
//!
//! | value | Rust `{}` / `ryu` | ECMAScript |
//! |---|---|---|
//! | `1e21` | `1e21` | `1e+21` |
//! | `1e20` | `1e20` | `100000000000000000000` |
//! | `-0.0` | `-0` | `0` |
//! | `1.0` | `1` / `1.0` | `1` |
//! | `1e-7` | `1e-7` | `1e-7` |
//!
//! The exponent sign, the decision threshold between fixed and exponential
//! notation, and negative zero are all places where a plausible Rust
//! implementation silently produces different bytes and therefore a different
//! digest. `ryu-js` implements the ECMAScript algorithm; this module is the
//! single place that decision is made.

/// Serialize a finite double the way `JSON.stringify` would.
///
/// # Panics
///
/// Never: `JsonNumber` admits only finite values, and `ryu_js` is total over
/// finite doubles. A non-finite value reaching here would be a bug in
/// `JsonNumber`, and is returned as `null`-free text rather than panicking.
#[must_use]
pub fn format_number(value: f64) -> String {
    if !value.is_finite() {
        // Unreachable through `JsonNumber`; kept total rather than panicking on
        // a persistence path. `JsonNumber::new` is the enforcement point.
        return "null".to_owned();
    }
    // ECMAScript Number::toString step 1: both zeros print as "0".
    if value == 0.0 {
        return "0".to_owned();
    }
    ryu_js::Buffer::new().format_finite(value).to_owned()
}

#[cfg(test)]
mod tests {
    use super::format_number;

    #[test]
    fn tc_380_negative_zero_prints_as_zero() {
        assert_eq!(format_number(-0.0), "0");
        assert_eq!(format_number(0.0), "0");
    }

    #[test]
    fn tc_380_exponent_threshold_and_sign_follow_ecmascript() {
        assert_eq!(format_number(1e20), "100000000000000000000");
        assert_eq!(format_number(1e21), "1e+21");
        assert_eq!(format_number(1e-6), "0.000001");
        assert_eq!(format_number(1e-7), "1e-7");
    }

    #[test]
    fn tc_380_integral_doubles_print_without_a_fraction() {
        assert_eq!(format_number(1.0), "1");
        assert_eq!(format_number(-1.0), "-1");
        assert_eq!(format_number(9_007_199_254_740_992.0), "9007199254740992");
    }
}
