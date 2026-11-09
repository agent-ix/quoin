// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The `spec-fuzz` skill's cargo-fuzz template must emit targets that compile
//! warning-free and bind both trace carriers (the `#[trace(...)]` marker and the
//! id in the symbol name) to a named wrapper function.
//!
//! Provenance: PLAT-166

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
use std::path::{Path, PathBuf};

fn skill_file(relative: &str) -> String {
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    std::fs::read_to_string(root.join("skills/spec-fuzz").join(relative)).expect("skill file")
}

#[trace("FR-038-AC-5")]
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
    assert!(
        invocations >= 1,
        "template shows no fuzz_target! invocation"
    );
}

/// `NFR-003-M-1` -> `nfr_003_m_1`, the form an identifier can carry.
fn slug(id: &str) -> String {
    id.to_lowercase().replace('-', "_")
}

#[trace("FR-038-AC-5")]
#[test]
fn tc_166_cargo_fuzz_template_carriers_sit_on_a_named_wrapper() {
    let template = skill_file("assets/templates/cargo-fuzz.rs.md");
    let lines: Vec<&str> = template.lines().collect();
    let mut wrappers = 0;
    for (i, line) in lines.iter().enumerate() {
        let Some(rest) = line.strip_prefix("#[trace(\"") else {
            continue;
        };
        let id = rest.split('"').next().unwrap_or_default();
        let doc_block: Vec<&&str> = lines[..i]
            .iter()
            .rev()
            .take_while(|l| l.starts_with("///"))
            .collect();
        assert!(
            doc_block
                .iter()
                .any(|l| l.contains(&format!("spec-fuzz: obligation={id} "))),
            "provenance line for {id} must sit in the doc block above its trace marker"
        );
        let item = lines[i + 1];
        let name = item
            .strip_prefix("fn ")
            .and_then(|rest| rest.split('(').next())
            .unwrap_or_else(|| {
                panic!("trace marker for {id} must sit on a named fn, got {item:?}")
            });
        assert!(
            name.starts_with("fuzz_") && name.contains(&slug(id)),
            "wrapper name {name:?} must carry the obligation id {id}"
        );
        wrappers += 1;
    }
    assert!(wrappers >= 1, "template shows no trace-marked wrapper");
}
