// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Exercise the native config command replacements for the retirement gate.

#![allow(
    clippy::expect_used,
    reason = "test setup failures are the intended failure report"
)]
#![cfg(unix)]

use ix_trace_rs::trace;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Output};

fn invoke(
    config_root: &std::path::Path,
    arguments: &[&str],
    editor: Option<&std::path::Path>,
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_quoin"));
    command
        .arg("--config-root")
        .arg(config_root)
        .args(arguments)
        .env("HOME", config_root)
        .env_remove("IX_HOME")
        .env_remove("IX_CONFIG_ROOT");
    if let Some(editor) = editor {
        command.env("EDITOR", editor).env_remove("VISUAL");
    }
    command.output().expect("quoin starts")
}

#[trace("FR-099-AC-8")]
#[test]
fn tc_099_native_config_commands_read_write_validate_and_edit() {
    let root = tempfile::tempdir().expect("config root exists");
    let unset = invoke(root.path(), &["config", "get", "org"], None);
    assert!(
        unset.status.success(),
        "{}",
        String::from_utf8_lossy(&unset.stderr)
    );
    assert!(String::from_utf8_lossy(&unset.stdout).contains("quoin.org is unset"));

    let set = invoke(root.path(), &["config", "set", "org", "acme"], None);
    assert!(
        set.status.success(),
        "{}",
        String::from_utf8_lossy(&set.stderr)
    );
    let read = invoke(root.path(), &["config", "get", "org"], None);
    assert!(
        read.status.success(),
        "{}",
        String::from_utf8_lossy(&read.stderr)
    );
    assert!(String::from_utf8_lossy(&read.stdout).contains("quoin.org: acme"));

    let doctor = invoke(root.path(), &["config", "doctor"], None);
    assert!(
        doctor.status.success(),
        "{}",
        String::from_utf8_lossy(&doctor.stderr)
    );
    assert!(String::from_utf8_lossy(&doctor.stdout).contains("quoin valid"));

    let editor = root.path().join("editor.sh");
    std::fs::write(&editor, "#!/bin/sh\nprintf 'org: edited\\n' > \"$1\"\n")
        .expect("editor fixture is written");
    let mut permissions = std::fs::metadata(&editor)
        .expect("editor metadata")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&editor, permissions).expect("editor is executable");
    let edited = invoke(root.path(), &["config", "edit"], Some(&editor));
    assert!(
        edited.status.success(),
        "{}",
        String::from_utf8_lossy(&edited.stderr)
    );
    let read = invoke(root.path(), &["config", "get", "org"], None);
    assert!(
        read.status.success(),
        "{}",
        String::from_utf8_lossy(&read.stderr)
    );
    assert!(String::from_utf8_lossy(&read.stdout).contains("quoin.org: edited"));
}

#[trace("FR-027-AC-10")]
#[test]
fn tc_367_config_get_renders_numeric_matrix_request_limit() {
    let root = tempfile::tempdir().expect("config root exists");
    let set = invoke(
        root.path(),
        &["config", "set", "matrix_max_request_bytes", "123456"],
        None,
    );
    assert!(
        set.status.success(),
        "{}",
        String::from_utf8_lossy(&set.stderr)
    );

    let read = invoke(
        root.path(),
        &["config", "get", "matrix_max_request_bytes"],
        None,
    );
    assert!(
        read.status.success(),
        "{}",
        String::from_utf8_lossy(&read.stderr)
    );
    assert!(
        String::from_utf8_lossy(&read.stdout).contains("quoin.matrix_max_request_bytes: 123456"),
        "{}",
        String::from_utf8_lossy(&read.stdout)
    );
}

#[trace("FR-027-AC-10")]
#[test]
fn tc_367_config_set_refuses_matrix_limit_above_core_ceiling() {
    let root = tempfile::tempdir().expect("config root exists");
    let output = invoke(
        root.path(),
        &["config", "set", "matrix_max_request_bytes", "83886081"],
        None,
    );
    assert!(!output.status.success());
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    assert!(diagnostic.contains("QC009_CONFIG_SCHEMA"), "{diagnostic}");
    let unset = invoke(
        root.path(),
        &["config", "get", "matrix_max_request_bytes"],
        None,
    );
    assert!(unset.status.success());
    assert!(String::from_utf8_lossy(&unset.stdout).contains("is unset"));
}
