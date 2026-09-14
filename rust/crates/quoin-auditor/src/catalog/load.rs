// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The first-wins merge of every module's `verification_catalog`.
//!
//! A port of `src/method-catalog.ts:795 loadMethodCatalog`. The merge is
//! first-wins by method id, matching quire-rs FR-054: if the two disagreed,
//! the advisor would recommend from one catalog while the auditor checked
//! conformance against another.

use std::collections::{BTreeMap, BTreeSet};

use quoin_combinatorial::js;
use serde_json::Value;

use super::method::{DuplicateMethod, MethodCatalog, UnreadableModule, VerificationMethod};
use super::source::{ModuleCatalogSource, ModuleRoot};
use crate::jsvalue::{string_of, string_or};

/// Load and merge every module's `verification_catalog`.
///
/// A module declaring none contributes none, and a catalog nobody declares is
/// an empty catalog rather than an error — a repository that has not adopted
/// the catalog can still run everything else.
///
/// Infallible by construction: an unreadable manifest is **data**, recorded in
/// [`MethodCatalog::unreadable`], because the command that would have crashed
/// is the one an operator runs to diagnose the module (agent-ix/quoin#106).
#[must_use]
pub fn load_method_catalog(source: &dyn ModuleCatalogSource) -> MethodCatalog {
    load_method_catalog_from(source, &source.default_roots())
}

/// [`load_method_catalog`] over an explicit candidate list.
#[must_use]
pub fn load_method_catalog_from(
    source: &dyn ModuleCatalogSource,
    candidates: &[ModuleRoot],
) -> MethodCatalog {
    // Insertion order is load-bearing for `duplicates[].modules` (first-wins
    // order), so these are ordered maps keyed by insertion, not by key: the
    // final report sorts by id, but the *contents* of a duplicate entry follow
    // module order.
    let mut methods: Vec<VerificationMethod> = Vec::new();
    let mut index: BTreeMap<String, usize> = BTreeMap::new();
    let mut collisions: Vec<(String, Vec<String>)> = Vec::new();
    let mut collision_index: BTreeMap<String, usize> = BTreeMap::new();
    let mut seen_roots: BTreeSet<String> = BTreeSet::new();
    let mut unreadable: Vec<UnreadableModule> = Vec::new();

    for candidate in candidates {
        let Some(module_root) = source.locate(candidate) else {
            continue;
        };
        if !seen_roots.insert(module_root.as_str().to_owned()) {
            continue;
        }

        let manifest = match read_manifest(source, &module_root) {
            Ok(manifest) => manifest,
            Err(reason) => {
                unreadable.push(UnreadableModule {
                    module_root: module_root.as_str().to_owned(),
                    reason,
                });
                continue;
            }
        };
        // `if (!manifest || typeof manifest !== "object") continue;` — a scalar
        // document, a list, or an empty file is not a manifest. `typeof null`
        // is `"object"` in JavaScript, which is why the falsy test comes first
        // and why `null` lands here rather than in the object branch.
        let Some(manifest) = manifest.as_object() else {
            continue;
        };
        let module_name = string_or(manifest.get("name"), module_root.as_str());
        let Some(catalog) = manifest
            .get("verification_catalog")
            .and_then(Value::as_object)
        else {
            continue;
        };

        for (id, raw) in catalog {
            if let Some(&existing) = index.get(id.as_str()) {
                // Reported rather than absorbed: two modules disagreeing about
                // what `mutation-testing` means is a collision an operator
                // must see.
                let listed = if let Some(&at) = collision_index.get(id.as_str()) {
                    at
                } else {
                    let winner = methods
                        .get(existing)
                        .map_or_else(String::new, |method| method.module_name.clone());
                    collisions.push((id.clone(), vec![winner]));
                    collision_index.insert(id.clone(), collisions.len() - 1);
                    collisions.len() - 1
                };
                if let Some(entry) = collisions.get_mut(listed) {
                    entry.1.push(module_name.clone());
                }
                continue;
            }
            methods.push(entry_of(id, raw, &module_name));
            index.insert(id.clone(), methods.len() - 1);
        }
    }

    // `byId`: plain `<`, not `localeCompare`. The merged catalog drives advice
    // and conformance, and ordering must not depend on the runtime's ICU data.
    methods.sort_by(|left, right| js::compare(&left.id, &right.id));
    let mut duplicates: Vec<DuplicateMethod> = collisions
        .into_iter()
        .map(|(id, modules)| DuplicateMethod { id, modules })
        .collect();
    duplicates.sort_by(|left, right| js::compare(&left.id, &right.id));
    unreadable.sort_by(|left, right| js::compare(&left.module_root, &right.module_root));

    MethodCatalog {
        methods,
        duplicates,
        unreadable,
    }
}

/// Read and parse one manifest, yielding the reader's message on failure.
fn read_manifest(source: &dyn ModuleCatalogSource, root: &ModuleRoot) -> Result<Value, String> {
    let text = source
        .read_manifest(root)
        .map_err(|cause| cause.to_string())?;
    quoin_yaml::from_str(&text).map_err(|cause| cause.to_string())
}

/// One `verification_catalog` entry, coerced exactly as the reader coerces it.
fn entry_of(id: &str, raw: &Value, module_name: &str) -> VerificationMethod {
    // `raw.name` on a non-object is `undefined` in JavaScript rather than a
    // throw, so a `verification_catalog` entry that is a scalar yields an
    // all-empty method rather than skipping. Retained behaviour, not an
    // improvement (FR-018).
    let fields = raw.as_object();
    let field = |key: &str| fields.and_then(|object| object.get(key));
    VerificationMethod {
        id: id.to_owned(),
        name: string_or(field("name"), ""),
        class: string_or(field("class"), ""),
        definition: string_or(field("definition"), ""),
        // `typeof raw.evidence_kind === "string" ? … : undefined` — a number
        // here is dropped, NOT coerced. The one field the reader refuses to
        // coerce, and the port must refuse it too.
        evidence_kind: field("evidence_kind")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        applicability: normalize_rules(field("applicability")),
        tooling: field("tooling")
            .and_then(Value::as_array)
            .map(|items| items.iter().map(string_of).collect())
            .unwrap_or_default(),
        module_name: module_name.to_owned(),
    }
}

/// `normalizeRules`: rule name → values, keeping only list-valued rules.
fn normalize_rules(value: Option<&Value>) -> BTreeMap<String, Vec<String>> {
    value
        .and_then(Value::as_object)
        .map(|rules| {
            rules
                .iter()
                .filter_map(|(rule, values)| {
                    values
                        .as_array()
                        .map(|items| (rule.clone(), items.iter().map(string_of).collect()))
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{load_method_catalog, load_method_catalog_from};
    use crate::catalog::source::{MemoryModuleCatalogSource, ModuleRoot};

    const ONE: &str = r"
name: alpha
verification_catalog:
  unit-testing:
    name: Unit testing
    class: Test
    definition: Exercise the unit.
    evidence_kind: test-run
    applicability:
      criticality: [high, medium]
      shape: not-a-list
    tooling: [jest, 7]
";

    const TWO: &str = r"
name: beta
verification_catalog:
  unit-testing:
    name: Something else
    class: Test
  analysis:
    name: Analysis
    class: Analysis
";

    #[test]
    fn the_first_module_to_declare_an_id_wins_and_the_collision_is_reported() {
        let source = MemoryModuleCatalogSource::new()
            .with_module("/one", ONE)
            .with_module("/two", TWO);
        let catalog = load_method_catalog(&source);
        assert_eq!(
            catalog
                .methods
                .iter()
                .map(|m| m.id.as_str())
                .collect::<Vec<_>>(),
            ["analysis", "unit-testing"],
            "methods sort by id regardless of which module contributed them"
        );
        let unit = &catalog.methods[1];
        assert_eq!(unit.name, "Unit testing", "the first module wins the entry");
        assert_eq!(unit.module_name, "alpha");
        assert_eq!(catalog.duplicates.len(), 1);
        assert_eq!(catalog.duplicates[0].id, "unit-testing");
        assert_eq!(
            catalog.duplicates[0].modules,
            ["alpha", "beta"],
            "the winner is listed first"
        );
    }

    #[test]
    fn non_list_rules_are_dropped_and_list_members_are_coerced() {
        let source = MemoryModuleCatalogSource::new().with_module("/one", ONE);
        let catalog = load_method_catalog(&source);
        let unit = &catalog.methods[0];
        assert_eq!(
            unit.applicability.get("criticality").unwrap(),
            &["high", "medium"]
        );
        assert!(
            !unit.applicability.contains_key("shape"),
            "a scalar rule is not an applicability axis"
        );
        assert_eq!(
            unit.tooling,
            ["jest", "7"],
            "a numeric tooling entry is String()-coerced, not dropped"
        );
    }

    #[test]
    fn an_unreadable_manifest_is_recorded_and_the_rest_of_the_merge_proceeds() {
        let source = MemoryModuleCatalogSource::new()
            .with_unreadable("/broken", "EISDIR: illegal operation")
            .with_module("/one", ONE)
            .with_missing("/gone");
        let catalog = load_method_catalog(&source);
        assert_eq!(
            catalog.unreadable.len(),
            1,
            "a candidate that resolves to nothing is not unreadable"
        );
        assert_eq!(catalog.unreadable[0].module_root, "/broken");
        assert_eq!(catalog.methods.len(), 1, "the readable module still merged");
    }

    #[test]
    fn malformed_yaml_is_recorded_rather_than_thrown() {
        let source = MemoryModuleCatalogSource::new().with_module("/bad", "a: [1,\nb: 2\n");
        let catalog = load_method_catalog(&source);
        assert_eq!(catalog.unreadable.len(), 1);
        assert!(catalog.methods.is_empty());
    }

    #[test]
    fn a_root_offered_twice_is_merged_once() {
        let source = MemoryModuleCatalogSource::new().with_module("/one", ONE);
        let twice = [ModuleRoot::candidate("/one"), ModuleRoot::candidate("/one")];
        let catalog = load_method_catalog_from(&source, &twice);
        assert!(
            catalog.duplicates.is_empty(),
            "a module cannot collide with itself"
        );
        assert_eq!(catalog.methods.len(), 1);
    }

    #[test]
    fn a_module_declaring_no_catalog_contributes_nothing_and_is_not_an_error() {
        let source = MemoryModuleCatalogSource::new()
            .with_module("/plain", "name: plain\n")
            .with_module("/scalar", "just-a-string\n");
        let catalog = load_method_catalog(&source);
        assert!(catalog.methods.is_empty());
        assert!(catalog.unreadable.is_empty());
    }

    #[test]
    fn an_evidence_kind_that_is_not_a_string_is_dropped_rather_than_coerced() {
        let source = MemoryModuleCatalogSource::new().with_module(
            "/one",
            "name: m\nverification_catalog:\n  m1:\n    evidence_kind: 7\n",
        );
        let catalog = load_method_catalog(&source);
        assert_eq!(catalog.methods[0].evidence_kind, None);
    }

    #[test]
    fn a_module_without_a_name_is_attributed_to_its_root() {
        let source = MemoryModuleCatalogSource::new()
            .with_module("/anon", "verification_catalog:\n  m1: {}\n");
        let catalog = load_method_catalog(&source);
        assert_eq!(catalog.methods[0].module_name, "/anon");
    }
}
