// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The typed readers the retained `parseAssuranceArgument` reads through.
//!
//! One function per retained helper (`record`, `stringAt`, `arrayAt`, …),
//! each returning [`Checked`] so a refusal carries the retained
//! implementation's own message (quoin#384).

use super::instant::js_trim_is_empty;
use super::types::{Checked, reject};

/// `record(name, value)`: an object, and not an array.
pub(crate) fn record<'a>(
    name: &str,
    value: Option<&'a serde_json::Value>,
) -> Checked<&'a serde_json::Map<String, serde_json::Value>> {
    match value.and_then(serde_json::Value::as_object) {
        Some(object) => Ok(object),
        None => reject(format!("{name} must be an object")),
    }
}

/// `arrayAt(object, key)`.
pub(super) fn array_at<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Checked<&'a Vec<serde_json::Value>> {
    match object.get(key).and_then(serde_json::Value::as_array) {
        Some(array) => Ok(array),
        None => reject(format!("{key} must be an array")),
    }
}

/// `stringAt(object, key)`: a string, non-empty once trimmed.
pub(crate) fn string_at(
    object: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Checked<String> {
    let Some(value) = object.get(key).and_then(serde_json::Value::as_str) else {
        return reject(format!("{key} must be a string"));
    };
    if js_trim_is_empty(value) {
        return reject(format!("{key} must not be empty"));
    }
    Ok(value.to_owned())
}

/// `optionalStringAt(object, key)`.
///
/// The distinction that matters is `key in object`, not `value != null`: an
/// explicit `null` reaches [`string_at`] and is refused. See [`Challenge`].
pub(crate) fn optional_string_at(
    object: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Checked<Option<String>> {
    if object.contains_key(key) {
        string_at(object, key).map(Some)
    } else {
        Ok(None)
    }
}

/// `exactKeys(object, name, keys)`: no unknown key, and none missing.
pub(crate) fn exact_keys(
    object: &serde_json::Map<String, serde_json::Value>,
    name: &str,
    keys: &[&str],
    optional: &[&str],
) -> Checked<()> {
    for key in object.keys() {
        if !keys.contains(&key.as_str()) {
            return reject(format!("{name} has unknown field {key}"));
        }
    }
    for key in keys {
        if !optional.contains(key) && !object.contains_key(*key) {
            return reject(format!("{name} is missing {key}"));
        }
    }
    Ok(())
}

/// `stringArray(name, value, requireValue)`: strings, unique, optionally non-empty.
pub(crate) fn string_array(
    name: &str,
    value: Option<&serde_json::Value>,
    require_value: bool,
) -> Checked<Vec<String>> {
    let Some(array) = value.and_then(serde_json::Value::as_array) else {
        return reject(format!(
            "{name} must be {} array",
            if require_value { "a non-empty" } else { "an" }
        ));
    };
    if require_value && array.is_empty() {
        return reject(format!("{name} must be a non-empty array"));
    }
    let mut strings = Vec::with_capacity(array.len());
    for item in array {
        let Some(text) = item.as_str() else {
            return reject(format!("{name} must contain strings"));
        };
        if js_trim_is_empty(text) {
            return reject(format!("{name} must not be empty"));
        }
        strings.push(text.to_owned());
    }
    let mut seen: Vec<&String> = Vec::with_capacity(strings.len());
    for text in &strings {
        if seen.contains(&text) {
            return reject(format!("{name} must contain unique values"));
        }
        seen.push(text);
    }
    Ok(strings)
}

/// `literal(value, name, allowed)`: membership, checked at run time.
pub(crate) fn literal<T: Copy>(
    value: Option<&serde_json::Value>,
    name: &str,
    allowed: &[(&str, T)],
) -> Checked<T> {
    if let Some(text) = value.and_then(serde_json::Value::as_str) {
        for (candidate, mapped) in allowed {
            if *candidate == text {
                return Ok(*mapped);
            }
        }
    }
    let names: Vec<&str> = allowed.iter().map(|(name, _)| *name).collect();
    reject(format!("{name} must be one of {}", names.join(", ")))
}

/// `uniqueIds(name, values)`.
pub(super) fn unique_ids(name: &str, ids: &[&str]) -> Checked<()> {
    let mut seen: Vec<&str> = Vec::with_capacity(ids.len());
    for id in ids {
        if seen.contains(id) {
            return reject(format!("{name} contains duplicate id {id}"));
        }
        seen.push(id);
    }
    Ok(())
}

/// `/^AA-[0-9]+$/`, without a regex engine.
pub(super) fn is_argument_id(id: &str) -> bool {
    let Some(digits) = id.strip_prefix("AA-") else {
        return false;
    };
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}
