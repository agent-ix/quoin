// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The one sanctioned way to carry a JSON-shaped value into YAML.
//!
//! A [`serde_json::Value`] survives a **non-serde_json serializer** only while
//! `serde_json/arbitrary_precision` is off. With it on, every number leaves the
//! serde data model as the private marker map
//! `{"$serde_json::private::Number": "7"}`, and that is what `serde_yaml_ng`
//! would write to the user's config file. The feature is global and unifying,
//! so one dependency edge anywhere in the workspace enabling it silently
//! corrupts the value (quoin#442).
//!
//! The bridge here goes through **JSON text**: `serde_json`'s own serializer
//! renders each number token correctly, and YAML 1.2 is a superset of JSON, so
//! the text round-trip is faithful and immune to that feature. `clippy.toml`
//! forbids `serde_yaml_ng::to_value`, `to_string` and `to_writer` so this
//! helper is the path of least resistance (PLAT-97).

use serde::Serialize;
use serde_yaml_ng::Value as Yaml;

/// The JSON-text bridge failed.
#[derive(Debug, thiserror::Error)]
pub enum JsonBridgeError {
    /// The value could not be rendered as JSON text.
    #[error("rendering as JSON text: {0}")]
    Render(serde_json::Error),
    /// The JSON text was not read back as YAML.
    #[error("reading the JSON text as YAML: {0}")]
    Read(serde_yaml_ng::Error),
}

/// Convert any serializable value — a [`serde_json::Value`], or a typed config
/// that may contain one — into a YAML value through JSON text.
///
/// # Errors
/// [`JsonBridgeError::Render`] when `value` has no JSON form (for example a map
/// with non-string keys), [`JsonBridgeError::Read`] if the rendered text is not
/// read back as YAML.
pub fn to_yaml_via_json_text<T: Serialize + ?Sized>(value: &T) -> Result<Yaml, JsonBridgeError> {
    let text = serde_json::to_string(value).map_err(JsonBridgeError::Render)?;
    serde_yaml_ng::from_str(&text).map_err(JsonBridgeError::Read)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    #[trace("FR-027-AC-1")]
    #[test]
    fn tc_381_132_a_json_value_reaches_yaml_with_its_numbers_intact() {
        let value: serde_json::Value =
            serde_json::from_str(r#"{"retries":7,"deep":[1,{"n":2.5}],"s":"x"}"#)
                .expect("valid JSON");
        let yaml = to_yaml_via_json_text(&value).expect("bridges");
        let Yaml::Mapping(map) = &yaml else {
            panic!("expected a mapping, got {yaml:?}");
        };
        assert_eq!(map.get("retries").and_then(Yaml::as_u64), Some(7));
        assert_eq!(map.get("s").and_then(Yaml::as_str), Some("x"));
        let nested = map
            .get("deep")
            .and_then(Yaml::as_sequence)
            .and_then(|items| items.get(1))
            .and_then(Yaml::as_mapping)
            .and_then(|inner| inner.get("n"))
            .and_then(Yaml::as_f64);
        assert_eq!(nested, Some(2.5), "a nested number left as a marker map");
    }

    #[trace("FR-027-AC-1")]
    #[test]
    fn tc_381_133_a_value_with_no_json_form_is_a_render_error() {
        let mut map = std::collections::BTreeMap::new();
        map.insert((1_u8, 2_u8), 3_u8);
        assert!(matches!(
            to_yaml_via_json_text(&map),
            Err(JsonBridgeError::Render(_))
        ));
    }
}
