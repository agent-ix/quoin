// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `module-ok` fixture, read from `quire-fixtures`, shared by the
//! module-install tests. One definition per crate: an integration-test module
//! cannot be imported from another crate's tests.

#![allow(
    clippy::unwrap_used,
    dead_code,
    reason = "in a test, a panic IS the failure report; each test file uses a subset"
)]

use std::fs;
use std::path::Path;

/// `semantic.semantic_core` as the module-manifest schema pinned through
/// `@agent-ix/semantic-schema` requires it. The `quire-fixtures` `module-ok`
/// manifest omits the key, so `copy_module_ok` adds it; drop this injection
/// when the pinned `@agent-ix/semantic-schema` no longer requires
/// `semantic.semantic_core`. Quoin reads nothing from the value.
const SCHEMA_REQUIRED_SEMANTIC_CORE: &str = "0.0.0";

/// A removed-on-drop copy of `module-ok` with the `semantic_core` key the
/// pinned schema requires.
pub(crate) fn module_ok_copy() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    copy_module_ok(dir.path());
    dir
}

/// Copy `module-ok` to `to`, adding the `semantic_core` key.
pub(crate) fn copy_module_ok(to: &Path) {
    copy_tree(&quire_fixtures::module_ok_dir(), to);
    let manifest = to.join("manifest.yaml");
    let text = fs::read_to_string(&manifest).unwrap();
    fs::write(
        &manifest,
        text.replace(
            "semantic:\n",
            &format!("semantic:\n  semantic_core: {SCHEMA_REQUIRED_SEMANTIC_CORE}\n"),
        ),
    )
    .unwrap();
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}
