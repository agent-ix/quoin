// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Agent-IX

//! Declared vocabulary coverage, read from module data (FR-037).
//!
//! quire-rs FR-059 computes which declared values no document claims. This reads
//! the *same declaration* the engine reads, so quoin can say what a finding
//! means — which vocabulary, how many values it has, and where a justified
//! absence may be recorded — without minting a second list.
//!
//! Minting one is the failure this whole area exists to avoid: the 25010
//! characteristic set is **12 values in module data**, and the original ticket
//! proposed walking a hardcoded 9.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::ids::{ArtifactTypeName, FrontmatterField, VocabularyName};

/// One `traceability.vocabulary_coverage` entry, with its values resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VocabularyDeclaration {
    /// The declaration's own name, e.g. `quality-characteristics`.
    pub name: VocabularyName,
    /// Artifact type the projection reads, e.g. `NFR`.
    pub from: ArtifactTypeName,
    /// Frontmatter field carrying the claim, e.g. `quality_attribute`.
    pub field: FrontmatterField,
    /// Corpus-check token the engine reports under.
    pub check: String,
    /// Frontmatter field recording a deliberate non-applicability, if declared.
    pub justified_absence_field: Option<String>,
    /// The declared vocabulary, read from the artifact type's frontmatter schema.
    pub values: Vec<String>,
    /// Module that declared it, for collision reporting.
    pub module_name: String,
}

/// A declaration whose vocabulary could not be resolved, and why.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct UnresolvedDeclaration {
    /// The declaration's name.
    pub name: VocabularyName,
    /// Why it could not be resolved.
    pub reason: String,
}

/// Every module's declared vocabulary coverage.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VocabularyDeclarations {
    /// Declarations whose vocabulary resolved.
    pub declarations: Vec<VocabularyDeclaration>,
    /// Declarations whose vocabulary could not be resolved, and why.
    pub unresolved: Vec<UnresolvedDeclaration>,
}

/// The module root for a candidate path: the path itself if it holds a
/// `manifest.yaml`, otherwise its first child that does.
///
/// **This is a deliberate, bounded duplicate of `src/catalog.ts`'s
/// `locateModuleRoot`.** The catalog is EPIC #373 Stage 7 and this crate is
/// Stage 3, so porting it here in full would take a dependency on a crate that
/// does not exist yet, and taking already-located roots as input would change
/// the API this crate is meant to be parity-tested against. The successor is the
/// catalog crate; when it lands, this function is deleted and the caller passes
/// its result (NFR-024: every retained path has a successor).
#[must_use]
pub fn locate_module_root(candidate: &Path) -> Option<PathBuf> {
    let root = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(candidate)
    };
    if !root.exists() {
        return None;
    }
    if root.join("manifest.yaml").exists() {
        return Some(root);
    }
    if !root.is_dir() {
        return None;
    }
    let mut names: Vec<std::ffi::OsString> = std::fs::read_dir(&root)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.file_name())
        .collect();
    // `readdirSync` order is filesystem order in Node and unspecified here, so
    // sort: two children with a manifest would otherwise resolve differently on
    // two machines. The TypeScript takes the first in readdir order, which is
    // the same answer whenever at most one child is a module — the only case
    // this is called with.
    names.sort();
    names
        .into_iter()
        .map(|name| root.join(name))
        .find(|child| child.join("manifest.yaml").exists())
}

/// Load every module's declared vocabulary coverage.
///
/// A module declaring none contributes none. A declaration whose values cannot
/// be resolved is **reported, not dropped** — it is the case where the engine
/// emits findings quoin cannot explain, and silence there is worse than an
/// empty list.
///
/// An unreadable or unparseable module is skipped: that is the advisor's
/// diagnostic to report, and a module that cannot be parsed declares no
/// coverage, which is the honest reading here.
#[must_use]
pub fn load_vocabulary_coverage(module_roots: &[PathBuf]) -> VocabularyDeclarations {
    let mut out = VocabularyDeclarations::default();
    let mut seen_roots: BTreeSet<PathBuf> = BTreeSet::new();

    for candidate in module_roots {
        let Some(module_root) = locate_module_root(candidate) else {
            continue;
        };
        if !seen_roots.insert(module_root.clone()) {
            continue;
        }

        let Ok(text) = std::fs::read_to_string(module_root.join("manifest.yaml")) else {
            continue;
        };
        let Ok(manifest) = serde_norway::from_str::<Value>(&text) else {
            continue;
        };
        let Some(manifest_map) = manifest.as_object() else {
            continue;
        };

        let module_name = manifest_map
            .get("name")
            .and_then(Value::as_str)
            .map_or_else(|| module_root.to_string_lossy().into_owned(), str::to_owned);

        let Some(entries) = manifest_map
            .get("traceability")
            .and_then(Value::as_object)
            .and_then(|t| t.get("vocabulary_coverage"))
            .and_then(Value::as_array)
        else {
            continue;
        };

        for raw in entries {
            let name = VocabularyName::new(string_at(raw, "name"));
            let from = string_at(raw, "from");
            let field = string_at(raw, "field");
            match enum_for(&manifest, &module_root, &from, &field) {
                Ok(values) => out.declarations.push(VocabularyDeclaration {
                    name,
                    from: ArtifactTypeName::new(from),
                    field: FrontmatterField::new(field),
                    check: string_at(raw, "check"),
                    justified_absence_field: raw
                        .get("justified_absence_field")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    values,
                    module_name: module_name.clone(),
                }),
                Err(reason) => out.unresolved.push(UnresolvedDeclaration { name, reason }),
            }
        }
    }
    out
}

/// `String(raw[key] ?? "")` for the scalar shapes a manifest carries.
fn string_at(value: &Value, key: &str) -> String {
    match value.get(key) {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// The declared enum for `<artifact_type>.<field>`, or a reason it is
/// unavailable.
///
/// The vocabulary lives in the artifact type's frontmatter schema — the same
/// place the engine reads it — so the two cannot disagree about how many values
/// exist. A field with no `enum` is an **open** vocabulary: coverage over it is
/// not a finite question, and reporting one would invent a denominator.
fn enum_for(
    manifest: &Value,
    module_root: &Path,
    artifact_type: &str,
    field: &str,
) -> Result<Vec<String>, String> {
    let Some(types) = manifest.get("artifact_types").and_then(Value::as_array) else {
        return Err("module declares no artifact_types".to_owned());
    };
    let Some(declared) = types
        .iter()
        .find(|t| t.get("name").and_then(Value::as_str).unwrap_or_default() == artifact_type)
    else {
        return Err(format!("no artifact type '{artifact_type}' in this module"));
    };
    let Some(reference) = declared
        .get("frontmatter_schema_ref")
        .and_then(Value::as_str)
    else {
        return Err(format!(
            "artifact type '{artifact_type}' declares no frontmatter schema"
        ));
    };

    let path = module_root.join(reference);
    let schema: Value = match std::fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str(&text) {
            Ok(value) => value,
            Err(cause) => {
                return Err(format!(
                    "frontmatter schema '{reference}' unreadable: {cause}"
                ))
            }
        },
        Err(cause) => {
            return Err(format!(
                "frontmatter schema '{reference}' unreadable: {cause}"
            ))
        }
    };

    let property = schema
        .get("properties")
        .and_then(Value::as_object)
        .and_then(|properties| properties.get(field));
    let Some(property) = property else {
        return Err(format!(
            "schema '{reference}' declares no property '{field}'"
        ));
    };
    let Some(values) = property.get("enum").and_then(Value::as_array) else {
        return Err(format!(
            "property '{field}' declares no enum, so its vocabulary is open"
        ));
    };
    Ok(values
        .iter()
        .map(|value| match value {
            Value::String(s) => s.clone(),
            Value::Null => "null".to_owned(),
            other => other.to_string(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trace: FR-037
    #[test]
    fn tc_378_220_an_absent_candidate_locates_nothing() {
        assert_eq!(locate_module_root(Path::new("/definitely/not/here")), None);
    }

    /// Trace: FR-037
    #[test]
    fn tc_378_221_a_root_with_a_manifest_locates_itself() {
        let Ok(temp) = tempfile::tempdir() else {
            unreachable!("tempdir");
        };
        let _ = std::fs::write(temp.path().join("manifest.yaml"), "name: x\n");
        assert_eq!(
            locate_module_root(temp.path()).as_deref(),
            Some(temp.path())
        );
    }

    /// Trace: FR-037
    #[test]
    fn tc_378_222_a_root_whose_child_has_a_manifest_locates_the_child() {
        let Ok(temp) = tempfile::tempdir() else {
            unreachable!("tempdir");
        };
        let child = temp.path().join("iso");
        let _ = std::fs::create_dir_all(&child);
        let _ = std::fs::write(child.join("manifest.yaml"), "name: x\n");
        assert_eq!(
            locate_module_root(temp.path()).as_deref(),
            Some(child.as_path())
        );
    }

    /// Trace: FR-037
    #[test]
    fn tc_378_223_a_module_declaring_no_coverage_contributes_none() {
        let Ok(temp) = tempfile::tempdir() else {
            unreachable!("tempdir");
        };
        let _ = std::fs::write(
            temp.path().join("manifest.yaml"),
            "name: x\nversion: 1.0.0\n",
        );
        let loaded = load_vocabulary_coverage(&[temp.path().to_path_buf()]);
        assert!(loaded.declarations.is_empty());
        assert!(loaded.unresolved.is_empty());
    }

    /// Trace: FR-037
    #[test]
    fn tc_378_224_the_same_root_listed_twice_is_read_once() {
        let Ok(temp) = tempfile::tempdir() else {
            unreachable!("tempdir");
        };
        let _ = std::fs::write(
            temp.path().join("manifest.yaml"),
            "name: x\ntraceability:\n  vocabulary_coverage:\n  - name: v\n    from: NFR\n    field: f\n",
        );
        let root = temp.path().to_path_buf();
        let loaded = load_vocabulary_coverage(&[root.clone(), root]);
        assert_eq!(loaded.unresolved.len(), 1, "{loaded:?}");
    }
}
