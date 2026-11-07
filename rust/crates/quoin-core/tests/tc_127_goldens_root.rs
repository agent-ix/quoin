// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Corpora that tests read as data live in one place, `rust/goldens/`, each
//! with a provenance record in one shape.
//!
//! Three crates once invented three homes and three provenance layouts. A
//! fourth would make "where did this corpus come from" unanswerable by one
//! check, so the layout is read off the tree here.
//!
//! This census carries no `Trace:` tag: it is an engineering criterion from
//! quoin#411, not an obligation any requirement states.
//!
//! Provenance: PLAT-127, quoin#411

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

const SECTIONS: [&str; 3] = ["## Origin", "## Captured at", "## Divergences"];

fn rust_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn subdirs(of: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(of)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect();
    found.sort();
    found
}

fn name(path: &Path) -> String {
    path.file_name().unwrap().to_string_lossy().into_owned()
}

#[test]
fn no_crate_keeps_a_golden_directory_of_its_own() {
    let mut strays = Vec::new();
    for krate in subdirs(&rust_root().join("crates")) {
        let tests = krate.join("tests");
        if !tests.is_dir() {
            continue;
        }
        for dir in subdirs(&tests) {
            if matches!(name(&dir).as_str(), "golden" | "goldens") {
                strays.push(dir.display().to_string());
            }
        }
    }
    assert!(
        strays.is_empty(),
        "move these under rust/goldens/<crate>/: {strays:?}"
    );
}

#[test]
fn every_goldens_directory_names_a_crate_and_holds_json_plus_provenance() {
    let root = rust_root();
    let dirs = subdirs(&root.join("goldens"));
    assert!(!dirs.is_empty(), "rust/goldens/ holds no corpus");
    for dir in dirs {
        let crate_name = name(&dir);
        assert!(
            root.join("crates").join(&crate_name).is_dir(),
            "rust/goldens/{crate_name} names no crate under rust/crates/"
        );
        let mut corpora = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            let file = name(&path);
            assert!(path.is_file(), "{} is not a file", path.display());
            if file == "PROVENANCE.md" || file.starts_with('.') {
                continue;
            }
            assert!(
                path.extension().is_some_and(|ext| ext == "json"),
                "{} is neither a .json corpus nor PROVENANCE.md",
                path.display()
            );
            corpora += 1;
        }
        assert!(corpora > 0, "rust/goldens/{crate_name} holds no corpus");
    }
}

#[test]
fn provenance_has_its_three_sections_in_order_and_a_resolvable_divergence_link() {
    let root = rust_root();
    for dir in subdirs(&root.join("goldens")) {
        let path = dir.join("PROVENANCE.md");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()));
        let mut from = 0;
        for section in SECTIONS {
            let at = text[from..]
                .find(&format!("\n{section}\n"))
                .unwrap_or_else(|| panic!("{} lacks `{section}` in order", path.display()));
            from += at + section.len();
        }
        let divergences = &text[text.find("\n## Divergences\n").unwrap()..];
        if let Some(open) = divergences.find("](") {
            let rest = &divergences[open + 2..];
            let target = &rest[..rest.find(')').expect("link closes")];
            assert!(
                dir.join(target).is_file(),
                "{} links {target}, which does not exist",
                path.display()
            );
        }
    }
}
