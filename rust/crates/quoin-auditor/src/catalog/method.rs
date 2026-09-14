// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The merged verification-method catalog, as both halves of this crate read it.

use std::collections::{BTreeMap, BTreeSet};

use quoin_combinatorial::js;
use serde::{Deserialize, Serialize};

/// One catalog entry, as the module declared it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationMethod {
    /// The method id, unique in the merged catalog (first module wins).
    pub id: String,
    /// The human name.
    pub name: String,
    /// IADT in practice; a free string to the engine, so a free string here.
    pub class: String,
    /// What discharging the method means.
    pub definition: String,
    /// The evidence kind the method produces, when the module declares one.
    ///
    /// Its absence makes method conformance **unanswerable** rather than
    /// failed: a run of an undeclared kind is not a mismatch
    /// (agent-ix/quoin#105).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_kind: Option<String>,
    /// Rule name → values.
    ///
    /// **Never interpreted structurally** — the advisor matches values, and
    /// which axes exist is the declaring module's business (quire-rs
    /// FR-054-CON-2). A rule naming an axis the advisor cannot observe is
    /// skipped, not failed.
    #[serde(default)]
    pub applicability: BTreeMap<String, Vec<String>>,
    /// Tools the module names for the method.
    #[serde(default)]
    pub tooling: Vec<String>,
    /// The module that contributed this entry.
    pub module_name: String,
}

/// A method id more than one module declared, in first-wins order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateMethod {
    /// The contested id.
    pub id: String,
    /// The modules that declared it, the winner first.
    pub modules: Vec<String>,
}

/// A module root whose `manifest.yaml` could not be read or parsed.
///
/// Reported rather than thrown: a catalog missing one module's entries is
/// still worth having, and the command that would have crashed is the one an
/// operator runs *to diagnose* the module (agent-ix/quoin#106).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnreadableModule {
    /// The resolved module root.
    pub module_root: String,
    /// The reader's own message. See `DIVERGENCE.md` §4.
    pub reason: String,
}

/// The merged catalog plus what the merge could not use.
///
/// Merge is **first-wins by method id**, matching quire-rs FR-054 exactly. If
/// the two disagreed, the advisor would recommend from one catalog while the
/// auditor checked conformance against another.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MethodCatalog {
    /// Every method, sorted by id.
    pub methods: Vec<VerificationMethod>,
    /// Contested ids, sorted by id.
    pub duplicates: Vec<DuplicateMethod>,
    /// Unreadable module roots, sorted by root.
    pub unreadable: Vec<UnreadableModule>,
}

/// Every distinct `class` in the catalog, sorted — the IADT axis in practice.
///
/// `[...new Set(...)].sort()` in the retained source: the default
/// `Array.prototype.sort`, which orders by UTF-16 code unit and not by Unicode
/// scalar value.
#[must_use]
pub fn method_classes(catalog: &MethodCatalog) -> Vec<String> {
    let distinct: BTreeSet<&str> = catalog
        .methods
        .iter()
        .map(|method| method.class.as_str())
        .collect();
    let mut classes: Vec<String> = distinct.into_iter().map(ToOwned::to_owned).collect();
    classes.sort_by(|left, right| js::compare(left, right));
    classes
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
    use super::{MethodCatalog, VerificationMethod, method_classes};

    fn method(id: &str, class: &str) -> VerificationMethod {
        VerificationMethod {
            id: id.to_owned(),
            name: String::new(),
            class: class.to_owned(),
            definition: String::new(),
            evidence_kind: None,
            applicability: std::collections::BTreeMap::new(),
            tooling: Vec::new(),
            module_name: "m".to_owned(),
        }
    }

    #[test]
    fn classes_are_distinct_and_ordered_by_code_unit() {
        // U+FFFD sorts BEFORE U+10000 by code unit and after it by scalar
        // value: the two orders disagree, so this is not a tautology.
        let catalog = MethodCatalog {
            methods: vec![
                method("b", "\u{10000}"),
                method("a", "Test"),
                method("c", "\u{fffd}"),
                method("d", "Test"),
            ],
            duplicates: Vec::new(),
            unreadable: Vec::new(),
        };
        assert_eq!(
            method_classes(&catalog),
            ["Test", "\u{10000}", "\u{fffd}"],
            "classes must order by UTF-16 code unit, as Array.prototype.sort does"
        );
    }

    #[test]
    fn an_absent_evidence_kind_is_omitted_and_never_written_as_null() {
        assert_eq!(
            serde_json::to_string(&method("unit-testing", "Test")).unwrap(),
            r#"{"id":"unit-testing","name":"","class":"Test","definition":"","applicability":{},"tooling":[],"moduleName":"m"}"#
        );
    }
}
