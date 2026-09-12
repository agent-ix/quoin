// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Unit-test scaffolding. Compiled only under `cfg(test)`.

use std::path::PathBuf;

/// A per-test scratch directory, removed and recreated on each call.
///
/// Deliberately not a `tempfile` dependency: one function, two callers, and a
/// dev-dependency that exists for a `mkdir` is a supply-chain entry nobody
/// reviews. Named by the caller so two tests running in parallel cannot share
/// a tree.
pub(crate) fn scratch(name: &str) -> PathBuf {
    let base = std::env::temp_dir()
        .join("quoin-quire-tests")
        .join(format!("{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).expect("scratch directory");
    base
}
