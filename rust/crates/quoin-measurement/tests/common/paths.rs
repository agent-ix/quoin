// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The repository root, for the quoin#479 criterion suites.
//!
//! All seven suites read committed evidence under `spec/`, so all seven need
//! this. It is here once rather than seven times so that a change to the crate's
//! depth in the tree is a one-line change.

use std::path::{Path, PathBuf};

/// The repository root, from this crate's manifest directory.
///
/// Unresolved on purpose: `canonicalize` would follow symlinks, and the
/// committed fixtures record lexical paths.
pub(crate) fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
}
