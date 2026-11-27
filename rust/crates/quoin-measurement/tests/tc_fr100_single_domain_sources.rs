// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Keep the canonical evidence and measurement models in their owning crates.

#![allow(
    clippy::expect_used,
    reason = "test fixtures use expect as the failure report"
)]

use ix_trace_rs::trace;
use std::path::{Path, PathBuf};

fn rust_sources(root: &Path) -> Vec<(PathBuf, String)> {
    fn walk(dir: &Path, out: &mut Vec<(PathBuf, String)>) {
        for item in std::fs::read_dir(dir).expect("crate source directory is readable") {
            let path = item.expect("directory entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                out.push((
                    path.clone(),
                    std::fs::read_to_string(path).expect("Rust source reads"),
                ));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out);
    out
}

#[trace("FR-100-CON-1")]
#[test]
fn evidence_and_measurement_domain_types_have_one_declared_owner() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("crate root lives under rust/crates");
    let mut all = Vec::new();
    for (crate_name, owner) in [
        ("quoin-evidence", "quoin-evidence"),
        ("quoin-store", "quoin-store"),
        ("quoin-measurement", "quoin-measurement"),
        ("quoin-measurement-graph", "quoin-measurement-graph"),
    ] {
        let root = workspace.join("rust/crates").join(crate_name).join("src");
        all.extend(
            rust_sources(&root)
                .into_iter()
                .map(|(path, source)| (crate_name, owner, path, source)),
        );
    }
    let declarations = [
        ("pub struct RunEntry", "quoin-evidence"),
        ("pub struct RunRecord", "quoin-evidence"),
        ("pub struct MeasurementPlan", "quoin-measurement"),
        ("pub enum MeasurementState", "quoin-measurement"),
    ];
    for (declaration, expected_owner) in declarations {
        let matches: Vec<_> = all
            .iter()
            .filter(|(_, _, _, source)| {
                source.lines().any(|line| {
                    line.trim_start()
                        .strip_prefix(declaration)
                        .is_some_and(|rest| {
                            rest.chars()
                                .next()
                                .is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
                        })
                })
            })
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "{declaration} has one canonical declaration"
        );
        assert_eq!(
            matches[0].0, expected_owner,
            "{declaration} stays in its owner crate"
        );
    }
}
