// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `semantic.import_types` shape rules (FR-070).

use serde_json::Value;

use super::json_text;
use crate::diagnostic::{DiagnosticCode, SemanticDiagnostic};

/// The shape rules of `semantic.import_types` (FR-070), judged before the
/// schema so the reason names the package and the offending entry whatever
/// the schema's own message says.
///
/// Every refusal is `semantic.invalid-value` at `semantic.import_types.<package>`
/// (`semantic.import_types` for a non-object), the code `quire load` reports for
/// the same shapes.
pub(super) fn shape(block: &serde_json::Map<String, Value>) -> Vec<SemanticDiagnostic> {
    let Some(declared) = block.get("import_types") else {
        return Vec::new();
    };
    let Some(map) = declared.as_object() else {
        return vec![SemanticDiagnostic::error(
            DiagnosticCode::InvalidValue,
            "semantic.import_types",
            "semantic.import_types must be an object mapping a package to its type names",
        )];
    };
    let imports = block.get("imports").and_then(Value::as_object);
    let mut found = Vec::new();
    for (package, value) in map {
        let at = format!("semantic.import_types.{package}");
        let refuse = |reason: String| {
            SemanticDiagnostic::error(DiagnosticCode::InvalidValue, at.clone(), reason)
        };
        if !imports.is_some_and(|imports| imports.contains_key(package)) {
            found.push(refuse(format!(
                "{at}: {package} is not a package in semantic.imports"
            )));
            continue;
        }
        let Some(items) = value.as_array() else {
            found.push(refuse(format!(
                "{at}: the type list for {package} must be an array"
            )));
            continue;
        };
        if items.is_empty() {
            found.push(refuse(format!(
                "{at}: the type list for {package} must not be empty"
            )));
        }
        let mut seen = std::collections::BTreeSet::new();
        for item in items {
            match item.as_str() {
                None => found.push(refuse(format!(
                    "{at}: entry {} for {package} is not a string",
                    json_text(item)
                ))),
                Some("") => found.push(refuse(format!("{at}: {package} lists an empty type name"))),
                Some(name) if !seen.insert(name) => found.push(refuse(format!(
                    "{at}: {package} lists type {name} more than once"
                ))),
                Some(_) => {}
            }
        }
    }
    found
}
