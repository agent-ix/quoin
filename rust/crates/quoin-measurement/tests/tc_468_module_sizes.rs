// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! No module in this crate grows past the size a reader can hold.
//!
//! # Why a census rather than a review note
//!
//! The ceiling is a review criterion (quoin#464): 700 lines hard, 500 soft.
//! A criterion enforced only by review is enforced only when someone is
//! looking, and `quoin-change-assurance`'s `model/record.rs` reached 1,078
//! lines that way before quoin#457 split it. So
//! the ceiling is read off the crate's own `src/` tree here instead, with a
//! floor underneath it: a census that stopped finding files would otherwise
//! pass by finding nothing.
//!
//! There is no allow-list. If one is ever needed, it belongs here as an entry
//! carrying the one-line reason that module is exempt — not as a raised
//! ceiling, which would exempt every module at once.

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

/// The count below which this census is not measuring the crate at all.
const SOURCE_FLOOR: usize = 20;

/// Modules permitted above [`SOFT_CEILING`], named so the next one to cross it
/// is visible rather than quiet.
///
/// `plans.rs` crossed it under PLAT-936: catching a `#`-prefixed comment
/// inside a fenced code block from ending the "Comparison and Enforcement"
/// section early (quoin#583's blocking review finding) needs fence-tracking
/// state threaded through the heading walk, and the tests proving it holds
/// live in the same file as the rest of this module's unit tests, per this
/// crate's existing convention.
///
/// `store/apparatus/walk.rs` and `validate/stack.rs` crossed it under
/// PLAT-985 (quoin#600 review): the walk's directory, per-plan and
/// write-wide limit tests, and the plan-id/path key-validation tests, live
/// beside the code they cover, per the same convention. `store/apparatus.rs`
/// crossed the hard ceiling under the same ticket and was split on
/// responsibility — the filesystem walk into `walk.rs`, what a write records
/// in `mod.rs`.
///
/// `report/verdict.rs` crossed it under PLAT-958 part 2: the `gate` stage's
/// verdict (`gate`, `gate_baseline`) shares its `InconclusiveReason` vocabulary
/// and its `usable`/`earlier_values` evidence pool with `ratchet` and
/// `target`, so splitting it out would either duplicate that pool or make it
/// `pub(super)` across two files for one stage's sake; the file stays under
/// the 700-line hard ceiling (quoin#464).
const NAMED_OVER_SOFT_CEILING: &[&str] = &[
    "plans.rs",
    "report/verdict.rs",
    "store/apparatus/walk.rs",
    "validate/stack.rs",
];

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
fn tc_468_no_module_is_over_the_hard_ceiling() {
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
             Split it on responsibility, as `validate.ts` was split into \
             `validate/mod.rs`, `stack.rs` and `read.rs` here. Do not raise the ceiling."
        );
    }
}

/// The modules above the soft ceiling are named, so crossing it is visible.
#[test]
fn tc_468_the_modules_over_the_soft_ceiling_are_the_ones_named_here() {
    let over: Vec<String> = modules()
        .into_iter()
        .filter(|(_, lines)| *lines > SOFT_CEILING)
        .map(|(name, lines)| format!("{name} ({lines})"))
        .collect();
    let expected: Vec<String> = NAMED_OVER_SOFT_CEILING
        .iter()
        .map(std::string::ToString::to_string)
        .collect();
    let over_names: Vec<String> = over
        .iter()
        .map(|entry| entry.split(" (").next().unwrap_or(entry).to_owned())
        .collect();
    assert_eq!(
        over_names, expected,
        "the set of modules over the {SOFT_CEILING}-line soft ceiling changed (saw {over:?}). \
         Splitting one out, or letting a new one cross, is a deliberate act: update \
         NAMED_OVER_SOFT_CEILING and say which it was."
    );
}
