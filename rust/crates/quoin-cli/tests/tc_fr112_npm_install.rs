// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Verify the npm-managed update refusal through the relocated native CLI.

#![cfg(unix)]
#![allow(clippy::expect_used, reason = "test setup failures should be explicit")]

use ix_trace_rs::trace;
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn snapshot(root: &Path) -> std::io::Result<BTreeMap<PathBuf, Option<Vec<u8>>>> {
    fn visit(
        root: &Path,
        current: &Path,
        found: &mut BTreeMap<PathBuf, Option<Vec<u8>>>,
    ) -> std::io::Result<()> {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .expect("walked path remains within installation")
                .to_path_buf();
            if entry.file_type()?.is_dir() {
                found.insert(relative, None);
                visit(root, &path, found)?;
            } else if entry.file_type()?.is_file() {
                found.insert(relative, Some(fs::read(path)?));
            }
        }
        Ok(())
    }

    let mut found = BTreeMap::new();
    visit(root, root, &mut found)?;
    Ok(found)
}

#[trace("FR-112-AC-3")]
#[test]
fn update_refuses_npm_managed_binary_without_changing_it() {
    let fixture = tempfile::tempdir().expect("npm installation fixture");
    let executable = fixture
        .path()
        .join("node_modules/@agent-ix/quoin-linux-x64/bin/quoin");
    let installation = executable
        .parent()
        .and_then(Path::parent)
        .expect("installed package directory");
    fs::create_dir_all(executable.parent().expect("installed bin directory"))
        .expect("npm package path");
    fs::copy(env!("CARGO_BIN_EXE_quoin"), &executable).expect("copy native CLI into package");
    let mut permissions = fs::metadata(&executable)
        .expect("installed CLI metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&executable, permissions).expect("npm CLI is executable");
    let before = fs::read(&executable).expect("installed bytes before update");
    let installation_before = snapshot(installation).expect("snapshot npm installation");

    let output = Command::new(&executable)
        .arg("update")
        .env("HOME", fixture.path())
        .env("IX_HOME", fixture.path().join("empty-home"))
        .output()
        .expect("invoke npm-installed quoin update");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 refusal diagnostic");
    assert!(
        stderr.contains("this quoin was installed by npm"),
        "{stderr}"
    );
    assert!(
        stderr.contains("npm install -g @agent-ix/quoin@latest"),
        "{stderr}"
    );
    assert_eq!(
        fs::read(&executable).expect("installed bytes after update"),
        before,
        "npm-owned executable remains byte-identical"
    );
    assert_eq!(
        snapshot(installation).expect("snapshot npm installation after update"),
        installation_before,
        "the npm package tree remains unchanged"
    );
}
