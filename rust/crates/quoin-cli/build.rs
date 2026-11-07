// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Bake the same truthful version provenance into the native CLI that the
//! retired Node entrypoint carried: a clean tag is semver, while a development
//! build identifies its distance and commit. Source archives have no Git
//! metadata, so their Cargo package version remains the deterministic fallback.

use std::path::Path;
use std::process::Command;

mod build_git;

fn main() {
    // `.git` is a text file in a linked worktree, so watching it never fires;
    // watch the files Git itself says move HEAD, branch and tags.
    for path in build_git::watched_paths(Path::new(".")) {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    let version = Command::new("git")
        .args(["describe", "--tags", "--dirty"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|version| version.trim().trim_start_matches('v').to_owned())
        .filter(|version| !version.is_empty())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_owned());
    println!("cargo:rustc-env=QUOIN_VERSION={version}");
}
