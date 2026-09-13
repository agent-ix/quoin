// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Reading the three declared inputs, and writing the one canonical form.
//!
//! # Reading
//!
//! The retained contracts are zod schemas (`input.ts:13-80`, `load.ts:318-342`)
//! and they are **not** re-expressed as `serde` attributes here. Two of them
//! are `.strict()` and three are `.passthrough()`, mixed inside one document;
//! `#[serde(deny_unknown_fields)]` and `#[serde(flatten)]` cannot both apply to
//! one struct, and the combination silently accepts unknown members where the
//! retained schema refuses them (`reference_serde_unit_variant_deny_unknown_fields`).
//! So the contracts are read by the small combinators below, which say `strict`
//! or do not, once, per object.
//!
//! `serde_json` does the tokenising rather than
//! `quoin_store::parse_strict_json_str`, because the oracle is `JSON.parse`:
//! it accepts duplicate members (last wins) and unpaired surrogates, and the
//! strict I-JSON reader refuses both. A reader stricter than the one it
//! replaces refuses inputs that already exist.
//!
//! # Writing
//!
//! There is one canonical writer and it is not here.
//! `renderGraphAnalysisJson` is `canonicalJson` (`render.ts:12`), so
//! [`canonical_text`] hands a report to `quoin_store::canonical_json_bytes` by the
//! route `quoin-evidence`'s codec uses: `serde_json` writes the struct's own
//! member order, the text is re-read, and `quoin-store` orders the members and
//! formats the numbers. FR-100-CON-4 forbids a second canonicalization, and
//! `tests/tc_385_one_canonical_json.rs` asserts there is not one.

use serde::Serialize;
use serde_json::{Map, Value};

use crate::error::{GraphError, Result};

/// One report's canonical JSON text, trailing newline included.
///
/// # Errors
///
/// [`GraphError::Canonicalization`] when the value has no canonical spelling.
pub(crate) fn canonical_text<T: Serialize>(what: &'static str, value: &T) -> Result<String> {
    let text = serde_json::to_string(value).map_err(|error| GraphError::Canonicalization {
        what,
        detail: error.to_string(),
    })?;
    let parsed = quoin_store::parse_strict_json_str(&text).map_err(|error| {
        GraphError::Canonicalization {
            what,
            detail: error.to_string(),
        }
    })?;
    // `canonical_json_bytes` and not its `String` sibling: the acceptance
    // criterion names the byte-producing entry point, and these are the bytes
    // a consumer compares. The decode cannot fail — the store built the bytes
    // from a `String` — but it is not asserted away, because a refusal is
    // cheaper than an assumption about another crate's internals.
    let bytes = quoin_store::canonical_json_bytes(&parsed).map_err(|error| {
        GraphError::Canonicalization {
            what,
            detail: error.to_string(),
        }
    })?;
    String::from_utf8(bytes).map_err(|error| GraphError::Canonicalization {
        what,
        detail: error.to_string(),
    })
}

/// The sort key `stableKey` (`input.ts:274`) computes, for one JSON value.
///
/// `JSON.stringify(canonical(value))` is RFC 8785 JCS for every document this
/// crate sorts: same member order, same escaping, same number formatting. So
/// this calls `quoin_store::canonicalize_jcs` rather than growing a third
/// serializer. The one input shape where the two orders part company — a
/// member name that is an ECMAScript array index — is a declared divergence
/// with a corpus case behind it; see `tests/tc_385_parity.rs`.
///
/// # Errors
///
/// [`GraphError::Canonicalization`], as [`canonical_text`].
pub(crate) fn stable_key(value: &Value) -> Result<String> {
    const WHAT: &str = "an audit report entry";
    let text = serde_json::to_string(value).map_err(|error| GraphError::Canonicalization {
        what: WHAT,
        detail: error.to_string(),
    })?;
    let parsed = quoin_store::parse_strict_json_str(&text).map_err(|error| {
        GraphError::Canonicalization {
            what: WHAT,
            detail: error.to_string(),
        }
    })?;
    quoin_store::canonicalize_jcs(&parsed).map_err(|error| GraphError::Canonicalization {
        what: WHAT,
        detail: error.to_string(),
    })
}

/// A contract reader's refusal: which member, and what was wrong with it.
pub(crate) type Refusal = std::result::Result<(), String>;

/// The document, as `JSON.parse` would read it.
///
/// # Errors
///
/// The parser's sentence, prefixed the way the retained code prefixes it.
pub(crate) fn document(text: &str) -> std::result::Result<Value, String> {
    serde_json::from_str(text).map_err(|error| format!("invalid JSON: {error}"))
}

/// `value` as an object, or why it is not one.
///
/// # Errors
///
/// The member path and the type that was found instead.
pub(crate) fn object<'a>(
    value: &'a Value,
    at: &str,
) -> std::result::Result<&'a Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{at}: expected an object, found {}", kind(value)))
}

/// `value` as a string, or why it is not one.
///
/// # Errors
///
/// The member path and the type that was found instead.
pub(crate) fn text<'a>(value: &'a Value, at: &str) -> std::result::Result<&'a str, String> {
    value
        .as_str()
        .ok_or_else(|| format!("{at}: expected a string, found {}", kind(value)))
}

/// `value` as an array, or why it is not one.
///
/// # Errors
///
/// The member path and the type that was found instead.
pub(crate) fn array<'a>(value: &'a Value, at: &str) -> std::result::Result<&'a Vec<Value>, String> {
    value
        .as_array()
        .ok_or_else(|| format!("{at}: expected an array, found {}", kind(value)))
}

/// One required member.
///
/// # Errors
///
/// The member path, when it is absent.
pub(crate) fn member<'a>(
    object: &'a Map<String, Value>,
    at: &str,
    name: &str,
) -> std::result::Result<&'a Value, String> {
    object
        .get(name)
        .ok_or_else(|| format!("{at}{name}: required member is missing"))
}

/// Every member of a `.strict()` object is one of `allowed`.
///
/// # Errors
///
/// The first undeclared member, in the object's own order.
pub(crate) fn strict(object: &Map<String, Value>, at: &str, allowed: &[&str]) -> Refusal {
    for name in object.keys() {
        if !allowed.contains(&name.as_str()) {
            return Err(format!("{at}{name}: unrecognized member"));
        }
    }
    Ok(())
}

/// One member whose only accepted value is `expected`.
///
/// # Errors
///
/// The member path and what was found instead.
pub(crate) fn literal_text(value: &Value, at: &str, expected: &str) -> Refusal {
    if value.as_str() == Some(expected) {
        Ok(())
    } else {
        Err(format!("{at}: expected the literal \"{expected}\""))
    }
}

/// One member whose only accepted value is the integer `expected`.
///
/// # Errors
///
/// The member path and what was found instead.
pub(crate) fn literal_number(value: &Value, at: &str, expected: u64) -> Refusal {
    if value.as_u64() == Some(expected) {
        Ok(())
    } else {
        Err(format!("{at}: expected the literal {expected}"))
    }
}

/// The JSON type name, for a diagnostic.
fn kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

/// A member path prefix: `""` at the root, `"modules.0."` inside.
#[must_use]
pub(crate) fn at(prefix: &str, segment: &str) -> String {
    format!("{prefix}{segment}.")
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{canonical_text, document, stable_key, strict};

    /// The writer is `quoin-store`'s, so it produces `quoin-store`'s shape:
    /// two-space indent, `": "` separator, sorted members, trailing newline.
    ///
    /// Provenance: quoin#385
    #[test]
    fn the_canonical_text_is_the_stores_and_not_a_local_one() {
        let value = serde_json::json!({ "b": 1, "a": [2, 3] });
        assert_eq!(
            canonical_text("a test value", &value).unwrap(),
            "{\n  \"a\": [\n    2,\n    3\n  ],\n  \"b\": 1\n}\n"
        );
    }

    /// Provenance: quoin#385
    #[test]
    fn the_stable_key_is_compact_jcs() {
        let value = serde_json::json!({ "b": 1, "a": "x" });
        assert_eq!(stable_key(&value).unwrap(), "{\"a\":\"x\",\"b\":1}");
    }

    /// `JSON.parse` keeps the last of two same-named members. So does this.
    ///
    /// Provenance: quoin#385
    #[test]
    fn a_duplicated_member_keeps_the_last_one_as_json_parse_does() {
        let parsed = document("{\"a\":1,\"a\":2}").expect("JSON.parse accepts this");
        assert_eq!(parsed["a"], serde_json::json!(2));
    }

    /// Provenance: quoin#385
    #[test]
    fn a_strict_object_names_the_member_it_did_not_declare() {
        let value = serde_json::json!({ "format": "x", "extra": 1 });
        let refusal = strict(value.as_object().unwrap(), "", &["format"]).expect_err("refused");
        assert_eq!(refusal, "extra: unrecognized member");
    }
}
