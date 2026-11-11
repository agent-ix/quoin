// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! PLAT-1401: the NFR percentage metrics are measured over populations the
//! code enumerates, and each plan's declared set must equal that enumeration.
//!
//! Provenance: PLAT-1401

#![allow(
    clippy::expect_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod nfr_populations;

use ix_trace_rs::trace;
use nfr_populations::{assert_accepted, commands, manifests};

fn home() -> tempfile::TempDir {
    tempfile::tempdir().expect("an isolated home is creatable")
}

/// NFR-003-M-1 and NFR-003-M-2: every command refuses an unknown flag out loud,
/// and none fails silently.
#[trace("FR-131-AC-9", "TC-3003", "NFR-003-M-1", "NFR-003-M-2")]
#[test]
fn tc_3003_every_command_fails_a_bad_argument_out_loud() {
    let home = home();
    let (actionable, silent) = commands::bad_arguments(home.path());
    assert!(
        actionable.examined.contains("catalog") && actionable.examined.len() > 10,
        "{:?}",
        actionable.examined
    );
    assert_eq!(
        actionable.matched, actionable.examined,
        "a command fails quietly"
    );
    assert!(silent.matched.is_empty(), "{:?}", silent.matched);
    assert_accepted("MP-250", &actionable);
    assert_accepted("MP-251", &silent);
}

/// NFR-007-M-1: `ix-flow` absent and `ix-flow` failing both reach the caller as a
/// non-zero exit, for every workflow command.
#[cfg(unix)]
#[trace("FR-131-AC-10", "TC-3004", "NFR-007-M-1")]
#[test]
fn tc_3004_an_ix_flow_failure_is_a_non_zero_quoin_exit() {
    let home = home();
    let probed = commands::external_tool_failures(home.path());
    assert_eq!(probed.examined.len(), 4, "{:?}", probed.examined);
    assert_eq!(probed.matched, probed.examined);
    assert_accepted("MP-252", &probed);
}

/// NFR-008-M-1 and NFR-008-M-2: every corrupt manifest aborts assembly and every
/// candidate without one is skipped.
#[trace("FR-131-AC-11", "TC-3005", "NFR-008-M-1", "NFR-008-M-2")]
#[test]
fn tc_3005_corrupt_manifests_abort_and_missing_ones_are_skipped() {
    let home = home();
    let corrupt = manifests::corrupt_manifests(home.path());
    assert_eq!(
        corrupt.matched, corrupt.examined,
        "a corrupt manifest did not abort"
    );
    assert_accepted("MP-253", &corrupt);
    let missing = manifests::missing_manifests(home.path());
    assert_eq!(
        missing.matched, missing.examined,
        "a missing manifest was not skipped"
    );
    assert_accepted("MP-254", &missing);
}
