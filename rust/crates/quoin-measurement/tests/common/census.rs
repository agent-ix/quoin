// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Enumerating a module tree, for the static censuses.
//!
//! Four of the quoin#479 criterion suites assert that a set of modules reaches
//! for no process or network. Each must agree on what counts as a source file,
//! or two censuses could cover different trees while both reporting success.

use std::path::{Path, PathBuf};

/// Every `.rs` file at or under `path`, sorted.
///
/// A single file is returned as itself, so a census may name one module or a
/// whole directory without its caller branching. The extension is read through
/// [`Path::extension`] rather than a `.ends_with(".rs")` test, which is
/// case-sensitive and would also match a file named exactly `.rs`.
pub(crate) fn rust_files(path: &Path) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }
    let mut out = Vec::new();
    let entries = std::fs::read_dir(path)
        .unwrap_or_else(|error| panic!("{}: not readable: {error}", path.display()));
    for entry in entries {
        let child = entry.expect("a directory entry").path();
        if child.is_dir() {
            out.extend(rust_files(&child));
        } else if child.extension().is_some_and(|extension| extension == "rs") {
            out.push(child);
        }
    }
    out.sort();
    out
}
