// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Unit-test scaffolding. Compiled only under `cfg(test)`.

#![allow(
    clippy::expect_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

/// A per-test scratch directory that removes itself.
///
/// Deliberately not a `tempfile` dependency: one function, two callers, and a
/// dev-dependency that exists for a `mkdir` is a supply-chain entry nobody
/// reviews. Named by the caller so two tests running in parallel cannot share
/// a tree.
///
/// Cleaning on entry only is the defect agent-ix/quoin#184 records for the
/// TypeScript side, where `mkdtempSync` trees were created per test and never
/// unlinked: the suite passes, and the machine accumulates one tree per test
/// per run forever. Entry-cleaning makes the *next* run deterministic but
/// leaves the last run's tree behind permanently, so the teardown is here as a
/// `Drop` rather than as a line every caller must remember to write.
pub(crate) struct Scratch {
    path: PathBuf,
}

impl Scratch {
    /// Create (or re-create) the named scratch tree.
    pub(crate) fn new(name: &str) -> Self {
        let path = std::env::temp_dir()
            .join("quoin-quire-tests")
            .join(format!("{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("scratch directory");
        Self { path }
    }

    /// The directory, which exists for as long as this value does.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        // Best effort: a teardown that panics would replace a real test
        // failure with a directory-removal failure, and a panic while
        // unwinding from one aborts the process.
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::Scratch;

    #[test]
    fn tc_379_025_a_scratch_tree_is_removed_when_its_guard_drops() {
        let path = {
            let scratch = Scratch::new("teardown");
            let path = scratch.path().to_path_buf();
            assert!(path.is_dir(), "the guard creates its tree");
            path
        };
        assert!(!path.exists(), "the guard removes its tree on drop");
    }
}
