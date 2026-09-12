// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! JSON Schema → TypeScript, for the subset `schemars` emits from the boundary
//! types (FR-097).
//!
//! Deliberately a *small, total* renderer rather than a general one. Every
//! construct it does not understand is a named refusal carrying the JSON
//! Pointer of the node that produced it — never `any`, never `unknown`, never
//! a silently dropped field. A generator that degrades to `any` on a shape it
//! did not expect produces a file that compiles, passes its own digest check,
//! and describes nothing; that is the failure this module is written to be
//! incapable of.
//!
//! The subset is exactly what the boundary declares today: objects with named
//! properties, string/integer/boolean/null unions, `Record<string, string>`
//! from a `BTreeMap<String, String>`, arrays, and `$ref` into `$defs`. When a
//! boundary type needs more, this module grows a case and a test in the same
//! commit — which is the point of refusing rather than degrading.

use std::fmt::Write as _;

use serde_json::Value;

/// A construct the renderer does not understand, and where it was found.
///
/// Carries the JSON Pointer rather than a type name because the offending node
/// is frequently a property several levels inside a `$def`, and "unsupported
/// schema" with no location is the error message that costs an afternoon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderRefusal {
    /// JSON Pointer of the node that could not be rendered.
    pub pointer: String,
    /// What about it was not understood.
    pub reason: String,
}

impl std::fmt::Display for RenderRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.pointer, self.reason)
    }
}

impl std::error::Error for RenderRefusal {}

fn refuse(pointer: &str, reason: impl Into<String>) -> RenderRefusal {
    RenderRefusal {
        pointer: pointer.to_owned(),
        reason: reason.into(),
    }
}

/// Render one `$defs` entry as a TypeScript `interface` declaration.
///
/// # Errors
///
/// [`RenderRefusal`] when the definition is not an object schema with named
/// properties, or when any property uses a construct outside the supported
/// subset.
pub fn render_interface(
    name: &str,
    schema: &Value,
    pointer: &str,
) -> Result<String, RenderRefusal> {
    let object = schema
        .as_object()
        .ok_or_else(|| refuse(pointer, "a `$defs` entry must be an object schema"))?;
    if object.get("type").and_then(Value::as_str) != Some("object") {
        return Err(refuse(
            pointer,
            "only `\"type\": \"object\"` definitions are rendered as interfaces",
        ));
    }
    let properties = object
        .get("properties")
        .and_then(Value::as_object)
        .ok_or_else(|| refuse(pointer, "an object definition must declare `properties`"))?;
    let required: Vec<&str> = object
        .get("required")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();

    let mut out = String::new();
    if let Some(description) = object.get("description").and_then(Value::as_str) {
        out.push_str(&doc_comment(description, ""));
    }
    let _ = writeln!(out, "export interface {name} {{");
    for (field, property) in properties {
        let field_pointer = format!("{pointer}/properties/{field}");
        if let Some(description) = property.get("description").and_then(Value::as_str) {
            out.push_str(&doc_comment(description, "  "));
        }
        let optional = if required.contains(&field.as_str()) {
            ""
        } else {
            "?"
        };
        let rendered = render_type(property, &field_pointer)?;
        let _ = writeln!(out, "  {field}{optional}: {rendered};");
    }
    out.push_str("}\n");
    Ok(out)
}

/// Render a schema node as a TypeScript type expression.
///
/// # Errors
///
/// [`RenderRefusal`] for any construct outside the supported subset.
pub fn render_type(schema: &Value, pointer: &str) -> Result<String, RenderRefusal> {
    let object = schema
        .as_object()
        .ok_or_else(|| refuse(pointer, "expected a schema object"))?;

    if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
        let name = reference.strip_prefix("#/$defs/").ok_or_else(|| {
            refuse(
                pointer,
                format!("only local `#/$defs/…` references are supported, got `{reference}`"),
            )
        })?;
        return Ok(name.to_owned());
    }

    let Some(type_node) = object.get("type") else {
        return Err(refuse(
            pointer,
            "a schema node without `type` or `$ref` is not renderable; the \
             renderer refuses rather than emitting `unknown`",
        ));
    };

    match type_node {
        Value::String(single) => render_named_type(single, object, pointer),
        Value::Array(members) => {
            let mut parts = Vec::with_capacity(members.len());
            for member in members {
                let name = member
                    .as_str()
                    .ok_or_else(|| refuse(pointer, "a `type` array must hold strings"))?;
                parts.push(render_named_type(name, object, pointer)?);
            }
            if parts.is_empty() {
                return Err(refuse(pointer, "an empty `type` array names no type"));
            }
            parts.dedup();
            Ok(parts.join(" | "))
        }
        _ => Err(refuse(pointer, "`type` must be a string or an array")),
    }
}

fn render_named_type(
    name: &str,
    object: &serde_json::Map<String, Value>,
    pointer: &str,
) -> Result<String, RenderRefusal> {
    match name {
        "string" => Ok("string".to_owned()),
        "integer" | "number" => Ok("number".to_owned()),
        "boolean" => Ok("boolean".to_owned()),
        "null" => Ok("null".to_owned()),
        "array" => {
            let items = object
                .get("items")
                .ok_or_else(|| refuse(pointer, "an array schema must declare `items`"))?;
            let inner = render_type(items, &format!("{pointer}/items"))?;
            Ok(format!("{inner}[]"))
        }
        "object" => {
            if object.contains_key("properties") {
                return Err(refuse(
                    pointer,
                    "an inline object with `properties` must be lifted into `$defs`; \
                     the renderer does not emit anonymous nested interfaces",
                ));
            }
            let additional = object.get("additionalProperties").ok_or_else(|| {
                refuse(
                    pointer,
                    "an object schema with neither `properties` nor \
                     `additionalProperties` describes nothing",
                )
            })?;
            let value = render_type(additional, &format!("{pointer}/additionalProperties"))?;
            Ok(format!("Record<string, {value}>"))
        }
        other => Err(refuse(pointer, format!("unsupported JSON type `{other}`"))),
    }
}

/// A `JSDoc` block for a schema `description`, indented by `indent`.
///
/// Rust doc comments arrive with hard line breaks and `[`Type`]` intra-doc
/// links. The breaks are preserved (they are the author's paragraphing) and
/// the links are flattened to their text, because a TypeScript reader cannot
/// follow a rustdoc path and a dangling `[`…`]` reads as a broken markdown
/// link in every editor that renders `JSDoc`.
fn doc_comment(description: &str, indent: &str) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{indent}/**");
    for line in description.lines() {
        let line = flatten_intra_doc_links(line);
        let line = line.trim_end();
        if line.is_empty() {
            let _ = writeln!(out, "{indent} *");
        } else {
            let _ = writeln!(out, "{indent} * {line}");
        }
    }
    let _ = writeln!(out, "{indent} */");
    out
}

fn flatten_intra_doc_links(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(open) = rest.find("[`") {
        let Some(close_offset) = rest[open..].find("`]") else {
            break;
        };
        let close = open + close_offset;
        out.push_str(&rest[..open]);
        out.push('`');
        // The rustdoc path `crate::protocol::Outcome::Partial` reads as noise
        // in TypeScript; only the final segment carries meaning there.
        let path = &rest[open + 2..close];
        out.push_str(path.rsplit("::").next().unwrap_or(path));
        out.push('`');
        rest = &rest[close + 2..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_nullable_string_renders_as_a_union() {
        let rendered = render_type(&json!({ "type": ["string", "null"] }), "#").unwrap();
        assert_eq!(rendered, "string | null");
    }

    #[test]
    fn a_string_map_renders_as_a_record() {
        let rendered = render_type(
            &json!({ "type": "object", "additionalProperties": { "type": "string" } }),
            "#",
        )
        .unwrap();
        assert_eq!(rendered, "Record<string, string>");
    }

    #[test]
    fn an_unknown_json_type_is_refused_and_names_its_location() {
        let error =
            render_type(&json!({ "type": "widget" }), "#/$defs/X/properties/y").unwrap_err();
        assert_eq!(error.pointer, "#/$defs/X/properties/y");
        assert!(error.reason.contains("widget"), "{}", error.reason);
    }

    #[test]
    fn a_node_with_neither_type_nor_ref_is_refused_rather_than_widened() {
        // The defect this renderer exists not to have: a shape it does not
        // understand becoming `unknown`, compiling, and describing nothing.
        let error = render_type(&json!({ "description": "opaque" }), "#/a").unwrap_err();
        assert!(error.reason.contains("unknown"), "{}", error.reason);
    }

    #[test]
    fn a_remote_reference_is_refused() {
        let error =
            render_type(&json!({ "$ref": "https://example.test/x.json" }), "#/a").unwrap_err();
        assert!(error.reason.contains("$defs"), "{}", error.reason);
    }

    #[test]
    fn an_optional_field_is_the_one_absent_from_required() {
        let rendered = render_interface(
            "T",
            &json!({
                "type": "object",
                "properties": { "a": { "type": "string" }, "b": { "type": "string" } },
                "required": ["a"]
            }),
            "#/$defs/T",
        )
        .unwrap();
        assert!(rendered.contains("  a: string;"), "{rendered}");
        assert!(rendered.contains("  b?: string;"), "{rendered}");
    }

    #[test]
    fn an_intra_doc_link_keeps_only_its_final_segment() {
        assert_eq!(
            flatten_intra_doc_links("see [`crate::protocol::Outcome::Partial`] here"),
            "see `Partial` here"
        );
    }
}
