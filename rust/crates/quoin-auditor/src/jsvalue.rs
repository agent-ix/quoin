// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `String(value)` and `Number(value)`, as ECMAScript defines them.
//!
//! `src/method-catalog.ts` coerces eight manifest fields with `String(...)`,
//! and a manifest is YAML written by a module author: `name: 1.0` is a number,
//! `name: true` is a boolean, and `tooling: [1, 2]` is a list of numbers. Rust
//! would refuse those at the type; the retained reader accepts them and writes
//! the coercion into the catalog, so the port has to do the same or it reads a
//! module the TypeScript reads and produces a different catalog.
//!
//! Number formatting is [`quoin_store::json::number::format_number`] and
//! nothing else — the ECMAScript `Number::toString` algorithm, which is not
//! Rust's `{}`.

use quoin_store::json::number::format_number;
use serde_json::Value;

/// `String(value)`, for the values a YAML 1.2 document can hold.
///
/// `null` is included and yields `"null"`. Callers that mean `value ?? other`
/// must apply the coalescing first: `??` tests for `null`/`undefined` and this
/// does not.
#[must_use]
pub fn string_of(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(true) => "true".to_owned(),
        Value::Bool(false) => "false".to_owned(),
        Value::Number(number) => number
            .as_f64()
            .map_or_else(|| number.to_string(), format_number),
        Value::String(text) => text.clone(),
        // `Array.prototype.toString` joins with `,` and renders `null` and
        // `undefined` members as the empty string.
        Value::Array(items) => {
            let parts: Vec<String> = items
                .iter()
                .map(|item| {
                    if item.is_null() {
                        String::new()
                    } else {
                        string_of(item)
                    }
                })
                .collect();
            parts.join(",")
        }
        // `Object.prototype.toString`, which is what a plain object stringifies
        // to and the reason a mistyped manifest field reads as this exact text
        // in a catalog rather than failing.
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

/// `value ?? fallback` followed by `String(...)`.
///
/// YAML has no `undefined`; a missing key and an explicit `null` both arrive
/// here as [`None`] or [`Value::Null`], which is what `??` treats alike.
#[must_use]
pub fn string_or(value: Option<&Value>, fallback: &str) -> String {
    match value {
        None | Some(Value::Null) => fallback.to_owned(),
        Some(present) => string_of(present),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use serde_json::json;

    use super::{string_of, string_or};

    #[test]
    fn coerces_the_way_the_retained_reader_does() {
        assert_eq!(string_of(&json!("already")), "already");
        assert_eq!(string_of(&json!(true)), "true");
        assert_eq!(string_of(&json!(null)), "null");
        assert_eq!(string_of(&json!({"a": 1})), "[object Object]");
        assert_eq!(string_of(&json!(["a", "b"])), "a,b");
        assert_eq!(string_of(&json!(["a", null, "b"])), "a,,b");
    }

    #[test]
    fn a_number_is_formatted_by_ecmascript_and_not_by_rust() {
        // `{}` on an f64 prints `1` for 1.0 too, but diverges at the
        // exponential boundaries — which is why this goes through the one
        // formatter in the workspace rather than `to_string`.
        assert_eq!(string_of(&json!(1.0)), "1");
        assert_eq!(string_of(&json!(1e21)), "1e+21");
        assert_eq!(string_of(&json!(0.1)), "0.1");
    }

    #[test]
    fn nullish_coalescing_happens_before_the_coercion() {
        assert_eq!(string_or(None, "fallback"), "fallback");
        assert_eq!(string_or(Some(&json!(null)), "fallback"), "fallback");
        assert_eq!(string_or(Some(&json!(false)), "fallback"), "false");
    }
}
