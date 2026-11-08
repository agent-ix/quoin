// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The `spec-fuzz` skill's cargo-fuzz template must emit targets that compile
//! warning-free and bind both trace carriers to a named symbol.
//!
//! Provenance: PLAT-166, PLAT-167

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

fn skill_file(relative: &str) -> String {
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    std::fs::read_to_string(root.join("skills/spec-fuzz").join(relative)).expect("skill file")
}

/// Trace: FR-038-AC-5
#[test]
fn tc_166_cargo_fuzz_template_puts_no_doc_comment_on_the_macro_invocation() {
    let template = skill_file("assets/templates/cargo-fuzz.rs.md");
    let lines: Vec<&str> = template.lines().collect();
    let mut invocations = 0;
    for (i, line) in lines.iter().enumerate() {
        if line.trim_start().starts_with("fuzz_target!(") {
            invocations += 1;
            assert!(
                i > 0 && !lines[i - 1].trim_start().starts_with("///"),
                "doc comment directly above fuzz_target! (unused_doc_comments): line {}",
                i + 1
            );
        }
    }
    assert_eq!(
        invocations, 2,
        "both template examples use the wrapper form"
    );
}

/// Trace: FR-038-AC-5
#[test]
fn tc_166_cargo_fuzz_template_carriers_sit_on_a_named_wrapper() {
    let template = skill_file("assets/templates/cargo-fuzz.rs.md");
    let lines: Vec<&str> = template.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("/// Trace:") {
            let next_non_doc = lines[i..]
                .iter()
                .find(|l| !l.starts_with("///"))
                .expect("item after doc comment");
            assert!(
                next_non_doc.starts_with("fn fuzz_"),
                "Trace carrier must document a named fuzz_ fn, got {next_non_doc:?}"
            );
        }
    }
}

/// Trace: FR-038-AC-4
#[test]
fn tc_167_step_0_separates_installed_tooling_from_the_repo_workspace() {
    let step = skill_file("references/step-0-scope-and-harness.md");
    assert!(
        !step.contains("or `cargo fuzz --version` resolves |"),
        "installed cargo-fuzz must not be listed as a harness"
    );
    assert!(step.contains("Tooling is not a harness"));
    assert!(step.contains("no files are emitted"));
}
