// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! No module in this crate grows past the size a reader can hold.
//!
//! # Why a census rather than a review note
//!
//! The ceiling is a review criterion (quoin#464): 700 lines hard, 500 soft. A
//! criterion enforced only by review is enforced only when someone is looking,
//! and `quoin-change-assurance`'s `model/record.rs` reached 1,078 lines that
//! way before quoin#457 split it. So the ceiling is read off this crate's own
//! `src/` tree here, with a floor underneath it: a census that stopped finding
//! files would otherwise pass by finding nothing.
//!
//! This is the crate's **one** census — `quoin-quire` and `quoin-jsonschema`
//! each carry the same test for their own tree, and a second one here would be
//! a copy rather than a measurement.
//!
//! This census carries no `Trace:` tag: it is an engineering criterion from
//! quoin#464, not an obligation any requirement states.
//!
//! Provenance: quoin#475

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

/// The hard ceiling: no module may exceed this.
const HARD_CEILING: usize = 700;

/// The soft ceiling: a module above this is worth splitting, and is listed by
/// this test so that the next one to cross it is visible rather than quiet.
const SOFT_CEILING: usize = 500;

/// Modules permitted above [`HARD_CEILING`], each with the reason it is exempt.
///
/// Empty, and meant to stay that way: an entry is a debt, not a dispensation.
const ALLOWED_ABOVE_HARD_CEILING: &[(&str, &str)] = &[];

/// The modules currently over [`SOFT_CEILING`].
///
/// Empty. The 763 retained TypeScript lines became a tree of small modules
/// rather than one file of the same size: the largest here is under half the
/// soft ceiling. A name appearing in this list is a deliberate act that has to
/// be argued for.
const OVER_SOFT_CEILING: &[&str] = &[];

/// The count below which this census is not measuring the crate at all.
const SOURCE_FLOOR: usize = 10;

/// Every `.rs` file under `src/`, as `(relative path, line count)`.
fn modules() -> Vec<(String, usize)> {
    fn walk(root: &Path, directory: &Path, into: &mut Vec<(String, usize)>) {
        for entry in std::fs::read_dir(directory).expect("src/ is readable") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                walk(root, &path, into);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let name = path
                    .strip_prefix(root)
                    .expect("a path under src/")
                    .to_string_lossy()
                    .replace('\\', "/");
                let contents = std::fs::read_to_string(&path).expect("a source file");
                into.push((name, contents.lines().count()));
            }
        }
    }
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = Vec::new();
    walk(&root, &root, &mut found);
    found.sort();
    found
}

/// No module is over the hard ceiling unless it is listed with a reason.
#[test]
fn tc_475_030_no_module_is_over_the_hard_ceiling() {
    let modules = modules();
    assert!(
        modules.len() >= SOURCE_FLOOR,
        "anti-vacuity floor: at least {SOURCE_FLOOR} modules expected, saw {} — a census that \
         finds nothing proves nothing",
        modules.len()
    );

    for (name, lines) in &modules {
        if let Some((_, reason)) = ALLOWED_ABOVE_HARD_CEILING
            .iter()
            .find(|(allowed, _)| allowed == name)
        {
            assert!(
                *lines > HARD_CEILING,
                "{name} is {lines} lines, under the ceiling, but still carries an exemption \
                 ({reason}); remove the entry"
            );
            continue;
        }
        assert!(
            *lines <= HARD_CEILING,
            "{name} is {lines} lines, over the {HARD_CEILING}-line hard ceiling (quoin#464). \
             Split it on responsibility. Do not raise the ceiling."
        );
    }
}

/// The modules above the soft ceiling are the ones named here.
#[test]
fn tc_475_031_the_modules_over_the_soft_ceiling_are_the_ones_named_here() {
    let over: Vec<String> = modules()
        .into_iter()
        .filter(|(_, lines)| *lines > SOFT_CEILING)
        .map(|(name, _)| name)
        .collect();
    assert_eq!(
        over, OVER_SOFT_CEILING,
        "the set of modules over the {SOFT_CEILING}-line soft ceiling changed. Splitting one \
         out, or letting a new one cross, is a deliberate act: update OVER_SOFT_CEILING and \
         say which it was and why."
    );
}
