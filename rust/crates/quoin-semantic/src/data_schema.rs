// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `data_schema` by emitted-schema path and digest (FR-073, issue #293).
//!
//! Port of `src/semantic/data-schema.ts`. An object type under a module with a
//! `semantic` block references its emitted JSON Schema as
//! `{ schema: <module-relative .json>, digest: sha256:… }`. Quoin resolves the
//! reference at `quoin module install`: the file must sit inside the module root
//! (no `..`, no symlink escape), hash to the recorded digest over its raw bytes,
//! be a JSON Schema 2020-12 document with an absolute `$id` under the module's
//! semantic package base, and every `$ref` it carries must resolve inside the
//! shipped bundle or the vendored semantic-core bundle at the version the
//! manifest records. **No network read, ever.**

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use serde_json::Value;

use crate::contract::{SEMANTIC_CONTRACT, sha256_hex};
use crate::diagnostic::{DiagnosticCode, SemanticDiagnostic};
use crate::ids::{ObjectTypeName, PackageIdentity, SemanticCoreVersion};

const SEMANTIC_CORE_BASE: &str = "https://schemas.agent-ix.org/semantic-core/";
const PACKAGE_BASE: &str = "https://schemas.agent-ix.org/";
const DIALECT_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";

/// Which of the two `data_schema` forms a value is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataSchemaForm {
    /// A JSON Schema written into the manifest.
    Inline,
    /// A `{ schema, digest }` pointer at a shipped file.
    Reference,
    /// Neither — the value is refused.
    Invalid,
}

impl DataSchemaForm {
    /// The wire spelling the TypeScript uses.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Inline => "inline",
            Self::Reference => "reference",
            Self::Invalid => "invalid",
        }
    }
}

/// The result of resolving one object type's `data_schema`.
#[derive(Debug, Clone)]
pub struct ResolvedDataSchema {
    /// Which form was resolved. `Invalid` inputs report as `Inline`, matching
    /// the TypeScript, so a caller counting reference-form exports is not
    /// misled into thinking a malformed value was a reference.
    pub kind: DataSchemaForm,
    /// The parsed schema document (inline object, or referenced file).
    pub schema: Option<Value>,
    /// Absolute path of the referenced file, for the reference form.
    pub file: Option<PathBuf>,
    /// Everything the resolution found wrong.
    pub diagnostics: Vec<SemanticDiagnostic>,
}

/// What a `data_schema` is resolved against.
#[derive(Debug, Clone)]
pub struct ResolveContext {
    /// The module's root directory.
    pub module_root: PathBuf,
    /// `<org>/<repo>` from `semantic.package`.
    pub package_identity: PackageIdentity,
    /// The module's `version`.
    pub module_version: String,
    /// `semantic.semantic_core`.
    pub semantic_core: SemanticCoreVersion,
    /// The object type being resolved.
    pub object_type: ObjectTypeName,
    /// The directory holding the vendored semantic-core bundle.
    pub semantic_core_dir: PathBuf,
}

impl ResolveContext {
    /// `object_types[<name>].data_schema`, the locus every diagnostic hangs off.
    #[must_use]
    pub fn locus(&self) -> String {
        format!("object_types[{}].data_schema", self.object_type)
    }

    /// The base URI a shipped schema's `$id` must sit under.
    #[must_use]
    pub fn package_base(&self) -> String {
        format!(
            "{PACKAGE_BASE}{}/{}/{}/",
            self.package_identity.org(),
            self.package_identity.repo(),
            self.module_version
        )
    }

    /// The base URI a semantic-core `$ref` must sit under.
    #[must_use]
    pub fn core_base(&self) -> String {
        format!("{SEMANTIC_CORE_BASE}{}/", self.semantic_core)
    }
}

/// Classify a `data_schema` value: inline object, reference, or ambiguous.
#[must_use]
pub fn classify_data_schema(
    value: &Value,
    locus: &str,
) -> (DataSchemaForm, Vec<SemanticDiagnostic>) {
    let Some(object) = value.as_object() else {
        return (
            DataSchemaForm::Invalid,
            vec![SemanticDiagnostic::error(
                DiagnosticCode::DataSchemaShape,
                locus,
                "data_schema must be an object",
            )],
        );
    };
    let has_schema = object.contains_key("schema");
    let has_digest = object.contains_key("digest");
    if !has_schema && !has_digest {
        return (DataSchemaForm::Inline, Vec::new());
    }
    let extra: Vec<&str> = object
        .keys()
        .filter(|k| k.as_str() != "schema" && k.as_str() != "digest")
        .map(String::as_str)
        .collect();
    if !extra.is_empty() || !has_schema || !has_digest {
        let detail = if extra.is_empty() {
            "missing schema or digest".to_owned()
        } else {
            extra.join(", ")
        };
        return (
            DataSchemaForm::Invalid,
            vec![SemanticDiagnostic::error(
                DiagnosticCode::DataSchemaAmbiguous,
                locus,
                format!("data_schema mixes the reference form with other keys ({detail})"),
            )],
        );
    }
    (DataSchemaForm::Reference, Vec::new())
}

/// True when `candidate` resolves — through symlinks — strictly inside `root`.
fn inside_root(root: &Path, candidate: &Path) -> bool {
    let Ok(real_root) = std::fs::canonicalize(root) else {
        return false;
    };
    let real = std::fs::canonicalize(candidate).unwrap_or_else(|_| absolutize(candidate));
    match real.strip_prefix(&real_root) {
        Ok(rest) => rest.components().next().is_some(),
        Err(_) => false,
    }
}

/// `path::resolve` without touching the filesystem.
fn absolutize(path: &Path) -> PathBuf {
    let mut out = if path.is_absolute() {
        PathBuf::new()
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))
    };
    for component in path.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

/// Every `$ref` string anywhere in a document, in document order.
fn collect_refs(node: &Value, out: &mut Vec<String>) {
    match node {
        Value::Array(items) => {
            for item in items {
                collect_refs(item, out);
            }
        }
        Value::Object(map) => {
            if let Some(Value::String(reference)) = map.get("$ref") {
                out.push(reference.clone());
            }
            for value in map.values() {
                collect_refs(value, out);
            }
        }
        _ => {}
    }
}

/// True when `path` carries a `..` segment, under either separator.
fn has_parent_segment(path: &str) -> bool {
    path.split(['\\', '/']).any(|segment| segment == "..")
}

/// `sha256:<64 lowercase hex>`.
fn is_sha256_digest(value: &str) -> bool {
    match value.strip_prefix("sha256:") {
        Some(hex) => {
            hex.len() == 64
                && hex
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        }
        None => false,
    }
}

/// Resolve one object type's `data_schema` per FR-073.
///
/// `semantic_block_present` gates the FR-073 migration warning: an inline
/// schema is only advisory under a module that has opted into the semantic
/// contract.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn resolve_data_schema(
    value: &Value,
    ctx: &ResolveContext,
    semantic_block_present: bool,
) -> ResolvedDataSchema {
    let locus = ctx.locus();
    let (form, classified) = classify_data_schema(value, &locus);
    match form {
        DataSchemaForm::Invalid => {
            return ResolvedDataSchema {
                kind: DataSchemaForm::Inline,
                schema: None,
                file: None,
                diagnostics: classified,
            };
        }
        DataSchemaForm::Inline => {
            let diagnostics = if semantic_block_present {
                vec![SemanticDiagnostic::warning(
                    DiagnosticCode::InlineDataSchema,
                    &locus,
                    format!(
                        "object type {} still carries an inline data_schema; migrate to \
                         {{ schema, digest }} referencing the module's emitted schema \
                         (FR-073, FR-074)",
                        ctx.object_type
                    ),
                )]
            } else {
                Vec::new()
            };
            return ResolvedDataSchema {
                kind: DataSchemaForm::Inline,
                schema: Some(value.clone()),
                file: None,
                diagnostics,
            };
        }
        DataSchemaForm::Reference => {}
    }

    let mut diagnostics: Vec<SemanticDiagnostic> = Vec::new();
    let reference = value
        .as_object()
        .map_or(&serde_json::Map::new(), |m| m)
        .clone();

    let fail = |diagnostics: Vec<SemanticDiagnostic>| ResolvedDataSchema {
        kind: DataSchemaForm::Reference,
        schema: None,
        file: None,
        diagnostics,
    };

    let schema_path = reference
        .get("schema")
        .and_then(Value::as_str)
        .unwrap_or("");
    if schema_path.is_empty() {
        diagnostics.push(SemanticDiagnostic::error(
            DiagnosticCode::DataSchemaPath,
            format!("{locus}.schema"),
            "schema must be a module-relative path",
        ));
        return fail(diagnostics);
    }

    let digest_value = reference.get("digest").cloned().unwrap_or(Value::Null);
    let digest_text = digest_value
        .as_str()
        .map_or_else(|| stringify_like_js(&digest_value), str::to_owned);
    if !is_sha256_digest(&digest_text) {
        diagnostics.push(SemanticDiagnostic::error(
            DiagnosticCode::DataSchemaDigest,
            format!("{locus}.digest"),
            format!("digest must be sha256:<64 hex>, got {digest_text}"),
        ));
        return fail(diagnostics);
    }

    if Path::new(schema_path).is_absolute() || has_parent_segment(schema_path) {
        diagnostics.push(SemanticDiagnostic::error(
            DiagnosticCode::DataSchemaEscape,
            format!("{locus}.schema"),
            format!("schema path escapes the module root: {schema_path}"),
        ));
        return fail(diagnostics);
    }

    let file = ctx.module_root.join(schema_path);
    if !file.exists() {
        diagnostics.push(SemanticDiagnostic::error(
            DiagnosticCode::DataSchemaMissing,
            format!("{locus}.schema"),
            format!("schema file is missing: {schema_path}"),
        ));
        return fail(diagnostics);
    }
    if !inside_root(&ctx.module_root, &file) {
        diagnostics.push(SemanticDiagnostic::error(
            DiagnosticCode::DataSchemaEscape,
            format!("{locus}.schema"),
            format!("schema path escapes the module root by symlink: {schema_path}"),
        ));
        return fail(diagnostics);
    }

    let bytes = match std::fs::read(&file) {
        Ok(bytes) => bytes,
        Err(error) => {
            diagnostics.push(SemanticDiagnostic::error(
                DiagnosticCode::DataSchemaUnreadable,
                format!("{locus}.schema"),
                format!("schema file is unreadable: {schema_path} ({error})"),
            ));
            return fail(diagnostics);
        }
    };

    let actual = sha256_hex(&bytes);
    if actual != digest_text {
        diagnostics.push(SemanticDiagnostic::error(
            DiagnosticCode::DataSchemaDigestMismatch,
            format!("{locus}.digest"),
            format!("schema {schema_path} hashes to {actual}, manifest records {digest_text}"),
        ));
        return fail(diagnostics);
    }

    let Ok(parsed) = serde_json::from_slice::<Value>(&bytes) else {
        diagnostics.push(SemanticDiagnostic::error(
            DiagnosticCode::DataSchemaNotJson,
            format!("{locus}.schema"),
            format!("schema file is not JSON: {schema_path}"),
        ));
        return fail(diagnostics);
    };

    let declared_id = parsed.as_object().and_then(|m| m.get("$id"));
    let Some(Value::String(id)) = declared_id else {
        diagnostics.push(SemanticDiagnostic::error(
            DiagnosticCode::DataSchemaNotSchema,
            format!("{locus}.schema"),
            format!(
                "schema file is not a JSON Schema document with an absolute $id: {schema_path}"
            ),
        ));
        return fail(diagnostics);
    };

    let base = ctx.package_base();
    let leaf = schema_path.rsplit('/').next().unwrap_or(schema_path);
    let expected_id = format!("{base}{leaf}");
    if id != &expected_id {
        diagnostics.push(SemanticDiagnostic::error(
            DiagnosticCode::DataSchemaId,
            format!("{locus}.schema"),
            format!("schema $id is {id}, expected {expected_id}"),
        ));
        return fail(diagnostics);
    }

    let dialect = parsed.as_object().and_then(|m| m.get("$schema"));
    if dialect.and_then(Value::as_str) != Some(DIALECT_2020_12) {
        diagnostics.push(SemanticDiagnostic::error(
            DiagnosticCode::DataSchemaNotSchema,
            format!("{locus}.schema"),
            format!("schema file does not declare JSON Schema 2020-12: {schema_path}"),
        ));
        return fail(diagnostics);
    }

    let mut walker = RefWalker {
        ctx,
        locus: &locus,
        base: &base,
        core_base: ctx.core_base(),
        schema_dir: file.parent().unwrap_or(&ctx.module_root).to_path_buf(),
        visited: BTreeSet::new(),
        stack: Vec::new(),
        diagnostics: Vec::new(),
    };
    walker.walk(&parsed, &format!("module/{leaf}"));
    diagnostics.extend(walker.diagnostics);

    ResolvedDataSchema {
        kind: DataSchemaForm::Reference,
        schema: Some(parsed),
        file: Some(file),
        diagnostics,
    }
}

/// `String(value)` for the values `digest` can hold, so the refusal message
/// reads the way the TypeScript's does.
fn stringify_like_js(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Walks `$ref` edges within the shipped bundle and the vendored semantic-core
/// bundle, refusing anything that leaves them.
struct RefWalker<'a> {
    ctx: &'a ResolveContext,
    locus: &'a str,
    base: &'a str,
    core_base: String,
    schema_dir: PathBuf,
    visited: BTreeSet<String>,
    stack: Vec<String>,
    diagnostics: Vec<SemanticDiagnostic>,
}

impl RefWalker<'_> {
    fn push(&mut self, code: DiagnosticCode, message: String) {
        self.diagnostics.push(SemanticDiagnostic::error(
            code,
            format!("{}.schema", self.locus),
            message,
        ));
    }

    fn walk(&mut self, document: &Value, file_key: &str) -> bool {
        let self_id = document
            .as_object()
            .and_then(|m| m.get("$id"))
            .and_then(Value::as_str)
            .map(str::to_owned);

        if self.stack.iter().any(|k| k == file_key) {
            let mut chain = self.stack.clone();
            chain.push(file_key.to_owned());
            self.push(
                DiagnosticCode::SchemaRefCycle,
                format!("$ref cycle: {}", chain.join(" -> ")),
            );
            return false;
        }
        if !self.visited.insert(file_key.to_owned()) {
            return true;
        }
        self.stack.push(file_key.to_owned());

        let mut refs = Vec::new();
        collect_refs(document, &mut refs);

        for target in refs {
            let url = target.split('#').next().unwrap_or("");
            if url.is_empty() || Some(url) == self_id.as_deref() {
                continue; // a fragment within this document
            }
            if let Some(name) = url.strip_prefix(self.core_base.as_str()) {
                let core_path = self.ctx.semantic_core_dir.join(name);
                if !core_path.exists() {
                    self.push(
                        DiagnosticCode::SchemaRefUnshipped,
                        format!(
                            "$ref {url} names no file in the vendored semantic-core {} bundle",
                            self.ctx.semantic_core
                        ),
                    );
                    continue;
                }
                let Ok(bytes) = std::fs::read(&core_path) else {
                    self.push(
                        DiagnosticCode::SchemaRefUnshipped,
                        format!(
                            "$ref {url} names no file in the vendored semantic-core {} bundle",
                            self.ctx.semantic_core
                        ),
                    );
                    continue;
                };
                let Ok(document) = serde_json::from_slice::<Value>(&bytes) else {
                    self.push(
                        DiagnosticCode::DataSchemaNotJson,
                        format!("referenced file is not JSON: {name}"),
                    );
                    continue;
                };
                if !self.walk(&document, &format!("semantic-core/{name}")) {
                    return false;
                }
            } else if let Some(rest) = url.strip_prefix(SEMANTIC_CORE_BASE) {
                let version = rest.split('/').next().unwrap_or("");
                self.push(
                    DiagnosticCode::SchemaRefVersion,
                    format!(
                        "$ref {url} names semantic-core {version}, manifest records {}",
                        self.ctx.semantic_core
                    ),
                );
            } else if let Some(name) = url.strip_prefix(self.base) {
                let sibling = self.schema_dir.join(name);
                if !sibling.exists() || !inside_root(&self.ctx.module_root, &sibling) {
                    self.push(
                        DiagnosticCode::SchemaRefUnshipped,
                        format!("$ref {url} names no shipped file ({name})"),
                    );
                    continue;
                }
                let Some(document) = std::fs::read(&sibling)
                    .ok()
                    .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
                else {
                    self.push(
                        DiagnosticCode::DataSchemaNotJson,
                        format!("referenced file is not JSON: {name}"),
                    );
                    continue;
                };
                if !self.walk(&document, &format!("module/{name}")) {
                    return false;
                }
            } else {
                self.push(
                    DiagnosticCode::SchemaRefUnshipped,
                    format!("$ref {url} is outside the module bundle and the semantic-core bundle"),
                );
            }
        }

        self.stack.pop();
        true
    }
}

/// The versions this quoin ships a semantic-core bundle for, for a caller that
/// wants to report them.
#[must_use]
pub fn shipped_semantic_core_versions() -> &'static [&'static str] {
    SEMANTIC_CONTRACT.semantic_core_versions
}

#[cfg(test)]
// Indexing and `unreachable!` are a test-only convenience: an out-of-range
// index in a test is a failing test, not a downed worker.
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Trace: FR-073
    #[test]
    fn tc_378_050_digest_form_is_exactly_sha256_64_lowercase_hex() {
        assert!(is_sha256_digest(&format!("sha256:{}", "a".repeat(64))));
        assert!(!is_sha256_digest(&format!("sha256:{}", "A".repeat(64))));
        assert!(!is_sha256_digest(&format!("sha256:{}", "a".repeat(63))));
        assert!(!is_sha256_digest(&format!("sha1:{}", "a".repeat(64))));
        assert!(!is_sha256_digest(&format!("sha256:{}", "g".repeat(64))));
    }

    /// Trace: FR-073
    #[test]
    fn tc_378_051_parent_segments_are_caught_under_either_separator() {
        assert!(has_parent_segment("../a.json"));
        assert!(has_parent_segment("a/../b.json"));
        assert!(has_parent_segment("a\\..\\b.json"));
        assert!(!has_parent_segment("a/..b.json"));
        assert!(!has_parent_segment("schemas/Entity.json"));
    }

    /// Trace: FR-073
    #[test]
    fn tc_378_052_collect_refs_walks_arrays_and_nested_objects() {
        let mut refs = Vec::new();
        collect_refs(
            &json!({"a": {"$ref": "one"}, "b": [{"$ref": "two"}, {"c": {"$ref": "three"}}]}),
            &mut refs,
        );
        refs.sort();
        assert_eq!(refs, vec!["one", "three", "two"]);
    }

    /// Trace: FR-073
    #[test]
    fn tc_378_053_a_ref_that_is_not_a_string_is_not_collected() {
        let mut refs = Vec::new();
        collect_refs(&json!({"$ref": 7}), &mut refs);
        assert!(refs.is_empty());
    }
}
