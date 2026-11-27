// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Default corpus sweeps keep semantic migration findings advisory.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    reason = "invalid fixture corpus is a test failure"
)]

mod common;

use std::fs;

use ix_trace_rs::trace;
use quoin_semantic::sweep::{CorpusRoot, SweepIdentity, sweep_corpus};

#[trace("NFR-017-AC-2")]
#[test]
fn corpus_fixture_sweep_reports_only_warning_semantic_findings_by_default() {
    let scratch = common::Scratch::new();
    let root = scratch.path().join("corpus");
    let specs = root.join("spec/functional");
    fs::create_dir_all(&specs).expect("fixture spec directory is writable");
    for name in [
        "config-version.table.md",
        "legacy-bullets.md",
        "legacy-mixed.md",
        "config-version.fence.md",
        "legacy-free-table.md",
    ] {
        fs::copy(common::mapping_dir().join(name), specs.join(name))
            .unwrap_or_else(|error| panic!("fixture {name} copies: {error}"));
    }

    let report = sweep_corpus(
        &[CorpusRoot {
            root,
            repository: "corpus-fixture".to_owned(),
        }],
        &SweepIdentity {
            package: "agent-ix/spec-objects-fixture".into(),
            version: "0.1.0".to_owned(),
        },
        "2026-11-27T00:00:00.000Z",
    )
    .expect("the fixture corpus sweep completes");

    let legacy = report
        .findings
        .iter()
        .filter_map(|finding| finding.diagnostic.as_ref())
        .collect::<Vec<_>>();
    assert!(!legacy.is_empty(), "the fixture set includes legacy forms");
    assert!(
        legacy
            .iter()
            .all(|diagnostic| diagnostic.severity == "warning"),
        "legacy semantic findings remain advisory by default: {legacy:#?}"
    );
}
