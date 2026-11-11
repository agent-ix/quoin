// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The `spec-correctness` skill takes `surviving-mutant` audit findings, grouped
//! by `row_id`, as an optional input and directs a test at each.
//!
//! Provenance: PLAT-394

#![allow(
    clippy::expect_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
use std::path::{Path, PathBuf};

fn skill_file(relative: &str) -> String {
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    std::fs::read_to_string(root.join("skills/spec-correctness").join(relative))
        .expect("skill file")
}

#[trace("FR-039-AC-17", "TC-3102")]
#[test]
fn the_skill_takes_surviving_mutant_findings_per_row_id() {
    let skill = skill_file("SKILL.md");
    assert!(skill.contains("references/surviving-mutants.md"));
    let reference = skill_file("references/surviving-mutants.md");
    for needle in [
        "quoin evidence audit --json",
        "surviving-mutant",
        "`row_id`",
        "`obligation`",
        "emit one property",
    ] {
        assert!(reference.contains(needle), "reference lacks {needle}");
    }
}
