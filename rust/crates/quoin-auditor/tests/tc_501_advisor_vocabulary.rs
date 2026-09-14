// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-031-CON-1: the advisor carries no method vocabulary of its own.
//!
//! *"The advisor SHALL NOT carry its own method list. Every method, class and
//! evidence kind comes from module data; a restated table is the failure being
//! closed."*
//!
//! The obvious test is the wrong one, and the distinction is the point:
//! asserting that the merged catalog IS read from module data passes just as
//! well when a restated table sits beside it. This asserts the ABSENCE the
//! criterion states — no module under `src/advise/` may name one of the four
//! declared method classes as a literal.
//!
//! Carried over from `tests/arch-boundaries.test.ts`, deleted with
//! `src/advisor/` at cutover.
//!
//! Trace: FR-031-CON-1
//! Provenance: quoin#501

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

/// The four method classes the catalog declares. A literal naming one inside
/// the advisor is a restatement whatever it is used for.
const VOCABULARY: &[&str] = &["Test", "Inspection", "Analysis", "Demonstration"];

/// Every `.rs` file under `src/advise/`, as `(relative path, contents)`.
fn advisor_sources() -> Vec<(String, String)> {
    fn walk(root: &Path, directory: &Path, into: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(directory).expect("src/advise/ is readable") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                walk(root, &path, into);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let name = path
                    .strip_prefix(root)
                    .expect("a path under src/advise/")
                    .to_string_lossy()
                    .replace('\\', "/");
                let text = std::fs::read_to_string(&path).expect("a source file");
                into.push((name, text));
            }
        }
    }
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/advise");
    let mut found = Vec::new();
    walk(&root, &root, &mut found);
    found.sort();
    found
}

/// Trace: FR-031-CON-1
/// Provenance: quoin#501
#[test]
fn tc_501_020_the_advisor_names_no_method_class_as_a_literal() {
    let sources = advisor_sources();
    assert!(
        sources.len() >= 4,
        "anti-vacuity floor: at least 4 advisor modules expected, saw {} — a \
         census that finds nothing proves nothing",
        sources.len()
    );

    let mut restated = Vec::new();
    for (name, text) in &sources {
        // A module's own `#[cfg(test)]` block is where a fixture catalog is
        // SUPPOSED to name classes: it is stating the module data the advisor
        // is handed, not carrying a table of its own. The retained TypeScript
        // check had the same boundary for free, its tests living outside
        // `src/`.
        let text = text
            .split_once("#[cfg(test)]")
            .map_or(text.as_str(), |(before, _)| before);
        for class in VOCABULARY {
            // A quoted literal, not the bare word: the prose in this crate
            // discusses these classes by name, and a doc comment explaining
            // what a class IS is not a table of them.
            let quoted = format!("\"{class}\"");
            for (index, line) in text.lines().enumerate() {
                if line.contains(&quoted) {
                    restated.push(format!("{name}:{} names {quoted}", index + 1));
                }
            }
        }
    }
    assert!(
        restated.is_empty(),
        "the advisor restates the catalog's method classes: {restated:?}. Every \
         class comes from module data; a literal here is the table FR-031-CON-1 \
         exists to close."
    );
}

/// The scanner fires on a planted restatement, so a clean census is evidence.
///
/// Trace: FR-031-CON-1
/// Provenance: quoin#501
#[test]
fn tc_501_021_the_scanner_would_find_a_planted_restatement() {
    let planted = "const CLASSES: &[&str] = &[\"Test\", \"Inspection\"];";
    let found: Vec<&&str> = VOCABULARY
        .iter()
        .filter(|class| planted.contains(&format!("\"{class}\"")))
        .collect();
    assert_eq!(
        found.len(),
        2,
        "the scanner must see a restated table when one is there"
    );
}
