// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Copying a committed tree so a test may mutate its copy.
//!
//! The evidence bytes under `spec/` are immutable (NFR-025), so a suite that
//! needs to mutate an input copies the tree into a temporary directory and
//! mutates that. Five of the quoin#479 criterion suites do this; it had been
//! written five times, in two spellings that differed only in panic wording.

use std::path::Path;

/// Copy a directory tree, recursively.
///
/// Every failure panics naming the path it was reading or writing: a copy that
/// silently skipped an unreadable entry would leave a suite asserting against a
/// smaller tree than it believed it had.
pub(crate) fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to)
        .unwrap_or_else(|error| panic!("{}: not writable: {error}", to.display()));
    let entries = std::fs::read_dir(from)
        .unwrap_or_else(|error| panic!("{}: not readable: {error}", from.display()));
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        let target = to.join(path.file_name().expect("a file name"));
        if path.is_dir() {
            copy_tree(&path, &target);
        } else {
            std::fs::copy(&path, &target)
                .unwrap_or_else(|error| panic!("{}: not copied: {error}", path.display()));
        }
    }
}
