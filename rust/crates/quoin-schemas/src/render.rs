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
//! properties, string/integer/boolean/null unions, `Record<string, T>` from a
//! `BTreeMap<String, T>`, arrays, `$ref` into `$defs`, newtype aliases
//! (`#[serde(transparent)]`), the closed set of string constants a **unit
//! enum** produces, and the `oneOf` of tagged objects an **internally tagged
//! enum** produces. A newtype and a unit enum arrive as a `$defs` entry that is
//! not an object at all; those become `export type X = string;` and
//! `export type X = "a" | "b";`, which is what the Rust side means and what a
//! TypeScript caller can check. When a boundary type needs more, this module
//! grows a case and a test in the same commit — which is the point of refusing
//! rather than degrading.

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

/// Render one `$defs` entry as a TypeScript declaration.
///
/// An object schema with named properties becomes an `interface`
/// ([`render_interface`]); a `oneOf` of tagged objects becomes named variant
/// interfaces plus a union ([`render_tagged_union`]); anything else the
/// renderer understands becomes a type alias. The alias case is what a
/// `#[serde(transparent)]` newtype (`ObligationId`, `ModuleName`, `CommitSha`)
/// and a unit enum (`FindingKind`, `Mode`) arrive as.
///
/// A newtype is deliberately NOT inlined into its uses. `path: RepoPath` and
/// `subject: string` are the same bytes on the wire and different things in the
/// domain, and the whole reason those newtypes exist is that swapping two of
/// them was a silent payload defect. Carrying the name across the boundary
/// keeps the distinction legible on the TypeScript side, even though TypeScript
/// will not enforce it.
///
/// # Errors
///
/// [`RenderRefusal`] for any construct outside the supported subset.
pub fn render_definition(
    name: &str,
    schema: &Value,
    pointer: &str,
) -> Result<String, RenderRefusal> {
    let object = schema
        .as_object()
        .ok_or_else(|| refuse(pointer, "a `$defs` entry must be an object schema"))?;
    if object.get("type").and_then(Value::as_str) == Some("object")
        && object.contains_key("properties")
    {
        return render_interface(name, schema, pointer);
    }
    // `is_empty` first, and deliberately: `all` is vacuously true over an empty
    // slice, so without it an empty `oneOf` took the tagged-union arm and this
    // renderer emitted `export type X = ;` — invalid TypeScript, silently, from
    // a renderer whose whole contract is to refuse rather than degrade. An
    // empty `oneOf` falls through to `render_type`, which refuses it by name.
    if let Some(members) = object.get("oneOf").and_then(Value::as_array)
        && !members.is_empty()
        && members
            .iter()
            .all(|member| member.get("properties").is_some())
    {
        return render_tagged_union(name, object, members, pointer);
    }
    let mut out = String::new();
    if let Some(description) = object.get("description").and_then(Value::as_str) {
        out.push_str(&doc_comment(description, ""));
    }
    let rendered = render_type(schema, pointer)?;
    out.push_str(&alias_declaration(name, &rendered));
    Ok(out)
}

/// Render an internally tagged enum as named variant interfaces plus a union.
///
/// Each variant is lifted into its own `export interface`, named for the
/// definition and its discriminant (`Source` + `git-subdir` →
/// `SourceGitSubdir`). The alternative — a union of anonymous object literals —
/// would hand a TypeScript reader six shapes with no names to refer to, and
/// would put the renderer in the business of emitting nested anonymous
/// interfaces, which [`render_named_type`] refuses everywhere else for the
/// reason that a shape worth writing down is worth naming.
fn render_tagged_union(
    name: &str,
    object: &serde_json::Map<String, Value>,
    members: &[Value],
    pointer: &str,
) -> Result<String, RenderRefusal> {
    let mut out = String::new();
    let mut variants = Vec::with_capacity(members.len());
    for (index, member) in members.iter().enumerate() {
        let member_pointer = format!("{pointer}/oneOf/{index}");
        let discriminant = discriminant_of(member, &member_pointer)?;
        let variant = format!("{name}{}", pascal_case(&discriminant));
        out.push_str(&render_interface(&variant, member, &member_pointer)?);
        out.push('\n');
        variants.push(variant);
    }
    if let Some(description) = object.get("description").and_then(Value::as_str) {
        out.push_str(&doc_comment(description, ""));
    }
    out.push_str(&alias_declaration(name, &variants.join(" | ")));
    Ok(out)
}

/// The `const` value of the single literal-valued property of a variant.
fn discriminant_of(member: &Value, pointer: &str) -> Result<String, RenderRefusal> {
    let properties = member
        .get("properties")
        .and_then(Value::as_object)
        .ok_or_else(|| refuse(pointer, "a union variant must declare `properties`"))?;
    let mut found: Option<&str> = None;
    for property in properties.values() {
        if let Some(literal) = property.get("const").and_then(Value::as_str) {
            if found.is_some() {
                return Err(refuse(
                    pointer,
                    "a union variant with two literal-valued properties has no \
                     single discriminant to name it by",
                ));
            }
            found = Some(literal);
        }
    }
    found.map(ToOwned::to_owned).ok_or_else(|| {
        refuse(
            pointer,
            "a union variant needs a literal-valued property to name it by; an \
             untagged union is not in the supported subset",
        )
    })
}

fn pascal_case(value: &str) -> String {
    value
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>() + chars.as_str()
            })
        })
        .collect()
}

/// `export type Name = …;`, wrapped the way `prettier` wraps it.
///
/// The generated file is digest-asserted against a fresh render, and `make
/// types` runs `prettier` over the file it wrote — so a declaration `prettier`
/// would reflow is a declaration that breaks the assertion. Only a union can
/// exceed the print width here, and `prettier`'s form for that is one
/// leading-pipe arm per line.
fn alias_declaration(name: &str, rendered: &str) -> String {
    const PRINT_WIDTH: usize = 80;
    let single = format!("export type {name} = {rendered};");
    if single.len() <= PRINT_WIDTH || !rendered.contains(" | ") {
        return single + "\n";
    }
    let mut out = format!("export type {name} =\n");
    for arm in rendered.split(" | ") {
        let _ = writeln!(out, "  | {arm}");
    }
    out.truncate(out.len().saturating_sub(1));
    out.push_str(";\n");
    out
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

    // A closed set of string constants, in either spelling `schemars` uses: a
    // bare `enum` when the variants carry no documentation, and a `oneOf` of
    // `const`s when they do. Both are one Rust enum whose variants are all
    // unit, and both mean the same TypeScript union.
    if let Some(members) = object.get("oneOf").and_then(Value::as_array) {
        if members.is_empty() {
            return Err(refuse(pointer, "an empty `oneOf` names no type"));
        }
        let mut parts = Vec::with_capacity(members.len());
        for (index, member) in members.iter().enumerate() {
            let member_pointer = format!("{pointer}/oneOf/{index}");
            // `const`, read here rather than by recursing through
            // `render_type`. Recursion would render whatever a member happened
            // to be — an object, a `$ref`, a `type` array — and this arm exists
            // for exactly one shape: `schemars`' spelling of a documented unit
            // enum. A `oneOf` of anything else is a boundary type nobody has
            // designed, and the module's contract is to refuse rather than
            // render something plausible (quoin#448 FND-007).
            let Some(constant) = member.get("const") else {
                return Err(refuse(
                    &member_pointer,
                    "a `oneOf` member must be a string `const`; a union of \
                     anything else is outside the subset this renderer supports",
                ));
            };
            parts.push(string_literal(constant, &member_pointer)?);
        }
        return Ok(union_of(parts));
    }
    if let Some(constant) = object.get("const") {
        return string_literal(constant, pointer);
    }
    if let Some(members) = object.get("enum").and_then(Value::as_array) {
        if members.is_empty() {
            return Err(refuse(pointer, "an empty `enum` names no value"));
        }
        let mut parts = Vec::with_capacity(members.len());
        for (index, member) in members.iter().enumerate() {
            parts.push(string_literal(member, &format!("{pointer}/enum/{index}"))?);
        }
        return Ok(union_of(parts));
    }

    // `anyOf`, which is what `schemars` emits for an `Option<T>` whose inner
    // type is itself a named `$ref`. Rendered by recursion rather than by the
    // `const`-only rule above, because these members are types and not
    // literals; a member outside the subset still refuses, one level down.
    if let Some(members) = object.get("anyOf").and_then(Value::as_array) {
        if members.is_empty() {
            return Err(refuse(pointer, "an empty `anyOf` names no type"));
        }
        let mut parts = Vec::with_capacity(members.len());
        for (index, member) in members.iter().enumerate() {
            parts.push(render_type(member, &format!("{pointer}/anyOf/{index}"))?);
        }
        return Ok(union_of(parts));
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
            Ok(union_of(parts))
        }
        _ => Err(refuse(pointer, "`type` must be a string or an array")),
    }
}

/// Join rendered members into a union, dropping repeats and keeping order.
///
/// `Vec::dedup` drops only ADJACENT repeats, so it left `"a" | "b" | "a"`
/// standing — a duplicate in a hand-written schema is exactly the case that is
/// not already sorted (quoin#448 FND-007). Sorting instead would dedup
/// correctly and reorder the union, and the order of a union is the order the
/// schema declared: `"error" | "warning" | "info"` is a severity ladder, not
/// an alphabet. So: first occurrence wins, later repeats drop.
fn union_of(parts: Vec<String>) -> String {
    let mut seen = std::collections::BTreeSet::new();
    let mut unique = Vec::with_capacity(parts.len());
    for part in parts {
        if seen.insert(part.clone()) {
            unique.push(part);
        }
    }
    unique.join(" | ")
}

/// A JSON string constant as a TypeScript string-literal type.
///
/// Only strings: a numeric or boolean constant on the boundary would be a
/// decision nobody has made yet, and guessing at one is exactly the widening
/// this module refuses.
fn string_literal(value: &Value, pointer: &str) -> Result<String, RenderRefusal> {
    let text = value.as_str().ok_or_else(|| {
        refuse(
            pointer,
            "only string constants are rendered; a non-string `const` is \
             outside the subset",
        )
    })?;
    serde_json::to_string(text).map_err(|error| {
        refuse(
            pointer,
            format!("a constant that is not encodable: {error}"),
        )
    })
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

    /// A `#[serde(transparent)]` newtype emits a `$defs` entry that is not an
    /// object, which `render_interface` refuses by design. It becomes an alias
    /// rather than being inlined, so `path: RepoPath` still reads as a path on
    /// the TypeScript side.
    #[test]
    fn a_newtype_definition_becomes_a_type_alias() {
        let rendered = render_definition(
            "RepoPath",
            &json!({ "description": "A repo-relative path.", "type": "string" }),
            "#/$defs/RepoPath",
        )
        .unwrap();
        assert_eq!(
            rendered,
            "/**\n * A repo-relative path.\n */\nexport type RepoPath = string;\n"
        );
    }

    /// A unit-variant enum whose variants carry doc comments: `schemars` emits
    /// `oneOf` of `const`s, one per variant.
    #[test]
    fn a_documented_unit_enum_becomes_a_string_literal_union() {
        let rendered = render_definition(
            "FindingKind",
            &json!({
                "oneOf": [
                    { "type": "string", "const": "gate-that-gates-nothing" },
                    { "type": "string", "const": "something-else" }
                ]
            }),
            "#/$defs/FindingKind",
        )
        .unwrap();
        assert_eq!(
            rendered,
            "export type FindingKind = \"gate-that-gates-nothing\" | \"something-else\";\n"
        );
    }

    /// A `oneOf` of anything but string constants is refused, not rendered.
    ///
    /// The arm was written for one shape — `schemars`' documented unit enum —
    /// and before quoin#448 it recursed through `render_type`, so a `oneOf` of
    /// objects or of `$ref`s rendered a union nobody designed. `tc_type_surface`
    /// counts interfaces, so an alias of the wrong shape reached the TypeScript
    /// side unremarked. Each member below renders PERFECTLY WELL on its own —
    /// that is what made the widening invisible — so the refusal asserted here
    /// can only be the `oneOf` arm's own.
    #[test]
    fn a_one_of_that_is_not_a_union_of_string_constants_is_refused() {
        for member in [
            json!({ "$ref": "#/$defs/Finding" }),
            json!({ "type": "array", "items": { "type": "string" } }),
            json!({ "type": "string" }),
        ] {
            // Renders perfectly well on its own — so the refusal below is this
            // ARM's decision, not a member that could not be rendered at all.
            assert!(
                render_type(&member, "#/$defs/X").is_ok(),
                "fixture: {member} must render on its own, or the refusal below \
                 proves nothing about the `oneOf` arm"
            );
            let refusal = render_type(
                &json!({ "oneOf": [ { "const": "a" }, member ] }),
                "#/$defs/X",
            )
            .unwrap_err();
            assert!(
                refusal.to_string().contains("#/$defs/X/oneOf/1"),
                "the refusal must name the member that caused it: {refusal}"
            );
        }
    }

    /// Repeats drop and the declared order survives.
    ///
    /// `Vec::dedup` dropped only adjacent repeats, so the first union below
    /// rendered `"a" | "b" | "a"` — invalid-looking TypeScript generated by
    /// the thing that exists to refuse rather than degrade (quoin#448 FND-007).
    /// A duplicate that is already adjacent is the case a sort would have
    /// fixed; this is the one it would not.
    #[test]
    fn a_non_adjacent_repeat_is_dropped_and_the_declared_order_stands() {
        assert_eq!(
            render_type(&json!({ "enum": ["a", "b", "a"] }), "#/$defs/X").unwrap(),
            "\"a\" | \"b\""
        );
        // Not alphabetical, and not sorted into being alphabetical: a union's
        // order is the schema's, and `error | warning | info` is a ladder.
        assert_eq!(
            render_type(
                &json!({ "enum": ["error", "warning", "info", "error"] }),
                "#/$defs/X"
            )
            .unwrap(),
            "\"error\" | \"warning\" | \"info\""
        );
    }

    /// The undocumented spelling of the same Rust construct — a bare `enum` —
    /// must render identically, because it IS the same construct.
    #[test]
    fn a_bare_enum_renders_the_same_as_a_oneof_of_consts() {
        let rendered = render_type(&json!({ "type": "string", "enum": ["a", "b"] }), "#").unwrap();
        assert_eq!(rendered, "\"a\" | \"b\"");
    }

    /// A constant that is not a string is a decision nobody has made; the
    /// renderer refuses rather than guessing at a numeric literal type.
    #[test]
    fn a_non_string_constant_is_refused_rather_than_guessed_at() {
        let error = render_type(&json!({ "const": 7 }), "#/$defs/X").unwrap_err();
        assert_eq!(error.pointer, "#/$defs/X");
        assert!(
            error.reason.contains("string constants"),
            "{}",
            error.reason
        );
    }

    /// A string constant with a quote in it is escaped, not concatenated into
    /// a broken literal.
    #[test]
    fn a_string_constant_is_escaped() {
        let rendered = render_type(&json!({ "const": "a\"b" }), "#").unwrap();
        assert_eq!(rendered, "\"a\\\"b\"");
    }

    /// An object definition still becomes an interface: the alias case must not
    /// have swallowed the common one.
    #[test]
    fn an_object_definition_is_still_an_interface() {
        let rendered = render_definition(
            "T",
            &json!({ "type": "object", "properties": { "a": { "type": "string" } }, "required": ["a"] }),
            "#/$defs/T",
        )
        .unwrap();
        assert!(rendered.starts_with("export interface T {"), "{rendered}");
    }

    #[test]
    fn a_transparent_newtype_stays_a_named_alias() {
        // `#[serde(transparent)]` on `ModuleName` erases the wrapper on the
        // wire but not the name in the declaration. Rendering it as bare
        // `string` at each use site would lose the only signal a TypeScript
        // reader has that the field is validated.
        let rendered = render_definition(
            "ModuleName",
            &json!({ "type": "string", "description": "A name." }),
            "#/$defs/ModuleName",
        )
        .unwrap();
        assert!(
            rendered.contains("export type ModuleName = string;"),
            "{rendered}"
        );
        assert!(rendered.contains("A name."), "{rendered}");
    }

    #[test]
    fn a_unit_enum_renders_as_a_union_of_string_literals() {
        let rendered = render_definition(
            "Mode",
            &json!({
                "oneOf": [
                    { "const": "lazy", "type": "string" },
                    { "const": "sync", "type": "string" }
                ]
            }),
            "#/$defs/Mode",
        )
        .unwrap();
        assert_eq!(rendered, "export type Mode = \"lazy\" | \"sync\";\n");
    }

    #[test]
    fn an_internally_tagged_enum_renders_as_a_discriminated_union() {
        let rendered = render_definition(
            "Source",
            &json!({
                "oneOf": [
                    {
                        "type": "object",
                        "properties": {
                            "type": { "const": "path", "type": "string" },
                            "path": { "type": "string" }
                        },
                        "required": ["type", "path"]
                    },
                    {
                        "type": "object",
                        "properties": {
                            "type": { "const": "npm", "type": "string" },
                            "package": { "type": "string" },
                            "version": { "type": ["string", "null"] }
                        },
                        "required": ["type", "package"]
                    }
                ]
            }),
            "#/$defs/Source",
        )
        .unwrap();
        // Each variant is a NAMED interface, so a caller can annotate one.
        assert!(
            rendered.contains("export interface SourcePath {"),
            "{rendered}"
        );
        assert!(
            rendered.contains("export interface SourceNpm {"),
            "{rendered}"
        );
        assert!(
            rendered.contains("export type Source = SourcePath | SourceNpm;"),
            "{rendered}"
        );
        // The discriminant is a literal type on every arm, which is what makes
        // `source.type === "path"` narrow in TypeScript. A widening to `string`
        // here would compile and narrow nothing.
        assert!(rendered.contains(r#"type: "path";"#), "{rendered}");
        assert!(rendered.contains(r#"type: "npm";"#), "{rendered}");
        assert!(rendered.contains("version?: string | null;"), "{rendered}");
    }

    #[test]
    fn a_variant_name_pascal_cases_its_kebab_discriminant() {
        let rendered = render_definition(
            "Source",
            &json!({
                "oneOf": [{
                    "type": "object",
                    "properties": {
                        "type": { "const": "git-subdir", "type": "string" },
                        "url": { "type": "string" }
                    },
                    "required": ["type", "url"]
                }]
            }),
            "#/$defs/Source",
        )
        .unwrap();
        assert!(
            rendered.contains("export interface SourceGitSubdir {"),
            "{rendered}"
        );
    }

    #[test]
    fn an_untagged_union_variant_is_refused_rather_than_named_by_position() {
        // `Source1`, `Source2` would compile and mean nothing. A variant with
        // no discriminant is a shape the renderer cannot name, so it refuses.
        let error = render_definition(
            "Thing",
            &json!({
                "oneOf": [{
                    "type": "object",
                    "properties": { "a": { "type": "string" } },
                    "required": ["a"]
                }]
            }),
            "#/$defs/Thing",
        )
        .unwrap_err();
        assert!(error.reason.contains("untagged"), "{}", error.reason);
    }

    #[test]
    fn a_union_too_long_for_one_line_wraps_the_way_prettier_wraps_it() {
        // The generated file is digest-asserted against a fresh render and
        // `make types` runs `prettier` over it, so a declaration `prettier`
        // would reflow breaks the assertion on the next gate run.
        let arms = [
            "SourceGithub",
            "SourceGitSubdir",
            "SourceGit",
            "SourceUrl",
            "SourcePath",
            "SourceNpm",
        ];
        let rendered = alias_declaration("Source", &arms.join(" | "));
        assert_eq!(
            rendered,
            "export type Source =\n  | SourceGithub\n  | SourceGitSubdir\n  \
             | SourceGit\n  | SourceUrl\n  | SourcePath\n  | SourceNpm;\n"
        );
        assert_eq!(
            alias_declaration("Narrow", "Alpha | Bravo"),
            "export type Narrow = Alpha | Bravo;\n"
        );
    }

    #[test]
    fn an_option_of_a_referenced_type_renders_as_a_nullable_reference() {
        let rendered = render_type(
            &json!({ "anyOf": [{ "$ref": "#/$defs/SemanticPin" }, { "type": "null" }] }),
            "#/$defs/InstalledModule/properties/semantic",
        )
        .unwrap();
        assert_eq!(rendered, "SemanticPin | null");
    }

    #[test]
    fn an_inline_object_in_a_field_position_is_still_refused() {
        // The union-member case above is the ONLY place an anonymous object
        // type is emitted. A nested object in a field position hides a type
        // that ought to be named, and that refusal must survive the widening.
        let error = render_type(
            &json!({ "type": "object", "properties": { "a": { "type": "string" } } }),
            "#/$defs/T/properties/nested",
        )
        .unwrap_err();
        assert!(error.reason.contains("$defs"), "{}", error.reason);
    }

    #[test]
    fn an_empty_union_is_refused() {
        let error = render_type(&json!({ "oneOf": [] }), "#/a").unwrap_err();
        assert!(error.reason.contains("oneOf"), "{}", error.reason);
    }

    /// The same refusal at the `$defs` entry point, which is the one a real
    /// schema reaches. `render_definition` has its own `oneOf` arm for tagged
    /// unions and its guard is an `all()` — vacuously true over an empty slice
    /// — so the assertion above proves nothing about this path.
    #[test]
    fn an_empty_union_is_refused_as_a_definition_too() {
        let error = render_definition("X", &json!({ "oneOf": [] }), "#/$defs/X").unwrap_err();
        assert!(error.reason.contains("oneOf"), "{}", error.reason);
    }

    #[test]
    fn an_intra_doc_link_keeps_only_its_final_segment() {
        assert_eq!(
            flatten_intra_doc_links("see [`crate::protocol::Outcome::Partial`] here"),
            "see `Partial` here"
        );
    }
}
