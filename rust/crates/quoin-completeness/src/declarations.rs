// SPDX-License-Identifier: AGPL-3.0-or-later
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

/// One module's frontmatter schema, as the CALLER read it.
///
/// `Unreadable` carries the caller's own message rather than being modelled as
/// an absent key, because the reason is what the user reads: "frontmatter
/// schema 'schemas/nfr.json' unreadable: ENOENT …" names the file and the OS
/// error, and an absent key could only ever produce a generic sentence.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum SchemaSource {
    /// The file's text.
    Text(String),
    /// Why the caller could not read it.
    Unreadable(String),
}

/// One module's text, as the CALLER read it.
///
/// The boundary's reason for existing (quoin#445): `quoin-core`'s library half
/// may not touch a filesystem, so the command shell locates the module, reads
/// `manifest.yaml`, asks [`schema_refs_of`] which schemas that manifest needs,
/// reads those, and sends all of it as bytes.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ModuleSource {
    /// What to call this module when its manifest declares no `name`. The
    /// filesystem shell passes the module root; the wire caller passes whatever
    /// it resolved, and the boundary never turns it back into a path.
    pub label: String,
    /// `manifest.yaml`, as text.
    pub manifest: String,
    /// Every `frontmatter_schema_ref` this manifest names, keyed by the ref
    /// exactly as the manifest spells it.
    pub schemas: std::collections::BTreeMap<String, SchemaSource>,
}

/// A declaration whose vocabulary could not be resolved, and why.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
    let mut sources: Vec<ModuleSource> = Vec::new();
    let mut seen_roots: BTreeSet<PathBuf> = BTreeSet::new();

    for candidate in module_roots {
        let Some(module_root) = locate_module_root(candidate) else {
            continue;
        };
        if !seen_roots.insert(module_root.clone()) {
            continue;
        }
        let Ok(manifest) = std::fs::read_to_string(module_root.join("manifest.yaml")) else {
            continue;
        };
        // Only the refs this manifest actually names. Reading every JSON file
        // under the module root would be a cheaper line here and a wider blast
        // radius on the wire, where the same set has to be sent.
        let schemas = schema_refs_of(&manifest)
            .into_iter()
            .map(|reference| {
                let text = std::fs::read_to_string(module_root.join(&reference));
                let source = match text {
                    Ok(text) => SchemaSource::Text(text),
                    Err(cause) => SchemaSource::Unreadable(cause.to_string()),
                };
                (reference, source)
            })
            .collect();
        sources.push(ModuleSource {
            label: module_root.to_string_lossy().into_owned(),
            manifest,
            schemas,
        });
    }

    declarations_from_sources(&sources)
}

/// Every `frontmatter_schema_ref` a manifest's `vocabulary_coverage` entries
/// reach for, deduplicated and in manifest order.
///
/// Split out so a caller with no filesystem access can be told which files to
/// read before it asks for an assessment (quoin#445). A manifest that declares
/// no coverage, or whose entries name no resolvable artifact type, needs none.
#[must_use]
pub fn schema_refs_of(manifest: &str) -> Vec<String> {
    let Ok(manifest) = quoin_yaml::from_str(manifest) else {
        return Vec::new();
    };
    let Some(entries) = manifest
        .get("traceability")
        .and_then(Value::as_object)
        .and_then(|traceability| traceability.get("vocabulary_coverage"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };
    let mut refs: Vec<String> = Vec::new();
    for raw in entries {
        // The SAME resolver `enum_for` uses, deliberately. These two are one
        // fact — "which schema does this artifact type's frontmatter live in" —
        // and when they were two copies, teaching one a fallback (a module-level
        // default ref, case-insensitive type matching) would leave the other
        // naming no file: the shell reads nothing, `enum_for` then finds no
        // content for the ref it did resolve, and every such declaration turns
        // `unresolved`. Both paths break identically, so the parity tests stay
        // green while the coverage silently empties.
        let Ok(reference) = schema_ref_for(&manifest, &string_at(raw, "from")) else {
            continue;
        };
        if !refs.iter().any(|seen| seen == &reference) {
            refs.push(reference);
        }
    }
    refs
}

/// The `frontmatter_schema_ref` a manifest declares for one artifact type, or
/// the reason it declares none.
///
/// The single source of truth for that lookup: [`schema_refs_of`] uses it to
/// decide which files a caller with no filesystem must read, and [`enum_for`]
/// uses it to decide which of those files the vocabulary came out of. A second
/// copy would let the file that is READ and the file that is LOOKED FOR drift
/// apart, and a declaration whose schema is absent is `unresolved` either way —
/// so neither the golden parity run nor the snapshot equivalence would see it.
fn schema_ref_for(manifest: &Value, artifact_type: &str) -> Result<String, String> {
    let Some(types) = manifest.get("artifact_types").and_then(Value::as_array) else {
        return Err("module declares no artifact_types".to_owned());
    };
    let Some(declared) = types
        .iter()
        .find(|t| t.get("name").and_then(Value::as_str).unwrap_or_default() == artifact_type)
    else {
        return Err(format!("no artifact type '{artifact_type}' in this module"));
    };
    declared
        .get("frontmatter_schema_ref")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("artifact type '{artifact_type}' declares no frontmatter schema"))
}

/// Resolve declared vocabulary coverage from module text the CALLER read.
///
/// The decidable half of [`load_vocabulary_coverage`], split out so it can cross
/// the `quoin-core` boundary without the library half acquiring a filesystem
/// (quoin#445). The filesystem shell above is the only other caller, so there is
/// one resolver and not two.
///
/// A module whose manifest does not parse, or is not a mapping, contributes
/// nothing — that is the advisor's diagnostic to report, and a module that
/// cannot be parsed declares no coverage.
#[must_use]
pub fn declarations_from_sources(modules: &[ModuleSource]) -> VocabularyDeclarations {
    let mut out = VocabularyDeclarations::default();

    for module in modules {
        let Ok(manifest) = quoin_yaml::from_str(&module.manifest) else {
            continue;
        };
        let Some(manifest_map) = manifest.as_object() else {
            continue;
        };

        let module_name = manifest_map
            .get("name")
            .and_then(Value::as_str)
            .map_or_else(|| module.label.clone(), str::to_owned);

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
            match enum_for(&manifest, &module.schemas, &from, &field) {
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
    schemas: &std::collections::BTreeMap<String, SchemaSource>,
    artifact_type: &str,
    field: &str,
) -> Result<Vec<String>, String> {
    let reference = schema_ref_for(manifest, artifact_type)?;
    let reference = reference.as_str();

    let schema: Value = match schemas.get(reference) {
        Some(SchemaSource::Text(text)) => match serde_json::from_str(text) {
            Ok(value) => value,
            Err(cause) => {
                return Err(format!(
                    "frontmatter schema '{reference}' unreadable: {cause}"
                ));
            }
        },
        Some(SchemaSource::Unreadable(cause)) => {
            return Err(format!(
                "frontmatter schema '{reference}' unreadable: {cause}"
            ));
        }
        // Unreachable through `load_vocabulary_coverage`, which drives its reads
        // from `schema_refs_of` over the same manifest. Reachable from the wire,
        // where the caller assembles the map itself — and a silent empty
        // vocabulary there would report full coverage over a denominator of
        // zero, which is the failure FR-037 exists to stop.
        None => {
            return Err(format!(
                "frontmatter schema '{reference}' unreadable: the caller supplied no content for it"
            ));
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

    /// Every reason a declaration can go unresolved, each said in its own
    /// words.
    ///
    /// FR-037-AC-2 has three branches and the golden corpus exercises two of
    /// them. The third — an artifact type that EXISTS but names no
    /// `frontmatter_schema_ref` — is the one a module hits while it is being
    /// written, and it is the branch a reader most needs told apart from "no
    /// such artifact type": the first is a module half-declared, the second a
    /// declaration pointing at nothing. Asserting the three reasons together
    /// is what makes them distinguishable; asserting only that the count is
    /// three would pass with one sentence repeated.
    ///
    /// Trace: FR-037-AC-2
    /// Provenance: agent-ix/quoin#445
    #[test]
    fn tc_445_230_an_artifact_type_with_no_schema_is_unresolved_in_its_own_words() {
        let manifest = concat!(
            "name: m\n",
            "artifact_types:\n",
            "- name: NFR\n",
            "traceability:\n",
            "  vocabulary_coverage:\n",
            "  - name: no-schema\n",
            "    from: NFR\n",
            "    field: characteristic\n",
            "  - name: no-such-type\n",
            "    from: Ghost\n",
            "    field: characteristic\n",
        );
        let loaded = declarations_from_sources(&[ModuleSource {
            label: "m".to_owned(),
            manifest: manifest.to_owned(),
            schemas: std::collections::BTreeMap::new(),
        }]);

        assert!(loaded.declarations.is_empty(), "{loaded:?}");
        let reasons: Vec<&str> = loaded
            .unresolved
            .iter()
            .map(|u| u.reason.as_str())
            .collect();
        assert_eq!(
            reasons,
            vec![
                "artifact type 'NFR' declares no frontmatter schema",
                "no artifact type 'Ghost' in this module",
            ],
            "{loaded:?}"
        );
    }

    /// A manifest with no `artifact_types` at all is a fourth, earlier reason.
    ///
    /// Trace: FR-037-AC-2
    /// Provenance: agent-ix/quoin#445
    #[test]
    fn tc_445_231_a_manifest_declaring_no_artifact_types_says_so() {
        let manifest = concat!(
            "name: m\n",
            "traceability:\n",
            "  vocabulary_coverage:\n",
            "  - name: v\n",
            "    from: NFR\n",
            "    field: characteristic\n",
        );
        let loaded = declarations_from_sources(&[ModuleSource {
            label: "m".to_owned(),
            manifest: manifest.to_owned(),
            schemas: std::collections::BTreeMap::new(),
        }]);
        let reasons: Vec<&str> = loaded
            .unresolved
            .iter()
            .map(|u| u.reason.as_str())
            .collect();
        assert_eq!(
            reasons,
            vec!["module declares no artifact_types"],
            "{loaded:?}"
        );
    }

    /// The files the shell is TOLD to read are the files the resolver LOOKS
    /// for.
    ///
    /// `schema_refs_of` and `enum_for` answer one question — which schema holds
    /// this artifact type's frontmatter — and the boundary is sound only while
    /// they answer it the same way. When they were two copies, teaching one a
    /// fallback and not the other left the shell reading nothing and the
    /// resolver reporting "the caller supplied no content for it" for every
    /// such declaration; both halves break together, so a parity run over a
    /// corpus where the fallback never fires stays green.
    ///
    /// The assertion is therefore not that the two return equal strings — that
    /// would restate the implementation — but that a caller which reads EXACTLY
    /// what `schema_refs_of` names can resolve every declaration.
    ///
    /// Trace: FR-037-AC-2
    /// Provenance: agent-ix/quoin#445
    #[test]
    fn tc_445_232_the_refs_the_shell_reads_are_the_refs_the_resolver_looks_for() {
        let manifest = concat!(
            "name: m\n",
            "artifact_types:\n",
            "- name: NFR\n",
            "  frontmatter_schema_ref: schemas/nfr.json\n",
            "- name: FR\n",
            "  frontmatter_schema_ref: schemas/fr.json\n",
            "traceability:\n",
            "  vocabulary_coverage:\n",
            "  - name: quality\n",
            "    from: NFR\n",
            "    field: characteristic\n",
            "  - name: kind\n",
            "    from: FR\n",
            "    field: characteristic\n",
        );

        // A shell with no filesystem knowledge: it reads what it is named, and
        // nothing else reaches the map.
        let schema = r#"{"properties":{"characteristic":{"enum":["a","b"]}}}"#;
        let refs = schema_refs_of(manifest);
        assert_eq!(refs.len(), 2, "{refs:?}");
        let schemas: std::collections::BTreeMap<String, SchemaSource> = refs
            .into_iter()
            .map(|reference| (reference, SchemaSource::Text(schema.to_owned())))
            .collect();

        let loaded = declarations_from_sources(&[ModuleSource {
            label: "m".to_owned(),
            manifest: manifest.to_owned(),
            schemas,
        }]);

        assert!(
            loaded.unresolved.is_empty(),
            "a declaration went unresolved over a map holding exactly what \
             `schema_refs_of` named: {loaded:?}"
        );
        assert_eq!(loaded.declarations.len(), 2, "{loaded:?}");
    }
}
