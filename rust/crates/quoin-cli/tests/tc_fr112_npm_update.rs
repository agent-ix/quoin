// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! An npm-managed installation defers updates to npm without changing its files.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "test assertions panic to report an invalid CLI contract"
)]

use ix_trace_rs::trace;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn snapshot(root: &Path, relative: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
    let directory = root.join(relative);
    for entry in fs::read_dir(&directory).expect("package directory remains readable") {
        let entry = entry.expect("package entry is readable");
        let path = relative.join(entry.file_name());
        if entry
            .file_type()
            .expect("package entry type is readable")
            .is_dir()
        {
            snapshot(root, &path, files);
        } else {
            files.insert(
                path,
                fs::read(root.join(relative).join(entry.file_name()))
                    .expect("package file is readable"),
            );
        }
    }
}

#[trace("FR-112-AC-3")]
#[test]
fn tc_fr112_update_refuses_an_npm_install_without_changing_its_files() {
    let temporary = tempfile::tempdir().expect("temporary npm installation is created");
    let package_root = temporary
        .path()
        .join("node_modules/@agent-ix/quoin-linux-x64");
    let binary_dir = package_root.join("bin");
    fs::create_dir_all(&binary_dir).expect("npm package bin directory is created");

    let native_binary = Path::new(env!("CARGO_BIN_EXE_quoin"));
    let binary_name = native_binary
        .file_name()
        .expect("built Quoin binary has a filename");
    let installed_binary = binary_dir.join(binary_name);
    fs::copy(native_binary, &installed_binary).expect("built binary is copied into node_modules");

    let mut before = BTreeMap::new();
    snapshot(temporary.path(), Path::new("node_modules"), &mut before);

    let output = Command::new(&installed_binary)
        .arg("update")
        .env("HOME", temporary.path())
        .env("USERPROFILE", temporary.path())
        .env("IX_HOME", temporary.path().join(".ix"))
        .output()
        .expect("npm-installed quoin starts");
    assert!(
        !output.status.success(),
        "npm update refusal exits non-zero"
    );
    let stderr = String::from_utf8(output.stderr).expect("Quoin stderr is UTF-8");
    assert!(stderr.contains("installed by npm"), "{stderr}");
    assert!(
        stderr.contains("npm install -g @agent-ix/quoin@latest"),
        "{stderr}"
    );

    let mut after = BTreeMap::new();
    snapshot(temporary.path(), Path::new("node_modules"), &mut after);
    assert_eq!(
        after, before,
        "npm update refusal leaves installation bytes unchanged"
    );
}
