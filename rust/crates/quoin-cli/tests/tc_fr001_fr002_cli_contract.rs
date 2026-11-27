// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! CLI-level contracts for argument binding and package version aliases.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test setup failures are the intended failure report"
)]

use std::path::Path;
use std::process::{Command, Output};

use ix_trace_rs::trace;

fn write_flow_stub(directory: &Path) -> std::path::PathBuf {
    std::fs::create_dir_all(directory).expect("stub directory is created");
    #[cfg(unix)]
    let (stub, contents) = (
        directory.join("ix-flow"),
        "#!/bin/sh\ncase \"$1\" in\n  --version) printf '0.3.1\\n' ;;\n  run) shift; printf '%s\\n' \"$@\" > \"$IX_FLOW_CAPTURE\"; printf '{\"instance_id\":\"test-run\"}\\n' ;;\n  add-item) printf '{}\\n' ;;\n  *) exit 2 ;;\nesac\n",
    );
    #[cfg(windows)]
    let (stub, contents) = (
        directory.join("ix-flow.cmd"),
        "@echo off\r\nif \"%~1\"==\"--version\" (echo 0.3.1 & exit /b 0)\r\nif \"%~1\"==\"run\" (shift & call :capture_args > \"%IX_FLOW_CAPTURE%\" & echo {\"instance_id\":\"test-run\"} & exit /b 0)\r\nif \"%~1\"==\"add-item\" (echo {} & exit /b 0)\r\nexit /b 2\r\n:capture_args\r\nif \"%~1\"==\"\" exit /b\r\necho %~1\r\nshift\r\ngoto capture_args\r\n",
    );
    std::fs::write(&stub, contents).expect("stub is written");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&stub)
            .expect("stub metadata exists")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&stub, permissions).expect("stub is executable");
    }
    stub
}

fn path_with_stub(directory: &Path) -> std::ffi::OsString {
    let mut paths = vec![directory.to_path_buf()];
    if let Some(current) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&current));
    }
    std::env::join_paths(paths).expect("stub and current PATH form a valid PATH")
}

fn invoke(arguments: &[&str], current_dir: &Path, home: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(arguments)
        .current_dir(current_dir)
        .env("IX_HOME", home)
        .env("HOME", home)
        .output()
        .expect("quoin starts")
}

/// Both supported long-flag syntaxes bind values, and repeated targets retain
/// their order through the actual CLI parser and flow launch.
#[trace("FR-001-AC-1", "FR-001-AC-4")]
#[test]
fn tc_fr001_target_value_syntaxes_bind_and_repeated_targets_accumulate() {
    let scratch = tempfile::tempdir().expect("scratch directory exists");
    let repo = scratch.path().join("repo");
    let home = scratch.path().join("ix-home");
    let skill = scratch.path().join("skill");
    let bin = scratch.path().join("bin");
    std::fs::create_dir_all(&repo).expect("repo exists");
    std::fs::create_dir_all(skill.join("workflows/to-plan"))
        .expect("workflow fixture directory exists");
    std::fs::write(skill.join("SKILL.md"), "fixture skill\n").expect("skill fixture exists");
    std::fs::write(skill.join("workflows/to-plan/def.yaml"), "id: to-plan\n")
        .expect("workflow fixture exists");
    std::fs::write(repo.join("first.md"), "first\n").expect("first target exists");
    std::fs::write(repo.join("second.md"), "second\n").expect("second target exists");
    let capture = scratch.path().join("flow-args.txt");
    write_flow_stub(&bin);

    let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args([
            "to-plan",
            "--path",
            skill.to_str().expect("skill path is UTF-8"),
            "--target=first.md",
            "--target",
            "second.md",
        ])
        .current_dir(&repo)
        .env("PATH", path_with_stub(&bin))
        .env("IX_HOME", &home)
        .env("HOME", &home)
        .env("IX_FLOW_CAPTURE", &capture)
        .output()
        .expect("quoin starts");
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let captured = std::fs::read_to_string(capture).expect("flow runner captured its arguments");
    let arguments: Vec<&str> = captured.lines().collect();
    let targets: Vec<&str> = arguments
        .windows(2)
        .filter(|pair| pair[0] == "--target")
        .map(|pair| pair[1])
        .collect();
    assert_eq!(
        targets,
        [
            repo.join("first.md")
                .to_str()
                .expect("first target path is UTF-8"),
            repo.join("second.md")
                .to_str()
                .expect("second target path is UTF-8"),
        ]
    );
}

/// Each version alias prints the same non-empty package version.
#[trace("FR-002-AC-1", "FR-002-AC-2")]
#[test]
fn tc_fr002_version_aliases_print_the_nonempty_package_version() {
    let scratch = tempfile::tempdir().expect("scratch directory exists");
    let mut observed = Vec::new();
    for alias in ["version", "--version", "-v"] {
        let output = invoke(&[alias], scratch.path(), scratch.path());
        assert!(
            output.status.success(),
            "{alias}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let printed = String::from_utf8(output.stdout)
            .expect("version output is UTF-8")
            .trim()
            .to_owned();
        assert!(!printed.is_empty(), "{alias} must print a package version");
        assert_eq!(printed, env!("QUOIN_VERSION"), "{alias}");
        observed.push(printed);
    }
    assert!(observed.windows(2).all(|pair| pair[0] == pair[1]));
}
