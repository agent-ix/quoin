// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The JSON scalars a measured effect carries, and their rendered spelling.
//!
//! `intervention-report.ts:76-80` renders a measured effect through
//! `String(value)`. That is not `{value}` — `String(1e21)` in JavaScript is
//! `"1e+21"` and `String(1e-7)` is `"1e-7"`, while Rust's `Display` for `f64`
//! never uses exponential notation and writes `1000000000000000000000` and
//! `0.0000001` instead. The committed oracle capture holds both spellings
//! (`tests/fixtures/report-oracle.json`, case `single_full`), so this is a
//! measured difference and not a hypothetical one.
//!
//! [`js_number_string`] implements ECMA-262 `Number::toString` radix 10, which
//! is the function `String(number)` reduces to. It is the only number
//! formatting in this crate.

use core::fmt;

use serde::{Deserialize, Serialize};

/// The value a measured effect records for an arm.
///
/// `intervention-types.ts:57,58` — `number | string | boolean | null`. Modelled
/// as a closed enum rather than `serde_json::Value` because the TypeScript
/// closes it: an array here is not a value this port should accept.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScalarValue {
    /// JSON `null`.
    Null,
    /// A JSON boolean.
    Bool(bool),
    /// A JSON number, held as written so a round trip returns the same bytes.
    Number(serde_json::Number),
    /// A JSON string.
    String(String),
}

impl fmt::Display for ScalarValue {
    /// The JavaScript `String(value)` spelling.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => formatter.write_str("null"),
            Self::Bool(true) => formatter.write_str("true"),
            Self::Bool(false) => formatter.write_str("false"),
            Self::Number(number) => formatter.write_str(&js_number_string(number)),
            Self::String(text) => formatter.write_str(text),
        }
    }
}

/// The effect a treatment had, when one could be stated.
///
/// `intervention-types.ts:59` — `number | string | null`. A boolean is not an
/// effect, and the narrower type is the one the TypeScript declares.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EffectValue {
    /// JSON `null` — no effect could be stated.
    Null,
    /// A JSON number.
    Number(serde_json::Number),
    /// A JSON string.
    String(String),
}

impl fmt::Display for EffectValue {
    /// The JavaScript `String(value)` spelling.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => formatter.write_str("null"),
            Self::Number(number) => formatter.write_str(&js_number_string(number)),
            Self::String(text) => formatter.write_str(text),
        }
    }
}

/// An environment entry's value.
///
/// `intervention-types.ts:27` and `operational-types.ts:34` —
/// `Record<string, string | number | boolean | null>`, declared twice and
/// identically.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EnvironmentValue {
    /// JSON `null`.
    Null,
    /// A JSON boolean.
    Bool(bool),
    /// A JSON number.
    Number(serde_json::Number),
    /// A JSON string.
    String(String),
}

/// The JavaScript `String(value)` spelling of a JSON number.
///
/// Every JSON number reaching a JavaScript program is an IEEE-754 double —
/// `JSON.parse` has no other numeric type — so this converts through `f64`
/// first. That is faithful rather than lossy: an integer literal too large for a
/// double is rounded by `JSON.parse` before `String` ever sees it, and this
/// rounds it the same way.
///
/// Implements ECMA-262 §6.1.6.1.20 `Number::toString` with radix 10. The
/// shortest round-tripping digit string and its exponent come from Rust's own
/// `{:e}`, which uses the same shortest-representation algorithm ECMA-262
/// specifies; only the placement rules below are written out.
#[must_use]
pub fn js_number_string(number: &serde_json::Number) -> String {
    number.as_f64().map_or_else(
        // `as_f64` is total for every `Number` serde_json can build without the
        // `arbitrary_precision` feature, which this crate does not enable. The
        // fallback is the stored spelling rather than a panic, because a panic
        // in a renderer is the failure the workspace panic lints exist to stop.
        || number.to_string(),
        js_f64_string,
    )
}

/// [`js_number_string`] for a value already reduced to a double.
#[must_use]
fn js_f64_string(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    // JSON carries neither, but a renderer that is total at the type level is
    // one fewer branch a caller has to reason about.
    if value.is_infinite() {
        return if value < 0.0 { "-Infinity" } else { "Infinity" }.to_owned();
    }
    if value == 0.0 {
        // Covers -0.0: ECMA-262 step 2 spells both zeros "0".
        return "0".to_owned();
    }
    if value < 0.0 {
        return format!("-{}", js_f64_string(-value));
    }

    // `{:e}` yields the shortest round-tripping mantissa and a base-10 exponent:
    // `1e21`, `9.55e1`, `1e-7`.
    let exponential = format!("{value:e}");
    let Some((mantissa, exponent)) = exponential.split_once('e') else {
        return exponential;
    };
    let Ok(exponent) = exponent.parse::<i32>() else {
        return exponential;
    };
    let digits: String = mantissa
        .chars()
        .filter(|character| *character != '.')
        .collect();
    let Ok(digit_count) = i32::try_from(digits.len()) else {
        return exponential;
    };
    // ECMA-262 names these `k` (digit count) and `n` (the decimal point's
    // position relative to the digit string).
    let point = exponent + 1;

    if digit_count <= point && point <= 21 {
        let zeros = usize::try_from(point - digit_count).unwrap_or(0);
        return format!("{digits}{}", "0".repeat(zeros));
    }
    if 0 < point && point <= 21 {
        let split = usize::try_from(point).unwrap_or(0);
        let (whole, fraction) = digits.split_at(split.min(digits.len()));
        return format!("{whole}.{fraction}");
    }
    if -6 < point && point <= 0 {
        let zeros = usize::try_from(-point).unwrap_or(0);
        return format!("0.{}{digits}", "0".repeat(zeros));
    }
    let sign = if point >= 1 { '+' } else { '-' };
    let magnitude = (point - 1).abs();
    if digit_count == 1 {
        return format!("{digits}e{sign}{magnitude}");
    }
    let (first, rest) = digits.split_at(1);
    format!("{first}.{rest}e{sign}{magnitude}")
}

/// JavaScript `String(value)` for a value that may be absent.
///
/// The whole of `String(x)`, not a case of it: two ports need it — the
/// semantic pass interpolates `item[field]` into a set key without checking
/// its type (`intervention.ts:236`), and the agent-eval reader matches
/// `String(result.passRate ?? "")` against a grammar
/// (`agent-eval-intervention.ts:167`). Writing it twice would be two chances
/// to spell a number differently from the renderer, so it lives beside
/// [`js_number_string`], which is the only number formatting in this crate.
pub(crate) fn js_string(value: Option<&serde_json::Value>) -> String {
    use serde_json::Value;
    match value {
        None => "undefined".to_owned(),
        Some(Value::Null) => "null".to_owned(),
        Some(Value::Bool(true)) => "true".to_owned(),
        Some(Value::Bool(false)) => "false".to_owned(),
        Some(Value::Number(number)) => js_number_string(number),
        Some(Value::String(text)) => text.clone(),
        // `Array.prototype.toString` joins with `,` and renders `null` and
        // `undefined` as the empty string; every object is `[object Object]`.
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                if item.is_null() {
                    String::new()
                } else {
                    js_string(Some(item))
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Some(Value::Object(_)) => "[object Object]".to_owned(),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{EffectValue, ScalarValue, js_f64_string};

    /// Every expectation here is what `String(x)` prints in node, not what this
    /// function prints — the oracle capture carries the same two extremes.
    #[test]
    fn the_ecma_262_placement_rules_are_reproduced() {
        for (value, expected) in [
            (0.0_f64, "0"),
            (-0.0_f64, "0"),
            (1.0, "1"),
            (120.0, "120"),
            (0.5, "0.5"),
            (95.5, "95.5"),
            (-24.5, "-24.5"),
            (1e21, "1e+21"),
            (1e-7, "1e-7"),
            (1e-6, "0.000001"),
            (1e20, "100000000000000000000"),
            (1.5e22, "1.5e+22"),
            (1.234_567_890_123_456_7e19, "12345678901234567000"),
        ] {
            assert_eq!(js_f64_string(value), expected, "String({value:?})");
        }
    }

    #[test]
    fn a_scalar_renders_the_way_javascript_spells_it() {
        assert_eq!(ScalarValue::Null.to_string(), "null");
        assert_eq!(ScalarValue::Bool(true).to_string(), "true");
        assert_eq!(ScalarValue::Bool(false).to_string(), "false");
        assert_eq!(ScalarValue::String("x".to_owned()).to_string(), "x");
        assert_eq!(EffectValue::Null.to_string(), "null");
    }

    #[test]
    fn a_scalar_round_trips_through_json_without_widening_its_number() {
        let json = r#"[null,true,3,3.5,"3"]"#;
        let values: Vec<ScalarValue> = serde_json::from_str(json).unwrap();
        assert_eq!(values[0], ScalarValue::Null);
        assert_eq!(values[1], ScalarValue::Bool(true));
        assert_eq!(values[3].to_string(), "3.5");
        assert_eq!(values[4].to_string(), "3");
        // Compared as values rather than as text: serializing to a string
        // here would be a second JSON writer in this crate, which
        // `tc_468_boundary` forbids (canonical bytes are `quoin_store`'s).
        assert_eq!(
            serde_json::to_value(&values).unwrap(),
            serde_json::from_str::<serde_json::Value>(json).unwrap()
        );
    }
}
