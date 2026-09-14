// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The two catalog checks: is the declared method declared at all, and did the
//! evidence match its kind?

use std::collections::BTreeSet;

use quoin_combinatorial::js;
use quoin_evidence::types::{Binding, RunRecord};
use quoin_finding_types::{Finding, FindingKind, Severity};
use quoin_quire_types::Obligation;

use crate::catalog::{MethodCatalog, VerificationMethod};

/// Catalog methods whose id or class matches the declared method.
///
/// `String.prototype.toLowerCase` is full Unicode case mapping, which is what
/// Rust's `str::to_lowercase` implements; `trim` is the JavaScript one
/// (`js_trim`), whose whitespace set includes U+FEFF and is wider than
/// `str::trim`'s in that one respect.
#[must_use]
pub fn catalog_methods_matching<'a>(
    method: &str,
    catalog: &'a MethodCatalog,
) -> Vec<&'a VerificationMethod> {
    let declared = js::js_trim(method).to_lowercase();
    catalog
        .methods
        .iter()
        .filter(|entry| {
            entry.id.to_lowercase() == declared || entry.class.to_lowercase() == declared
        })
        .collect()
}

/// Is the declared method declared at all?
///
/// A `Verification` cell naming neither a catalog method id nor a catalog
/// class used to return `null` from [`method_conformance`] — a silent skip —
/// so the requirements whose verification is *least* well defined were exactly
/// the ones nothing questioned. Measured across the ecosystem when this
/// landed: 55 of 577 obligations (agent-ix/quoin#105, quire-rs#152).
///
/// Asked before the binding guard, because this comparison needs no evidence
/// at all — the statement and the catalog are the whole input
/// (agent-ix/quoin#165).
#[must_use]
pub fn unknown_method_finding(
    obligation: &Obligation,
    catalog: Option<&MethodCatalog>,
) -> Option<Finding> {
    let catalog = catalog?;
    // `!obligation.method` — an empty cell is falsy, so it is not an unknown
    // method, it is no method.
    let method = obligation
        .method
        .as_deref()
        .filter(|cell| !cell.is_empty())?;
    if !catalog_methods_matching(method, catalog).is_empty() {
        return None;
    }
    Some(Finding::new(
        FindingKind::UNKNOWN_METHOD,
        &obligation.id,
        Severity::medium(),
        format!(
            "{id} declares `{method}`, which is neither a catalog method id nor a catalog \
             class. Nothing can say what discharging it means, so no conformance check can \
             run against it.",
            id = obligation.id
        ),
    ))
}

/// Did the evidence match the declared method's kind?
///
/// Compared **kind to kind**. The old test was `run.entries.length > 0` — read
/// as "this was a test run", but true of a transcribed inspection too, so
/// every `Inspection` or `Analysis` obligation recorded through `quoin
/// evidence record` was flagged. A run now declares its own `evidenceKind`,
/// and when it declares none the check says nothing: an undeclared kind means
/// the question cannot be asked, which is a different answer from "it
/// conformed".
///
/// An uncatalogued method returns [`None`] here without a finding — that is
/// not the old silent skip: [`unknown_method_finding`] has already reported
/// it, before the binding guard.
#[must_use]
pub fn method_conformance(
    obligation: &Obligation,
    paired: &[(&Binding, &RunRecord)],
    catalog: Option<&MethodCatalog>,
) -> Option<Finding> {
    let catalog = catalog?;
    let method = obligation
        .method
        .as_deref()
        .filter(|cell| !cell.is_empty())?;
    let matched = catalog_methods_matching(method, catalog);
    if matched.is_empty() {
        return None;
    }

    // The kinds the catalog says this method produces. An entry declaring none
    // makes the comparison unanswerable rather than failed.
    let expected: BTreeSet<&str> = matched
        .iter()
        .filter_map(|entry| entry.evidence_kind.as_deref())
        .filter(|kind| !kind.is_empty())
        .collect();
    if expected.is_empty() {
        return None;
    }

    let mismatched: Vec<&(&Binding, &RunRecord)> = paired
        .iter()
        .filter(|(_, run)| {
            run.evidence_kind
                .as_deref()
                .is_some_and(|kind| !kind.is_empty() && !expected.contains(kind))
        })
        .collect();
    if mismatched.is_empty() {
        return None;
    }

    let mut kinds: Vec<&str> = expected.into_iter().collect();
    kinds.sort_by(|left, right| js::compare(left, right));
    let discharged = mismatched
        .iter()
        .map(|(binding, run)| {
            format!(
                "a {kind} run in {suite}",
                kind = run.evidence_kind.as_deref().unwrap_or_default(),
                suite = binding.suite
            )
        })
        .collect::<Vec<_>>()
        .join(" and ");

    Some(Finding::new(
        FindingKind::METHOD_CONFORMANCE,
        &obligation.id,
        Severity::medium(),
        format!(
            "{id} declares `{method}`, whose evidence kind is {kinds}, but is discharged by \
             {discharged}. A method is not discharged by evidence of another kind.",
            id = obligation.id,
            kinds = kinds.join("/")
        ),
    ))
}
