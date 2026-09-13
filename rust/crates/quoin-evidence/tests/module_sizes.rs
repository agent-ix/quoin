// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! No module in this crate grows past the point where it stops being one idea.
//!
//! 700 lines is a hard ceiling and 500 is where a module should already be
//! looking for its seam. The ceiling is enforced because `src/evidence/` was
//! deleted at 3,914 lines across 18 files with a 720-line store module at its
//! centre (quoin#458), and the shape a replacement grows into is decided by
//! whether anything objects on the way.
//!
//! A ceiling with an allow-list is a ceiling; a ceiling with an allow-list
//! nobody has to justify is a formality. Every entry in [`ALLOWED`] carries the
//! reason the module is exempt and the size it was exempted at, so an entry
//! that has quietly grown further fails just as a new offender would.

#![allow(
    clippy::panic,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::fs;
use std::path::{Path, PathBuf};

/// The point past which a module is doing more than one thing.
const HARD_CEILING: usize = 700;

/// Modules exempted from [`HARD_CEILING`], each with its reason and its budget.
///
/// Empty, and that is the claim: at quoin#458 the largest module in the crate
/// is `assurance_records.rs` at 681 lines, so nothing needs an exemption. An
/// entry added here must name why the module cannot be split, not merely that
/// it is large.
const ALLOWED: &[(&str, usize, &str)] = &[];

/// Every module the crate compiles, as a path relative to `src/`.
fn modules() -> Vec<(String, usize)> {
    let mut found = Vec::new();
    collect(&source_root(), &source_root(), &mut found);
    found.sort();
    found
}

fn source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn collect(root: &Path, directory: &Path, found: &mut Vec<(String, usize)>) {
    for entry in fs::read_dir(directory).expect("the crate source tree is readable") {
        let path = entry.expect("a readable directory entry").path();
        if path.is_dir() {
            collect(root, &path, found);
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "rs") {
            continue;
        }
        let text = fs::read_to_string(&path).expect("a module is readable");
        let name = path
            .strip_prefix(root)
            .expect("every module is under src/")
            .to_string_lossy()
            .replace('\\', "/");
        found.push((name, text.lines().count()));
    }
}

/// No module passes the ceiling, and the allow-list is not a blank cheque.
///
/// The module-count floor is the anti-vacuity guard: a census that walked the
/// wrong directory would find nothing and pass, which is the failure mode this
/// kind of test has.
///
/// Trace: FR-101-AC-5
/// Provenance: quoin#458
#[test]
fn tc_458_310_no_module_grows_past_the_ceiling() {
    let modules = modules();
    assert!(
        modules.len() >= 30,
        "the census found {} modules, which is fewer than this crate has — it walked the wrong tree",
        modules.len()
    );

    let mut over = Vec::new();
    for (name, lines) in &modules {
        let budget = ALLOWED
            .iter()
            .find(|(allowed, _, _)| allowed == name)
            .map_or(HARD_CEILING, |(_, budget, _)| *budget);
        if *lines > budget {
            over.push(format!("{name}: {lines} lines, budget {budget}"));
        }
    }
    assert!(
        over.is_empty(),
        "modules past their budget:\n  {}",
        over.join("\n  ")
    );

    for (name, _, reason) in ALLOWED {
        assert!(
            modules.iter().any(|(module, _)| module == name),
            "{name} is exempted but does not exist; drop the entry"
        );
        assert!(
            !reason.trim().is_empty(),
            "{name} is exempted without a reason"
        );
    }
}
