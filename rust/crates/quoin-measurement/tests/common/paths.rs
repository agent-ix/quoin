// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Shared repository paths for the quoin#479 criterion suites.
//!
//! Some suites read committed fixtures; the release-producer suites build
//! synthetic inputs in temporary repositories. Keep the common repository-root
//! calculation here so a change to the crate's depth is a one-line change.

use std::path::{Path, PathBuf};

/// The committed measurement-store fixture root used by the FR-056..FR-058 suites.
#[allow(
    dead_code,
    reason = "the shared path module is included by suites that use different helpers"
)]
pub(crate) fn fixture_store() -> PathBuf {
    repo_root()
        .join("tests")
        .join("fixtures")
        .join("evidence")
        .join("measurement-store")
}

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
