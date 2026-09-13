// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The one YAML reader on the quoin boundary.
//!
//! Every YAML document quoin reads — a module `manifest.yaml`, a bundle
//! document's frontmatter — is read through [`from_str`], which resolves
//! scalars under the **YAML 1.2 core schema** and yields a
//! [`serde_json::Value`].
//!
//! # Why this crate exists
//!
//! The TypeScript side reads YAML with the `yaml` npm package, which
//! implements the YAML 1.2 core schema. Every libyaml binding available to
//! Rust (`serde_yaml`, `serde_norway`, `serde_yml`) implements YAML 1.1
//! implicit typing instead. The two schemas resolve a plain scalar
//! differently, and the difference reaches the verdict:
//!
//! | scalar  | YAML 1.2 core (here, and the oracle) | YAML 1.1 (libyaml) |
//! |---------|--------------------------------------|--------------------|
//! | `017`   | integer `17`                         | integer `15` (octal) |
//! | `010`   | integer `10`                         | integer `8` (octal) |
//! | `0b101` | string `"0b101"`                     | integer `5`        |
//!
//! Frontmatter readers drop non-string members from a vocabulary field, so a
//! scalar that resolves to a number on one side and a string on the other is
//! kept as a claim on one side and dropped on the other. `quoin#378`
//! reproduced that as a `PASS`/`FAIL` split on the same bundle. Reading
//! through one YAML 1.2 reader is what closes it.
//!
//! Do not add a second YAML reader to this workspace. Depend on this crate.

use std::borrow::Cow;

use saphyr::{LoadableYamlNode, Scalar, Yaml};
use serde_json::{Map, Value};

/// A YAML document could not be read.
#[derive(Debug, thiserror::Error)]
pub enum YamlError {
    /// The input is not well-formed YAML.
    #[error("{0}")]
    Scan(#[from] saphyr::ScanError),
    /// The input holds more than one YAML document.
    ///
    /// Matches the oracle: the `yaml` package's `parse()` rejects a
    /// multi-document stream rather than silently taking the first document.
    #[error("expected a single YAML document, found {found}")]
    MultipleDocuments {
        /// How many documents the stream actually held.
        found: usize,
    },
    /// A YAML feature with no JSON counterpart appeared.
    #[error("{0}")]
    Unrepresentable(String),
}

/// Read a single YAML document as a [`serde_json::Value`].
///
/// An empty input yields [`Value::Null`], as an empty YAML stream has no
/// document.
///
/// # Errors
///
/// Returns [`YamlError::Scan`] when the input is not well-formed YAML,
/// [`YamlError::MultipleDocuments`] when it holds more than one document, and
/// [`YamlError::Unrepresentable`] for a YAML construct JSON cannot hold (an
/// unresolved alias, or a sequence or mapping used as a mapping key).
pub fn from_str(yaml: &str) -> Result<Value, YamlError> {
    let mut documents = Yaml::load_from_str(yaml)?;
    if documents.len() > 1 {
        return Err(YamlError::MultipleDocuments {
            found: documents.len(),
        });
    }
    match documents.pop() {
        None => Ok(Value::Null),
        Some(document) => convert(&document),
    }
}

fn convert(node: &Yaml<'_>) -> Result<Value, YamlError> {
    match node {
        Yaml::Value(scalar) => Ok(scalar_to_value(scalar)),
        Yaml::Representation(text, _, _) => Ok(scalar_to_value(&Scalar::String(text.clone()))),
        Yaml::Sequence(items) => items
            .iter()
            .map(convert)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Yaml::Mapping(entries) => {
            let mut map = Map::new();
            for (key, value) in entries {
                map.insert(mapping_key(key)?, convert(value)?);
            }
            Ok(Value::Object(map))
        }
        Yaml::Tagged(_, inner) => convert(inner),
        Yaml::Alias(_) => Err(YamlError::Unrepresentable(
            "unresolved YAML alias".to_owned(),
        )),
        Yaml::BadValue => Err(YamlError::Unrepresentable(
            "a scalar whose contents could not be resolved".to_owned(),
        )),
    }
}

/// A JSON object key from a YAML mapping key.
///
/// JSON keys are strings and so are JavaScript object keys, so a non-string
/// scalar key becomes its own source spelling on both sides. A composite key
/// has no JSON counterpart at all and is refused rather than flattened.
fn mapping_key(key: &Yaml<'_>) -> Result<String, YamlError> {
    match key {
        Yaml::Value(Scalar::String(text)) | Yaml::Representation(text, _, _) => {
            Ok(text.clone().into_owned())
        }
        Yaml::Value(Scalar::Null) => Ok("null".to_owned()),
        Yaml::Value(Scalar::Boolean(flag)) => Ok(flag.to_string()),
        Yaml::Value(Scalar::Integer(number)) => Ok(number.to_string()),
        Yaml::Value(Scalar::FloatingPoint(number)) => Ok(number.into_inner().to_string()),
        Yaml::Tagged(_, inner) => mapping_key(inner),
        Yaml::Sequence(_) | Yaml::Mapping(_) => Err(YamlError::Unrepresentable(
            "a sequence or mapping used as a mapping key".to_owned(),
        )),
        Yaml::Alias(_) => Err(YamlError::Unrepresentable(
            "unresolved YAML alias used as a mapping key".to_owned(),
        )),
        Yaml::BadValue => Err(YamlError::Unrepresentable(
            "an unresolvable scalar used as a mapping key".to_owned(),
        )),
    }
}

/// A resolved YAML scalar as JSON.
///
/// `.inf` and `.nan` have no JSON form. They become `null`, which is what the
/// oracle's own `JSON.stringify` writes for JavaScript's `Infinity` and `NaN`,
/// so the two sides still agree.
fn scalar_to_value(scalar: &Scalar<'_>) -> Value {
    match scalar {
        Scalar::Null => Value::Null,
        Scalar::Boolean(flag) => Value::Bool(*flag),
        Scalar::Integer(number) => Value::Number((*number).into()),
        Scalar::FloatingPoint(number) => {
            serde_json::Number::from_f64(number.into_inner()).map_or(Value::Null, Value::Number)
        }
        Scalar::String(text) => Value::String(Cow::clone(text).into_owned()),
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{YamlError, from_str};
    use serde_json::json;

    /// Trace: FR-037
    /// Provenance: agent-ix/quoin#378
    #[test]
    fn tc_378_310_ambiguous_scalars_resolve_under_the_yaml_1_2_core_schema() {
        // Each expectation here was measured against the `yaml` npm package
        // the TypeScript oracle uses; see DIVERGENCE.md section 6 (D4).
        let read = from_str("k: [017, 010, 0b101, 0o17, 0x1F, on, no, y, null, 1:30, 2026-09-12]")
            .expect("well-formed");
        assert_eq!(
            read,
            json!({"k": [17, 10, "0b101", 15, 31, "on", "no", "y", null, "1:30", "2026-09-12"]})
        );
    }

    /// Trace: FR-037
    /// Provenance: agent-ix/quoin#378
    #[test]
    fn tc_378_311_an_empty_stream_is_null_and_a_multi_document_stream_is_refused() {
        assert_eq!(from_str("").expect("empty"), serde_json::Value::Null);
        let Err(YamlError::MultipleDocuments { found }) = from_str("---\na: 1\n---\nb: 2\n") else {
            panic!("a two-document stream must be refused");
        };
        assert_eq!(found, 2);
    }

    /// Trace: FR-037
    /// Provenance: agent-ix/quoin#378
    #[test]
    fn tc_378_312_malformed_yaml_is_an_error_and_aliases_resolve() {
        assert!(matches!(from_str("id: [unclosed"), Err(YamlError::Scan(_))));
        assert_eq!(
            from_str("a: &x 1\nb: *x\n").expect("aliases resolve"),
            json!({"a": 1, "b": 1})
        );
    }

    /// Trace: FR-037
    /// Provenance: agent-ix/quoin#378
    #[test]
    fn tc_378_313_non_string_mapping_keys_take_their_source_spelling() {
        assert_eq!(
            from_str("1: v\ntrue: w\nnull: x\n").expect("well-formed"),
            json!({"1": "v", "true": "w", "null": "x"})
        );
    }
}
