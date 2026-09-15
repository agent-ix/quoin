// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The installed specification-module catalog (quoin#373, Stage 8).
//!
//! This crate owns the projection from module manifests to the artifact/object
//! type catalog. Filesystem discovery deliberately does not live here: callers
//! hand it [`ModuleDocument`] values after resolving candidate roots, so the
//! projection is deterministic, testable, and usable by both the boundary and
//! future Rust authoring code. The input is parsed with [`quoin_yaml`], the
//! same YAML 1.2 reader that supplies the rest of Quoin's Rust port.

use std::collections::{BTreeMap, BTreeSet};

use quoin_semantic::{SemanticBlock, SemanticDiagnostic};
use serde::Serialize;
use serde_json::Value;

/// One already-located module document supplied by a filesystem host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleDocument {
    /// The resolved module root. Root spelling is retained for output parity.
    pub root: String,
    /// The text of `<root>/manifest.yaml`.
    pub manifest: String,
    /// Direct entries of `<root>/skeletons`, or an empty list when it is absent.
    pub skeleton_names: Vec<String>,
}

/// A semantic result supplied separately because semantic validation owns its
/// own vendored schema capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticView {
    /// The module root this result answers.
    pub root: String,
    /// The parsed semantic block when valid.
    pub block: Option<SemanticBlock>,
    /// Diagnostics raised while reading the semantic block.
    pub diagnostics: Vec<SemanticDiagnostic>,
}

/// The complete catalog projection.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    /// The active modules, in candidate-root order after duplicate suppression.
    pub modules: Vec<SpecModule>,
    /// Artifact and object entries, in manifest order.
    pub entries: Vec<SpecCatalogEntry>,
    /// Type names supplied by more than one module.
    pub duplicates: Vec<Duplicate>,
}

/// One module represented by the catalog.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct SpecModule {
    /// Manifest name, or the root basename when it is absent/non-string.
    pub name: String,
    /// Manifest version when it is a string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// The resolved module root.
    pub root: String,
    /// Declared artifact-type names.
    pub artifact_types: Vec<String>,
    /// Declared object-type names.
    pub object_types: Vec<String>,
    /// The parsed semantic block, when one passed semantic validation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic: Option<SemanticBlock>,
    /// Semantic diagnostics observed while loading this module.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_diagnostics: Option<Vec<SemanticDiagnostic>>,
}

/// One artifact or object type supplied by a module.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct SpecCatalogEntry {
    /// The artifact/object name.
    pub name: String,
    /// Which kind of type this entry is.
    pub kind: EntryKind,
    /// The module declaring this entry.
    pub module_name: String,
    /// The declaring module root.
    pub module_root: String,
    /// The artifact frontmatter schema reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_ref: Option<String>,
    /// The artifact schema path, derived from [`Self::schema_ref`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_path: Option<String>,
    /// A real skeleton filename with disk-accurate casing, when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skeleton_path: Option<String>,
    /// The raw object `data_schema` value, preserving inline JSON and refs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_schema: Option<Value>,
}

/// The two catalog namespaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    /// A Markdown artifact type.
    Artifact,
    /// A structured object type.
    Object,
}

/// A type name supplied by multiple modules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct Duplicate {
    /// The namespace in which the collision occurred.
    pub kind: EntryKind,
    /// The duplicate name, preserving its first declaration spelling.
    pub name: String,
    /// Module names that supplied it, sorted as TypeScript did.
    pub modules: Vec<String>,
}

/// A document could not be projected into the catalog.
#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    /// A `manifest.yaml` was not readable as one YAML 1.2 document.
    #[error("manifest at {root} is not valid YAML: {detail}")]
    ManifestYaml {
        /// The module root whose document failed.
        root: String,
        /// The YAML reader's detail.
        detail: String,
    },
}

/// Build a catalog from located manifests and already-read skeleton directories.
///
/// Duplicate roots and duplicate declared module names are ignored after their
/// first occurrence, matching `loadCatalog`. Semantic answers pair by root,
/// not position, so a host may return them in any order.
///
/// # Errors
/// Returns [`CatalogError::ManifestYaml`] when a retained module manifest is
/// not a YAML document the TypeScript `yaml` reader would accept.
pub fn build(
    documents: &[ModuleDocument],
    semantics: &[SemanticView],
) -> Result<Catalog, CatalogError> {
    let semantic_by_root: BTreeMap<&str, &SemanticView> = semantics
        .iter()
        .map(|view| (view.root.as_str(), view))
        .collect();
    let mut seen_roots = BTreeSet::new();
    let mut seen_names = BTreeSet::new();
    let mut modules = Vec::new();
    let mut entries = Vec::new();

    for document in documents {
        if !seen_roots.insert(document.root.as_str()) {
            continue;
        }
        let manifest = quoin_yaml::from_str(&document.manifest).map_err(|error| {
            CatalogError::ManifestYaml {
                root: document.root.clone(),
                detail: error.to_string(),
            }
        })?;
        let Some(manifest) = manifest.as_object() else {
            continue;
        };
        let name = string_field(manifest, "name").unwrap_or_else(|| basename(&document.root));
        if !seen_names.insert(name.clone()) {
            continue;
        }
        let artifacts = named_entries(manifest.get("artifact_types"));
        let objects = named_entries(manifest.get("object_types"));
        let semantic = semantic_by_root.get(document.root.as_str()).copied();

        modules.push(SpecModule {
            name: name.clone(),
            version: string_field(manifest, "version"),
            root: document.root.clone(),
            artifact_types: artifacts.iter().map(|entry| entry_name(entry)).collect(),
            object_types: objects.iter().map(|entry| entry_name(entry)).collect(),
            semantic: semantic.and_then(|view| view.block.clone()),
            semantic_diagnostics: semantic
                .and_then(|view| (!view.diagnostics.is_empty()).then(|| view.diagnostics.clone())),
        });

        for artifact in artifacts {
            let entry_name = entry_name(artifact);
            let schema_ref = string_field(artifact, "frontmatter_schema_ref");
            entries.push(SpecCatalogEntry {
                name: entry_name.clone(),
                kind: EntryKind::Artifact,
                module_name: name.clone(),
                module_root: document.root.clone(),
                schema_path: schema_ref
                    .as_ref()
                    .map(|reference| join(&document.root, reference)),
                schema_ref,
                skeleton_path: skeleton_path(&document.root, &entry_name, &document.skeleton_names),
                data_schema: None,
            });
        }
        for object in objects {
            let entry_name = entry_name(object);
            entries.push(SpecCatalogEntry {
                name: entry_name.clone(),
                kind: EntryKind::Object,
                module_name: name.clone(),
                module_root: document.root.clone(),
                schema_ref: None,
                schema_path: None,
                skeleton_path: skeleton_path(&document.root, &entry_name, &document.skeleton_names),
                data_schema: object.get("data_schema").cloned(),
            });
        }
    }

    Ok(Catalog {
        duplicates: find_duplicates(&entries),
        modules,
        entries,
    })
}

/// Find the first entry whose name matches `name` case-insensitively.
#[must_use]
pub fn find_entry<'a>(catalog: &'a Catalog, name: &str) -> Option<&'a SpecCatalogEntry> {
    catalog
        .entries
        .iter()
        .find(|entry| entry.name.to_lowercase() == name.to_lowercase())
}

fn named_entries(value: Option<&Value>) -> Vec<&serde_json::Map<String, Value>> {
    value
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |entries| {
            entries
                .iter()
                .filter_map(Value::as_object)
                .filter(|entry| entry.contains_key("name"))
                .collect()
        })
}

fn string_field(object: &serde_json::Map<String, Value>, name: &str) -> Option<String> {
    object.get(name).and_then(Value::as_str).map(str::to_owned)
}

fn entry_name(entry: &serde_json::Map<String, Value>) -> String {
    entry.get("name").map(js_string).unwrap_or_default()
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

fn basename(root: &str) -> String {
    root.rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .unwrap_or(root)
        .to_owned()
}

fn join(root: &str, child: &str) -> String {
    // Node's `path.join` both retains the parent for an absolute-looking
    // child and normalizes lexical `.` / `..` components.  Catalog paths are
    // intentionally display paths, so this must not resolve symlinks.
    let absolute = root.starts_with(['/', '\\']);
    let mut parts = Vec::new();
    for part in root
        .trim_matches(['/', '\\'])
        .split(['/', '\\'])
        .chain(child.trim_matches(['/', '\\']).split(['/', '\\']))
    {
        match part {
            "" | "." => {}
            ".." => {
                let _ = parts.pop();
            }
            part => parts.push(part),
        }
    }
    let joined = parts.join("/");
    if absolute {
        format!("/{joined}")
    } else {
        joined
    }
}

fn skeleton_path(root: &str, type_name: &str, names: &[String]) -> Option<String> {
    let exact = format!("{type_name}.md");
    let lower = format!("{}.md", type_name.to_lowercase());
    names
        .iter()
        .find(|name| **name == exact)
        .or_else(|| names.iter().find(|name| **name == lower))
        .map(|name| join(&join(root, "skeletons"), name))
}

fn find_duplicates(entries: &[SpecCatalogEntry]) -> Vec<Duplicate> {
    // A `BTreeMap` would make output deterministic, but not compatible: the
    // TypeScript implementation used a `Map`, so duplicate groups were emitted
    // in the order their first type declaration appeared. Keep that observable
    // declaration order while sorting only the MODULE names, as it did.
    let mut grouped: Vec<(EntryKind, String, Vec<String>)> = Vec::new();
    for entry in entries {
        if let Some((_, _, modules)) = grouped
            .iter_mut()
            .find(|(kind, name, _)| *kind == entry.kind && *name == entry.name)
        {
            if !modules.contains(&entry.module_name) {
                modules.push(entry.module_name.clone());
            }
        } else {
            grouped.push((
                entry.kind,
                entry.name.clone(),
                vec![entry.module_name.clone()],
            ));
        }
    }
    grouped
        .into_iter()
        .filter_map(|(kind, name, mut modules)| {
            (modules.len() > 1).then(|| {
                modules.sort_by(|left, right| left.encode_utf16().cmp(right.encode_utf16()));
                Duplicate {
                    kind,
                    name,
                    modules,
                }
            })
        })
        .collect()
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use serde_json::json;

    use super::{EntryKind, ModuleDocument, build, find_entry};

    /// Trace: FR-101
    #[test]
    fn tc_373_catalog_projects_manifest_types_and_real_skeleton_casing() {
        let catalog = build(
            &[ModuleDocument {
                root: "/modules/alpha".to_owned(),
                manifest: "name: alpha\nversion: 1.2.3\nartifact_types:\n  - name: FR\n    frontmatter_schema_ref: schemas/fr.json\nobject_types:\n  - name: Entity\n    data_schema: {type: object}\n".to_owned(),
                skeleton_names: vec!["FR.md".to_owned(), "entity.md".to_owned()],
            }],
            &[],
        )
        .expect("valid manifest");
        let [module] = catalog.modules.as_slice() else {
            panic!("the single manifest produces one module");
        };
        let [artifact, object] = catalog.entries.as_slice() else {
            panic!("the manifest produces one artifact and one object entry");
        };
        assert_eq!(module.artifact_types, ["FR"]);
        assert_eq!(
            artifact.schema_path.as_deref(),
            Some("/modules/alpha/schemas/fr.json")
        );
        assert_eq!(
            artifact.skeleton_path.as_deref(),
            Some("/modules/alpha/skeletons/FR.md")
        );
        assert_eq!(object.data_schema, Some(json!({"type": "object"})));
        assert_eq!(object.kind, EntryKind::Object);
    }

    /// Trace: FR-101
    #[test]
    fn tc_373_catalog_matches_node_path_and_skeleton_precedence() {
        let catalog = build(
            &[ModuleDocument {
                root: "/modules/alpha".to_owned(),
                manifest:
                    "artifact_types: [{name: Foo, frontmatter_schema_ref: ./schemas/../foo.json}]\n"
                        .to_owned(),
                skeleton_names: vec!["foo.md".to_owned(), "Foo.md".to_owned()],
            }],
            &[],
        )
        .expect("valid manifest");
        let Some(entry) = catalog.entries.first() else {
            panic!("the declared artifact is projected");
        };
        assert_eq!(
            entry.schema_path.as_deref(),
            Some("/modules/alpha/foo.json")
        );
        assert_eq!(
            entry.skeleton_path.as_deref(),
            Some("/modules/alpha/skeletons/Foo.md")
        );
    }

    /// Trace: FR-101
    #[test]
    fn tc_373_catalog_deduplicates_roots_names_and_reports_duplicate_types() {
        let document = |root: &str, name: &str| ModuleDocument {
            root: root.to_owned(),
            manifest: format!("name: {name}\nartifact_types: [{{name: FR}}]\n"),
            skeleton_names: Vec::new(),
        };
        let catalog = build(
            &[
                document("/modules/one", "one"),
                document("/modules/one", "ignored-root"),
                document("/modules/two", "two"),
                document("/modules/three", "two"),
            ],
            &[],
        )
        .expect("valid manifests");
        assert_eq!(catalog.modules.len(), 2);
        let [duplicate] = catalog.duplicates.as_slice() else {
            panic!("one duplicated artifact type is reported");
        };
        assert_eq!(duplicate.modules, ["one", "two"]);
        assert_eq!(
            find_entry(&catalog, "fr").map(|entry| entry.module_name.as_str()),
            Some("one")
        );
    }

    /// Trace: FR-101
    #[test]
    fn tc_373_catalog_duplicate_groups_keep_first_declaration_order() {
        let document = |root: &str, name: &str, types: &str| ModuleDocument {
            root: root.to_owned(),
            manifest: format!("name: {name}\n{types}"),
            skeleton_names: Vec::new(),
        };
        let catalog = build(
            &[
                document("/a", "a", "object_types: [{name: domain}]\n"),
                document("/b", "b", "artifact_types: [{name: FR}]\n"),
                document("/c", "c", "object_types: [{name: domain}]\n"),
                document("/d", "d", "artifact_types: [{name: FR}]\n"),
            ],
            &[],
        )
        .expect("valid manifests");
        assert_eq!(
            catalog
                .duplicates
                .iter()
                .map(|duplicate| (duplicate.kind, duplicate.name.as_str()))
                .collect::<Vec<_>>(),
            vec![(EntryKind::Object, "domain"), (EntryKind::Artifact, "FR")]
        );
    }

    /// Trace: FR-101
    #[test]
    fn tc_373_catalog_sorts_duplicate_module_names_as_javascript_does() {
        let document = |root: &str, name: &str| ModuleDocument {
            root: root.to_owned(),
            manifest: format!("name: {name}\nartifact_types: [{{name: FR}}]\n"),
            skeleton_names: Vec::new(),
        };
        let catalog = build(
            &[document("/one", "\u{e000}"), document("/two", "\u{10000}")],
            &[],
        )
        .expect("valid manifests");
        let [duplicate] = catalog.duplicates.as_slice() else {
            panic!("the shared artifact is reported once");
        };
        assert_eq!(duplicate.modules, ["\u{10000}", "\u{e000}"]);
    }
}
