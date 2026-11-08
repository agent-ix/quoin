// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! How a measured value reads in a report (PLAT-1050).
//!
//! A producer's float carries rounding noise (`0.005972900000000001`) and a
//! duration in seconds is unreadable at either extreme (`4.9953e-7`). The
//! report shows durations in the unit that keeps the number near one, and
//! every value to a fixed count of significant digits. The stored value is
//! untouched; JSON output and comparisons still read it exactly.

use crate::common::scalar::js_f64_string;

/// Significant digits a duration is shown to.
const DURATION_DIGITS: usize = 4;
/// Significant digits any other value is shown to; enough to keep a real
/// reading, few enough to drop float noise.
const VALUE_DIGITS: usize = 6;

/// `value` rounded to `digits` significant digits, as JavaScript would print it.
fn significant(value: f64, digits: usize) -> String {
    if !value.is_finite() || value == 0.0 {
        return js_f64_string(value);
    }
    let rounded = format!("{value:.precision$e}", precision = digits - 1)
        .parse::<f64>()
        .unwrap_or(value);
    js_f64_string(rounded)
}

/// The `"{value} {unit}"` of a measured observation.
pub(crate) fn measured_text(value: Option<f64>, unit: &str) -> String {
    let Some(value) = value else {
        return format!("null {unit}");
    };
    if unit == "s" && value.is_finite() && value != 0.0 {
        let magnitude = value.abs();
        let (scale, name) = if magnitude >= 1.0 {
            (1.0, "s")
        } else if magnitude >= 1e-3 {
            (1e3, "ms")
        } else if magnitude >= 1e-6 {
            (1e6, "\u{b5}s")
        } else {
            (1e9, "ns")
        };
        return format!("{} {name}", significant(value * scale, DURATION_DIGITS));
    }
    format!("{} {unit}", significant(value, VALUE_DIGITS))
}

#[cfg(test)]
mod tests {
    use super::measured_text;

    /// Trace: PLAT-1050
    #[test]
    fn tc_1050_001_durations_scale_to_a_readable_unit_with_four_significant_digits() {
        assert_eq!(
            measured_text(Some(0.005_972_900_000_000_001), "s"),
            "5.973 ms"
        );
        assert_eq!(measured_text(Some(4.9953e-7), "s"), "499.5 ns");
        assert_eq!(measured_text(Some(2.5e-5), "s"), "25 \u{b5}s");
        assert_eq!(measured_text(Some(12.0), "s"), "12 s");
    }

    /// Trace: PLAT-1050
    #[test]
    fn tc_1050_002_other_values_drop_float_noise_and_keep_their_unit() {
        assert_eq!(measured_text(Some(0.1 + 0.2), "ratio"), "0.3 ratio");
        assert_eq!(measured_text(Some(0.5), "ratio"), "0.5 ratio");
        assert_eq!(measured_text(Some(1e21), "ratio"), "1e+21 ratio");
        assert_eq!(measured_text(None, "ratio"), "null ratio");
    }
}
