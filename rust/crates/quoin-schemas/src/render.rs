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
//! enum** produces, the `oneOf` of tagged objects an **internally tagged
//! enum** produces, and the `oneOf` of single-property objects an **externally
//! tagged enum** produces. A newtype and a unit enum arrive as a `$defs` entry that is
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
/// Five shapes, because five are what `schemars` emits for the boundary's
/// canonical types and refusing any of them would mean declaring the wire type
/// twice — once in Rust and once by hand in TypeScript, which is the
/// duplication FR-097 exists to remove:
///
/// - an object schema with named properties → `export interface X { … }`
/// - a `oneOf` of `const` strings (a Rust unit enum) → `export type X = "a" | "b";`
/// - a `oneOf` of object variants that each carry a literal-valued property (an
///   **internally tagged** Rust enum) → one named `export interface` per
///   variant plus `export type Source = SourcePath | SourceNpm;`
/// - a `oneOf` of object variants with no such discriminant (an **externally
///   tagged** Rust enum, whose arms have no names of their own on the wire) →
///   `export type X = { a: string } | { b: string };`
/// - a scalar schema (a `#[serde(transparent)]` newtype) → `export type X = string;`
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
/// [`RenderRefusal`] for a definition outside those shapes, or for any property
/// using a construct outside the supported subset.
pub fn render_definition(
    name: &str,
    schema: &Value,
    pointer: &str,
) -> Result<String, RenderRefusal> {
    let object = schema
        .as_object()
        .ok_or_else(|| refuse(pointer, "a `$defs` entry must be an object schema"))?;

    if object.contains_key("oneOf") {
        return render_union(name, object, pointer);
    }
    // BOTH halves of this condition, and neither alone. `properties` without
    // `"type": "object"` is a schema that never said it was an object, and
    // `"type": "object"` without `properties` is a map whose
    // `additionalProperties` the alias arm below renders as a `Record`. Under
    // `||` the first emitted an interface for a shape that had not claimed to
    // be one and the second emitted `export interface X {}`, which accepts
    // anything (quoin#450 review, finding 6).
    if object.get("type").and_then(Value::as_str) == Some("object")
        && object.contains_key("properties")
    {
        return render_interface(name, schema, pointer);
    }
    // A `#[serde(transparent)]` newtype: the wire carries the inner scalar and
    // nothing else, so an alias is the whole truth about it. An `interface`
    // here would invent an object the wire never carries.
    let alias = render_type(schema, pointer)?;
    let mut out = String::new();
    if let Some(description) = object.get("description").and_then(Value::as_str) {
        out.push_str(&doc_comment(description, ""));
    }
    let _ = writeln!(out, "export type {name} = {alias};");
    Ok(out)
}

/// Render a `oneOf` definition as a TypeScript union.
///
/// Three kinds of union arrive here and none of them is mixed with another: a
/// union of `const` strings, a union of internally tagged object variants, and
/// a union of externally tagged ones. A definition carrying two of those kinds
/// at once is a shape nobody has decided the meaning of yet, and is refused
/// rather than guessed.
///
/// The discriminant is what decides between the two object unions, and it is a
/// real distinction rather than a convenience: an internally tagged variant
/// carries a literal-valued property, so it HAS a name of its own to be lifted
/// into (`Source` + `git-subdir` → `SourceGitSubdir`), and naming it is what
/// makes `source.type === "path"` narrow on the TypeScript side. An externally
/// tagged variant carries no such literal; there is nothing to name it by, and
/// `Source1`, `Source2` would compile and mean nothing — so it is rendered
/// inline, which is the one place an anonymous object type is the honest
/// answer.
///
/// `is_empty` is tested FIRST, and deliberately: every classification below is
/// a count over the members and `all`-shaped reasoning is vacuously true over
/// an empty slice, so without this guard an empty `oneOf` took a union arm and
/// the renderer emitted `export type X = ;` — invalid TypeScript, silently,
/// from a module whose whole contract is to refuse rather than degrade.
fn render_union(
    name: &str,
    object: &serde_json::Map<String, Value>,
    pointer: &str,
) -> Result<String, RenderRefusal> {
    let members = object
        .get("oneOf")
        .and_then(Value::as_array)
        .ok_or_else(|| refuse(pointer, "`oneOf` must be an array"))?;
    if members.is_empty() {
        return Err(refuse(pointer, "an empty `oneOf` names no type"));
    }

    let objects = members
        .iter()
        .filter(|member| member.get("properties").is_some())
        .count();
    let consts = members
        .iter()
        .filter(|member| member.get("const").is_some())
        .count();
    if consts != 0 && objects != 0 {
        return Err(refuse(
            pointer,
            "a `oneOf` mixing string constants with object variants is not \
             rendered; decide which kind of union it is",
        ));
    }
    if objects == members.len() {
        let tagged = members
            .iter()
            .filter(|member| has_discriminant(member))
            .count();
        if tagged == members.len() {
            return render_tagged_union(name, object, members, pointer);
        }
        if tagged != 0 {
            return Err(refuse(
                pointer,
                "a `oneOf` in which only some object variants carry a \
                 literal-valued discriminant is not rendered; tag every variant \
                 or none",
            ));
        }
    }

    let mut parts: Vec<String> = Vec::with_capacity(members.len());
    for (index, member) in members.iter().enumerate() {
        let member_pointer = format!("{pointer}/oneOf/{index}");
        let member_object = member
            .as_object()
            .ok_or_else(|| refuse(&member_pointer, "a `oneOf` member must be a schema object"))?;
        if let Some(literal) = member_object.get("const") {
            let text = literal.as_str().ok_or_else(|| {
                refuse(
                    &member_pointer,
                    "only string `const` variants are rendered; a numeric or \
                     structured constant has no TypeScript literal here",
                )
            })?;
            parts.push(Value::String(text.to_owned()).to_string());
        } else {
            parts.push(render_object_literal(member_object, &member_pointer)?);
        }
    }

    let mut out = String::new();
    if let Some(description) = object.get("description").and_then(Value::as_str) {
        out.push_str(&doc_comment(description, ""));
    }
    out.push_str(&union_declaration(name, &parts));
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
    out.push_str(&union_declaration(name, &variants));
    Ok(out)
}

/// Whether a `oneOf` member carries a literal-valued property to be named by.
///
/// The classification question only — [`discriminant_of`] answers the harder
/// one, and still refuses a variant carrying two literals once the tagged arm
/// has been chosen. Splitting them keeps a two-discriminant variant a REFUSAL
/// rather than a silent fall-through into the externally tagged arm, which
/// would have rendered it inline and dropped the defect on the floor.
fn has_discriminant(member: &Value) -> bool {
    member
        .get("properties")
        .and_then(Value::as_object)
        .is_some_and(|properties| {
            properties
                .values()
                .any(|property| property.get("const").and_then(Value::as_str).is_some())
        })
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

/// Lay a union declaration out the way `prettier` lays it out.
///
/// The generated file is `prettier`-formatted after it is written (`make
/// types`), and the digest gate asserts the bytes on disk against a fresh
/// render — so a declaration emitted in a shape `prettier` would reflow makes
/// that gate permanently red. Agreeing with the formatter is what keeps the
/// two readings of the same file equal.
///
/// `prettier`'s three forms, in the order it tries them, at its default print
/// width of 80:
///
/// - the whole declaration on one line;
/// - a break after `=`, arms still on one line, indented two spaces;
/// - one arm per line, each led by `| `.
fn union_declaration(name: &str, parts: &[String]) -> String {
    const PRINT_WIDTH: usize = 80;

    let joined = parts.join(" | ");
    let one_line = format!("export type {name} = {joined};");
    if one_line.chars().count() <= PRINT_WIDTH {
        return format!("{one_line}\n");
    }
    if joined.chars().count() + 3 <= PRINT_WIDTH {
        return format!("export type {name} =\n  {joined};\n");
    }
    let mut out = format!("export type {name} =\n");
    for (index, part) in parts.iter().enumerate() {
        let terminator = if index + 1 == parts.len() { ";" } else { "" };
        let _ = writeln!(out, "  | {part}{terminator}");
    }
    out
}

/// Render one object schema as an inline TypeScript object type.
///
/// Only reached from [`render_union`], for an externally tagged variant. A
/// standalone inline object is still refused by [`render_type`]: an anonymous
/// nested interface is unreadable and unreferenceable, whereas an externally
/// tagged union arm has nowhere else to live — it carries no discriminant, so
/// there is no name to lift it into.
fn render_object_literal(
    object: &serde_json::Map<String, Value>,
    pointer: &str,
) -> Result<String, RenderRefusal> {
    let properties = object
        .get("properties")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            refuse(
                pointer,
                "a `oneOf` object variant must declare `properties`",
            )
        })?;
    let required: Vec<&str> = object
        .get("required")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();

    let mut fields: Vec<String> = Vec::with_capacity(properties.len());
    for (field, property) in properties {
        let optional = if required.contains(&field.as_str()) {
            ""
        } else {
            "?"
        };
        let rendered = render_type(property, &format!("{pointer}/properties/{field}"))?;
        fields.push(format!("{field}{optional}: {rendered}"));
    }
    Ok(format!("{{ {} }}", fields.join("; ")))
}

/// The JSON Schema keywords that annotate a node without constraining it.
///
/// The list is of ANNOTATION keywords, and the test against it is that EVERY
/// key of a node is one of them. That direction is the load-bearing part: a
/// keyword this renderer has never seen is, by construction, not on this list,
/// so it falls through to a refusal instead of being read as "asserts nothing".
/// An assertion list would have had the opposite failure — `multipleOf` alone
/// would have rendered as `unknown` and silently dropped the constraint.
const ANNOTATIONS: &[&str] = &[
    "$comment",
    "default",
    "deprecated",
    "description",
    "examples",
    "readOnly",
    "title",
    "writeOnly",
];

/// Render one `$defs` entry as a TypeScript `interface` declaration.
///
/// # Errors
///
/// [`RenderRefusal`] when the definition is not an object schema with named
/// properties, or when any property uses a construct outside the supported
/// subset.
fn render_interface(name: &str, schema: &Value, pointer: &str) -> Result<String, RenderRefusal> {
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
    // The two BOOLEAN schemas draft 2020-12 defines. `true` admits every JSON
    // value, which `unknown` states exactly — this is not the renderer widening
    // a shape it failed to understand, it is the only faithful rendering of a
    // schema that genuinely accepts anything. `false` admits nothing, which
    // TypeScript can spell as `never` but which no honest wire type reaches, so
    // it is refused rather than rendered.
    if let Some(open) = schema.as_bool() {
        return if open {
            Ok("unknown".to_owned())
        } else {
            Err(refuse(
                pointer,
                "a `false` schema admits no value at all; nothing on the wire can satisfy it",
            ))
        };
    }

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
    // `Option<T>` where `T` is a named type. `schemars` cannot fold the null
    // into a `type` array here — the arm carrying the value is a `$ref`, which
    // has no `type` of its own — so it emits a two-member `anyOf` instead. It
    // is the same fact as `["string", "null"]` below, in the only spelling the
    // draft allows for a reference, and rendering it as `T | null` is a
    // translation rather than a widening.
    //
    // Rendered by recursion rather than by the `const`-only rule above, because
    // these members are types and not literals; a member outside the subset
    // still refuses, one level down.
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

    // An object schema carrying nothing but annotations asserts nothing, so it
    // admits every JSON value — it is the boolean `true` above written the long
    // way, and it is what `schemars` emits for a documented `serde_json::Value`
    // field. `unknown` states that exactly.
    if object.keys().all(|key| ANNOTATIONS.contains(&key.as_str())) {
        return Ok("unknown".to_owned());
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

    /// `Record<string, unknown>` is the honest rendering of a map whose values
    /// are arbitrary JSON — a document's frontmatter, say. Paired with the
    /// refusal below so the permissive half cannot be read as "boolean schemas
    /// render as unknown".
    #[test]
    fn an_open_value_map_renders_as_a_record_of_unknown() {
        let rendered = render_type(
            &json!({ "type": "object", "additionalProperties": true }),
            "#",
        )
        .unwrap();
        assert_eq!(rendered, "Record<string, unknown>");
    }

    /// The refusal reaches a human verbatim, so the WHOLE sentence is asserted
    /// and not a fragment of it. A `contains("no value at all")` left the
    /// second half unasserted, and it shipped with an 18-space run inside it.
    #[test]
    fn a_false_schema_is_refused_rather_than_rendered() {
        let error = render_type(&json!(false), "#/a").unwrap_err();
        assert_eq!(
            error.reason,
            "a `false` schema admits no value at all; nothing on the wire can satisfy it"
        );
    }

    /// A Rust unit enum reaches the wire as a closed set of strings, and a
    /// literal union is the only TypeScript that says so. `string` would
    /// compile and admit every typo.
    #[test]
    fn a_const_union_definition_renders_as_a_string_literal_union() {
        let rendered = render_definition(
            "Verdict",
            &json!({
                "oneOf": [
                    { "const": "PASS", "type": "string" },
                    { "const": "UNCHECKED", "type": "string" },
                ]
            }),
            "#/$defs/Verdict",
        )
        .unwrap();
        assert_eq!(
            rendered,
            "export type Verdict = \"PASS\" | \"UNCHECKED\";\n"
        );
    }

    /// An externally tagged Rust enum. The arms have no names of their own on
    /// the wire, so they are rendered inline — the one place an anonymous
    /// object type is the honest answer.
    #[test]
    fn an_object_union_definition_renders_as_a_union_of_object_literals() {
        let rendered = render_definition(
            "SchemaSource",
            &json!({
                "oneOf": [
                    {
                        "type": "object",
                        "properties": { "text": { "type": "string" } },
                        "required": ["text"],
                    },
                    {
                        "type": "object",
                        "properties": { "unreadable": { "type": "string" } },
                        "required": ["unreadable"],
                    },
                ]
            }),
            "#/$defs/SchemaSource",
        )
        .unwrap();
        assert_eq!(
            rendered,
            "export type SchemaSource = { text: string } | { unreadable: string };\n"
        );
    }

    /// `prettier` reflows a union declaration that does not fit in 80 columns,
    /// and the digest gate reads the reflowed bytes back. A renderer that
    /// emitted the long form would make that gate permanently red.
    #[test]
    fn a_union_too_wide_for_one_line_breaks_after_the_equals_sign() {
        let rendered = render_definition(
            "FindingKind",
            &json!({
                "oneOf": [
                    { "const": "unowned", "type": "string" },
                    { "const": "unjustified-exclusion", "type": "string" },
                    { "const": "undeclared-exclusion", "type": "string" },
                ]
            }),
            "#/$defs/FindingKind",
        )
        .unwrap();
        assert_eq!(
            rendered,
            "export type FindingKind =\n  \"unowned\" | \"unjustified-exclusion\" | \"undeclared-exclusion\";\n"
        );
    }

    /// The third form: arms that do not fit on one indented line each get
    /// their own, led by `| `, which is again what `prettier` does.
    #[test]
    fn a_union_too_wide_even_when_broken_gets_one_arm_per_line() {
        let wide = |text: &str| json!({ "const": text, "type": "string" });
        let rendered = render_definition(
            "Wide",
            &json!({
                "oneOf": [
                    wide("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
                    wide("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
                ]
            }),
            "#/$defs/Wide",
        )
        .unwrap();
        assert_eq!(
            rendered,
            "export type Wide =\n  | \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"\n  | \"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\";\n"
        );
    }

    /// `Option<T>` over a named type. `schemars` has no way to spell this as a
    /// `type` array, so it emits an `anyOf`; rendering it is the same
    /// translation the `["string", "null"]` case already performs.
    #[test]
    fn a_nullable_reference_renders_as_a_union_with_null() {
        let rendered = render_type(
            &json!({
                "anyOf": [
                    { "$ref": "#/$defs/DischargeReport" },
                    { "type": "null" },
                ]
            }),
            "#/$defs/AuthoredArgumentView/properties/discharge",
        )
        .unwrap();
        assert_eq!(rendered, "DischargeReport | null");
    }

    #[test]
    fn an_empty_any_of_is_refused() {
        let error = render_type(&json!({ "anyOf": [] }), "#/x").unwrap_err();
        assert!(error.reason.contains("no type"), "{}", error.reason);
    }

    /// A `serde_json::Value` field. `schemars` emits an assertion-free schema
    /// for it, which is `true` written the long way.
    #[test]
    fn an_assertion_free_object_schema_renders_as_unknown() {
        let rendered = render_type(
            &json!({ "description": "The authored argument, unvalidated." }),
            "#/$defs/BuildAuthoredArgumentRequest/properties/argument",
        )
        .unwrap();
        assert_eq!(rendered, "unknown");
    }

    #[test]
    fn a_union_mixing_constants_and_objects_is_refused() {
        let error = render_definition(
            "Mixed",
            &json!({
                "oneOf": [
                    { "const": "a", "type": "string" },
                    { "type": "object", "properties": { "b": { "type": "string" } } },
                ]
            }),
            "#/$defs/Mixed",
        )
        .unwrap_err();
        assert!(
            error.reason.contains("which kind of union"),
            "{}",
            error.reason
        );
    }

    /// A `#[serde(transparent)]` newtype carries its inner scalar and nothing
    /// else, so an alias is the whole truth. An interface would invent an
    /// object the wire never carries.
    #[test]
    fn a_transparent_newtype_definition_renders_as_an_alias() {
        let rendered = render_definition(
            "VocabularyName",
            &json!({ "type": "string", "description": "A declaration's own name." }),
            "#/$defs/VocabularyName",
        )
        .unwrap();
        assert!(
            rendered.ends_with("export type VocabularyName = string;\n"),
            "{rendered}"
        );
        assert!(rendered.contains("A declaration's own name."), "{rendered}");
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
        //
        // The node was `{"description": "opaque"}` until quoin#447, when the
        // boundary acquired its first documented `serde_json::Value` field and
        // `schemars` emitted exactly those bytes for it. An annotation-only
        // schema asserts nothing and so admits everything; that is a shape the
        // renderer DOES understand, and it now renders as `unknown` — see
        // `an_assertion_free_object_schema_renders_as_unknown`. The criterion
        // this test carries is unchanged, and is stated here over a node that
        // is genuinely unintelligible: `multipleOf` is a real constraint this
        // renderer cannot express, so widening it to `unknown` would drop it.
        let error = render_type(&json!({ "multipleOf": 3 }), "#/a").unwrap_err();
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

    /// A variant with no discriminant is never named by position.
    ///
    /// `Source1`, `Source2` would compile and mean nothing, and until quoin#449
    /// the renderer refused such a union outright to avoid inventing those
    /// names. An externally tagged Rust enum is now a supported shape
    /// (`SchemaSource`, on the boundary today), so the union renders — but
    /// INLINE, with no invented name anywhere in the output. The criterion is
    /// unchanged and is asserted directly here: nothing positional is emitted.
    #[test]
    fn an_untagged_union_variant_is_never_named_by_position() {
        let rendered = render_definition(
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
        .unwrap();
        assert_eq!(rendered, "export type Thing = { a: string };\n");
        assert!(!rendered.contains("Thing1"), "{rendered}");
        assert!(!rendered.contains("interface"), "{rendered}");
    }

    /// A variant carrying TWO literal-valued properties still refuses.
    ///
    /// It is a tagged variant by classification — it has a discriminant — but
    /// the renderer cannot tell which of the two names it. The split between
    /// [`has_discriminant`] and [`discriminant_of`] is what keeps this a
    /// refusal: were the tagged arm chosen by "does `discriminant_of` succeed",
    /// this union would have fallen through to the externally tagged arm and
    /// rendered inline, dropping the ambiguity on the floor.
    #[test]
    fn a_variant_with_two_discriminants_is_refused() {
        let error = render_definition(
            "Ambiguous",
            &json!({
                "oneOf": [{
                    "type": "object",
                    "properties": {
                        "kind": { "const": "a", "type": "string" },
                        "tag": { "const": "b", "type": "string" }
                    },
                    "required": ["kind", "tag"]
                }]
            }),
            "#/$defs/Ambiguous",
        )
        .unwrap_err();
        assert!(
            error.reason.contains("two literal-valued properties"),
            "{}",
            error.reason
        );
    }

    /// A union in which only SOME object variants carry a discriminant is
    /// neither kind of union, and is refused rather than rendered as the kind
    /// the first member happened to be.
    #[test]
    fn a_partly_tagged_object_union_is_refused() {
        let error = render_definition(
            "Half",
            &json!({
                "oneOf": [
                    {
                        "type": "object",
                        "properties": { "kind": { "const": "a", "type": "string" } },
                        "required": ["kind"]
                    },
                    {
                        "type": "object",
                        "properties": { "b": { "type": "string" } },
                        "required": ["b"]
                    }
                ]
            }),
            "#/$defs/Half",
        )
        .unwrap_err();
        assert!(
            error.reason.contains("only some object variants"),
            "{}",
            error.reason
        );
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
        let rendered = union_declaration("Source", &arms.map(ToOwned::to_owned));
        assert_eq!(
            rendered,
            "export type Source =\n  | SourceGithub\n  | SourceGitSubdir\n  \
             | SourceGit\n  | SourceUrl\n  | SourcePath\n  | SourceNpm;\n"
        );
        assert_eq!(
            union_declaration("Narrow", &["Alpha".to_owned(), "Bravo".to_owned()]),
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
    /// schema reaches. `render_definition` hands every `oneOf` to
    /// [`render_union`], which classifies its members by counting them —
    /// vacuously satisfied over an empty slice — so the assertion above proves
    /// nothing about this path.
    #[test]
    fn an_empty_union_is_refused_as_a_definition_too() {
        for (name, pointer) in [("X", "#/$defs/X"), ("Nothing", "#/$defs/Nothing")] {
            let error = render_definition(name, &json!({ "oneOf": [] }), pointer).unwrap_err();
            assert_eq!(error.pointer, pointer);
            assert!(error.reason.contains("oneOf"), "{}", error.reason);
            assert!(error.reason.contains("no type"), "{}", error.reason);
        }
    }

    /// The interface arm takes BOTH halves of its condition, and neither half
    /// alone.
    ///
    /// The guard was `type == "object" || contains_key("properties")` and is
    /// now `&&`. Reverting it to `||` left all 39 tests in this crate green,
    /// including the committed-digest case — the rule was pinned in neither
    /// direction (quoin#450 review, finding 6). Both off-diagonal cases are
    /// asserted here, because the whole content of the change is what happens
    /// to them:
    ///
    /// - `properties` with no `"type": "object"` falls through to
    ///   [`render_type`], which refuses by name rather than emitting an
    ///   interface for a schema that never said it was one.
    /// - `"type": "object"` with no `properties` renders the index signature
    ///   its `additionalProperties` describes, rather than an empty interface
    ///   that silently accepts anything.
    #[test]
    fn the_interface_arm_needs_the_type_and_the_properties() {
        let interface = render_definition(
            "X",
            &json!({ "type": "object", "properties": { "a": { "type": "string" } } }),
            "#/$defs/X",
        )
        .unwrap();
        assert!(interface.contains("export interface X"), "{interface}");

        // `properties` alone is not an interface: under `||` this rendered one.
        let error = render_definition(
            "X",
            &json!({ "properties": { "a": { "type": "string" } } }),
            "#/$defs/X",
        )
        .unwrap_err();
        assert_eq!(error.pointer, "#/$defs/X");

        // `type: object` alone is a map, not an empty interface: under `||`
        // this rendered `export interface X {}`.
        let map = render_definition(
            "X",
            &json!({ "type": "object", "additionalProperties": { "type": "string" } }),
            "#/$defs/X",
        )
        .unwrap();
        assert!(map.contains("Record<string, string>"), "{map}");
        assert!(!map.contains("interface"), "{map}");
    }

    #[test]
    fn an_intra_doc_link_keeps_only_its_final_segment() {
        assert_eq!(
            flatten_intra_doc_links("see [`crate::protocol::Outcome::Partial`] here"),
            "see `Partial` here"
        );
    }
}
