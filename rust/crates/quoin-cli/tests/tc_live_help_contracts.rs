// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Live help contracts that remain useful independently of the retained-shell captures.

#![allow(
    clippy::expect_used,
    reason = "test assertions deliberately panic to report a broken help contract"
)]

use ix_trace_rs::trace;
use std::process::{Command, Output};

fn invoke(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(arguments)
        .output()
        .expect("the native quoin binary runs")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("the native shell emits UTF-8")
}

/// `quoin update --help` describes the release-manifest path the code takes,
/// not an npm registry install.
#[trace("FR-022", "FR-102")]
#[test]
fn tc_373_update_help_describes_the_release_manifest_not_an_npm_install() {
    let output = invoke(&["update", "--help"]);
    assert_eq!(output.status.code(), Some(0));
    let help = text(&output.stdout);
    assert!(help.contains("release manifest"), "{help}");
    assert!(help.contains("node_modules"), "{help}");
    assert!(!help.contains("npm install -g to upgrade"), "{help}");
    assert!(!help.contains("registry.npmjs.org"), "{help}");
}

/// The assurance help exposes the formalization-record input.
/// Provenance: VER-111.
#[trace("FR-205-AC-12")]
#[test]
fn tc_205_assurance_help_exposes_formalization_records() {
    let output = invoke(&["assurance", "--help"]);
    assert_eq!(output.status.code(), Some(0));
    let help = text(&output.stdout);
    assert!(
        help.contains("[--formalization-records <FILE>]"),
        "usage must show the option and its file argument: {help}"
    );
    assert!(
        help.contains("--formalization-records=<FILE>"),
        "flags must describe the option and its file argument: {help}"
    );
    assert!(help.contains("Contract IR FR-071 records"), "{help}");
}

/// The record help describes the collection format and points at the worked example.
#[trace("FR-061", "FR-102")]
#[test]
fn tc_1050_measurement_record_help_describes_the_collection_format() {
    let output = invoke(&["measurement", "record", "--help"]);
    assert_eq!(output.status.code(), Some(0));
    let help = text(&output.stdout);
    for literal in [
        "collectionId",
        "verificationStack",
        "observations",
        "population",
        "sourceState",
        "docs/measurement-walkthrough.md",
    ] {
        assert!(
            help.contains(literal),
            "help must mention {literal}: {help}"
        );
    }
}
