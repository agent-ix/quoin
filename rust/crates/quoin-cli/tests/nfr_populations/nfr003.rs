// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! NFR-003-M-1 and NFR-003-M-2 over command x error class. Each class is its
//! own module with its own enumeration from the binary's help; this module
//! joins their probes into the two populations the plans declare, and holds
//! the per-class tests.
//!
//! A probe that does not match is a reading below the target, not a staging
//! fault: each class test names the ids that read below it today, so the set
//! can only shrink when the command is fixed and cannot grow unnoticed.

#![allow(
    clippy::expect_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::BTreeSet;

use ix_trace_rs::trace;

use super::context::Context;
use super::failure::{Failure, probed};
use super::{
    Probed, bad_flag, catalog_conflict, malformed_input, missing_argument, missing_path,
    unknown_subcommand, unknown_type,
};

/// Every class's probes, in a fixed order.
pub(crate) fn failures(cx: &Context) -> Vec<Failure> {
    [
        bad_flag::probe,
        unknown_subcommand::probe,
        missing_argument::probe,
        missing_path::probe,
        unknown_type::probe,
        malformed_input::probe,
        catalog_conflict::probe,
    ]
    .iter()
    .flat_map(|probe| probe(cx))
    .collect()
}

/// The populations of NFR-003-M-1 (actionable) and NFR-003-M-2 (silent).
pub(crate) fn populations(cx: &Context) -> (Probed, Probed) {
    probed(&failures(cx))
}

/// Assert that exactly `known` read below the target in `failures`, and that
/// nothing silent is outside it.
fn assert_reads(class: &str, failures: &[Failure], known: &[&str]) {
    let known: BTreeSet<String> = known.iter().map(|id| (*id).to_owned()).collect();
    let unactionable: BTreeSet<String> = failures
        .iter()
        .filter(|f| !f.actionable)
        .map(|f| f.id.clone())
        .collect();
    let detail: Vec<_> = failures
        .iter()
        .filter(|f| !known.contains(&f.id) && !f.actionable)
        .map(|f| format!("{}: {}", f.id, f.detail))
        .collect();
    assert_eq!(
        unactionable, known,
        "{class} reads differently: {detail:#?}"
    );
    assert!(
        failures.iter().all(|f| !f.silent || known.contains(&f.id)),
        "{class}: a failure is silent outside the known set"
    );
}

#[trace("FR-131-AC-9", "NFR-003-M-1", "NFR-003-M-2")]
#[test]
fn tc_3700_every_command_refuses_an_unknown_flag_naming_it() {
    let failures = bad_flag::probe(&Context::new());
    assert!(failures.len() > 40, "{}", failures.len());
    assert_reads("unknown-flag", &failures, &[]);
}

#[trace("FR-131-AC-9", "NFR-003-M-1", "NFR-003-M-2")]
#[test]
fn tc_3701_every_command_group_refuses_an_unknown_subcommand_listing_the_known() {
    let failures = unknown_subcommand::probe(&Context::new());
    assert!(
        failures.iter().any(|f| f.id == "quoin:unknown-subcommand")
            && failures
                .iter()
                .any(|f| f.id == "catalog:unknown-subcommand"),
        "{:?}",
        failures.iter().map(|f| &f.id).collect::<Vec<_>>()
    );
    assert_reads("unknown-subcommand", &failures, &[]);
}

#[trace("FR-131-AC-9", "NFR-003-M-1", "NFR-003-M-2")]
#[test]
fn tc_3702_a_bare_command_names_every_argument_it_requires() {
    let failures = missing_argument::probe(&Context::new());
    assert!(
        failures
            .iter()
            .any(|f| f.id == "config get:missing-argument"),
        "{:?}",
        failures.iter().map(|f| &f.id).collect::<Vec<_>>()
    );
    assert_reads("missing-argument", &failures, &[]);
}

#[trace("FR-131-AC-9", "NFR-003-M-1", "NFR-003-M-2")]
#[test]
fn tc_3703_a_path_that_does_not_exist_is_named_in_the_refusal() {
    let failures = missing_path::probe(&Context::new());
    assert!(
        failures
            .iter()
            .any(|f| f.id == "advise --repo:missing-path"),
        "{:?}",
        failures.iter().map(|f| &f.id).collect::<Vec<_>>()
    );
    assert_reads(
        "missing-path",
        &failures,
        &[
            "change-assurance recover --repo:missing-path",
            "completeness --bundle:missing-path",
            "completeness --module:missing-path",
            "completeness --repo:missing-path",
            "evidence gc --repo:missing-path",
            "measurement intervention --definition:missing-path",
            "measurement operational-release --definition:missing-path",
            "measurement record --input:missing-path",
            "report --portfolio:missing-path",
            "report --repo:missing-path",
            "validate --repo:missing-path",
        ],
    );
}

#[trace("FR-131-AC-9", "NFR-003-M-1", "NFR-003-M-2")]
#[test]
fn tc_3704_an_unknown_type_is_refused_listing_the_available_types() {
    let failures = unknown_type::probe(&Context::new());
    assert_eq!(
        failures.len(),
        2,
        "{:?}",
        failures.iter().map(|f| &f.id).collect::<Vec<_>>()
    );
    assert_reads(
        "unknown-type",
        &failures,
        &["catalog show type:unknown-type"],
    );
}

#[trace("FR-131-AC-9", "NFR-003-M-1", "NFR-003-M-2")]
#[test]
fn tc_3705_malformed_json_is_refused_saying_it_is_not_json_and_where() {
    let failures = malformed_input::probe(&Context::new());
    assert!(
        failures
            .iter()
            .any(|f| f.id == "evidence trust --decision:malformed-stdin"),
        "{:?}",
        failures.iter().map(|f| &f.id).collect::<Vec<_>>()
    );
    assert_reads(
        "malformed-input",
        &failures,
        &[
            "change-assurance write-record --input:malformed-file",
            "change-assurance write-record --input:malformed-stdin",
        ],
    );
}

#[trace("FR-131-AC-9", "NFR-003-M-1", "NFR-003-M-2")]
#[test]
fn tc_3706_a_catalog_conflict_names_the_type_and_both_modules() {
    let failures = catalog_conflict::probe(&Context::new());
    assert_eq!(
        failures.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(),
        ["catalog validate:duplicate-type"]
    );
    assert_reads("catalog-conflict", &failures, &[]);
}
