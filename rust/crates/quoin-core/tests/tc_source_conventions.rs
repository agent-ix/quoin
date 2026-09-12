// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The conventions `.claude/skills/rust-style/SKILL.md` states, checked.
//!
//! A source-inspection test is a last resort (rust-review §2), and it is the
//! right resort here: no runtime path can observe whether a file carries an
//! SPDX header, and an idiom doc nobody enforces is a doc that stops being
//! true. Every rule below is one the style doc states in prose.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    // crates/quoin-core -> crates -> <workspace root>
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Unbound: deliberately carries no `Trace:` line. It was tagged NFR-024,
/// which is about bounded staged coexistence — successors, expiry dates and
/// the allowance manifest — and says nothing about a licence header, so the
/// tag bound nothing and would have mis-bound if it had. The only SPDX
/// criterion in `spec/` is FR-081-AC-1, which governs the rendered public
/// repository template, not this workspace. No criterion requires an SPDX
/// header on `rust/**/*.rs`; that is a specification gap, and the test stands
/// untagged until one exists.
/// Provenance: quoin#375, quoin#390
#[test]
fn tc_375_every_rust_file_carries_the_agpl_spdx_header() {
    let root = workspace_root();
    let mut files = Vec::new();
    rust_sources(&root.join("crates"), &mut files);
    assert!(
        files.len() >= 8,
        "found only {} sources; the walk is broken",
        files.len()
    );

    let mut missing = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).unwrap();
        let head: String = text.lines().take(2).collect::<Vec<_>>().join("\n");
        if !head.contains("SPDX-License-Identifier: AGPL-3.0-or-later")
            || !head.contains("Copyright (C) 2026 Agent-IX")
        {
            missing.push(file.strip_prefix(&root).unwrap().display().to_string());
        }
    }
    assert_eq!(missing, Vec::<String>::new());
}

/// Trace: NFR-027-AC-2, NFR-027-M-2
/// Provenance: quoin#375
///
/// Backs the population half of AC-2 — every crate manifest opts in — and the
/// `Crates taking lints from [workspace.lints]` metric. It does NOT back AC-2's
/// planted-override half: nothing here demonstrates that a per-crate override
/// fails the gate.
#[test]
fn tc_375_every_crate_opts_into_the_workspace_lint_policy() {
    // The style doc says there is no exception. An exception added without
    // editing this test — and the reason in the root Cargo.toml — fails here.
    let root = workspace_root();
    let mut without = Vec::new();
    for entry in std::fs::read_dir(root.join("crates")).unwrap() {
        let manifest = entry.unwrap().path().join("Cargo.toml");
        let text = std::fs::read_to_string(&manifest).unwrap();
        if !text.contains("[lints]\nworkspace = true") {
            without.push(manifest.strip_prefix(&root).unwrap().display().to_string());
        }
    }
    assert_eq!(without, Vec::<String>::new());
}

/// Trace: NFR-026-M-2
/// Provenance: quoin#375
///
/// The metric row `Declared channel | 1.98.1` is exactly what this asserts, and
/// is the only obligation it discharges. It is NOT tagged NFR-026-AC-1, whose
/// other half is that the channel is declared in **exactly one file**: this
/// test asserts the channel value in `rust-toolchain.toml` and then asserts two
/// further `toolchain: 1.98.1` pins in `.github/workflows/build-test.yml`,
/// which NFR-026-AC-3 says a gate must reject. Tagging AC-1 would mark a
/// criterion backed by a test that exhibits the state its sibling forbids.
#[test]
fn tc_375_the_pinned_channel_and_the_declared_msrv_agree() {
    // Three files state the toolchain; a pin that disagrees with itself means
    // the gate measured something other than what the workspace declares.
    let root = workspace_root();
    let toolchain = std::fs::read_to_string(root.join("rust-toolchain.toml")).unwrap();
    let clippy = std::fs::read_to_string(root.join("clippy.toml")).unwrap();
    let cargo = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(toolchain.contains(r#"channel = "1.98.1""#), "{toolchain}");
    assert!(clippy.contains(r#"msrv = "1.98.1""#), "{clippy}");
    assert!(cargo.contains(r#"rust-version = "1.98.1""#), "{cargo}");

    // The CI workflow is the fourth site, and the one that decides what the
    // gate ACTUALLY ran on. A pin that agrees with itself in three files and
    // disagrees with the runner is the failure this assertion exists for.
    let workflow = root
        .parent()
        .unwrap()
        .join(".github/workflows/build-test.yml");
    let workflow = std::fs::read_to_string(&workflow).unwrap();
    let mut rust_pins: Vec<&str> = workflow
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("toolchain:"))
        .collect();
    rust_pins.sort_unstable();
    // Sorted, not positional: the assertion is about WHICH toolchains the
    // workflow pins, and reordering jobs is not a defect. The two 1.98.1 pins
    // are quoin's own Rust and difftest jobs. The 1.94.1 is the `test` job
    // building agent-ix/quire-cli from source — another repository's
    // toolchain, deliberately left alone, and named here so a reader does not
    // "fix" it into agreement.
    assert_eq!(
        rust_pins,
        vec![
            "toolchain: 1.94.1",
            "toolchain: 1.98.1",
            "toolchain: 1.98.1"
        ]
    );
}
