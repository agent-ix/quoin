// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Compile the engine pin into the crate from the one place it is declared.
//!
//! `src/quire/contract.ts` recorded the quire-rs revision as a **hand-edited
//! constant** next to a hand-edited table of schema hashes, and the refresh
//! script was what kept them honest. Here the pin lives in `Cargo.toml`, which
//! is also what Cargo resolves the dependency from, so there is exactly one
//! site — and this script reads it rather than a second copy.
//!
//! Deliberately reads the **manifest**, not `Cargo.lock`: this crate has no
//! committed lock (it becomes a member of the workspace quoin#375 owns, and
//! that workspace owns the lock). The manifest's `rev` is the exact object
//! Cargo checks out, so it is the fact worth reporting.

// A build script's only way to fail a build is to panic: there is no caller to
// return a `Result` to, and cargo reports the panic message as the build error.
// Scoped to this file, which compiles into no shipped artifact — the production
// panic surface these lints govern is `src/`, and the crate root keeps them.
#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "a build script fails a build by panicking; see the note above"
)]

use std::path::PathBuf;

// The parser that enforces the pin, shared verbatim with `tests/manifest_pin.rs`.
// A build script is never built as a test target, so a `#[cfg(test)]` module
// here would never run; including one file in both targets is what makes the
// assertions about `field` a real gate.
include!("build_support/manifest_pin.rs");

fn main() {
    let manifest_dir = PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo always sets CARGO_MANIFEST_DIR"),
    );
    let manifest_path = manifest_dir.join("Cargo.toml");
    println!("cargo:rerun-if-changed={}", manifest_path.display());
    println!("cargo:rerun-if-changed=build_support/manifest_pin.rs");

    let manifest = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", manifest_path.display()));
    let line = manifest
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("quire-rs"))
        .unwrap_or_else(|| {
            panic!(
                "{} declares no `quire-rs` dependency; the engine pin must be \
                 declared exactly once, in the manifest",
                manifest_path.display()
            )
        });

    let version = field(line, "version").unwrap_or_else(|| {
        panic!("the quire-rs dependency must pin an exact `version = \"=X.Y.Z\"`")
    });
    let revision = field(line, "rev").unwrap_or_else(|| {
        panic!(
            "the quire-rs dependency must pin a `rev`: a branch or tag is a claim \
             about whatever resolved that morning, not about these bytes"
        )
    });
    assert!(
        version.starts_with('='),
        "the quire-rs version must be an exact `=` pin, got {version:?}"
    );
    assert!(
        revision.len() == 40 && revision.bytes().all(|b| b.is_ascii_hexdigit()),
        "the quire-rs rev must be a full 40-character object id, got {revision:?}"
    );

    println!(
        "cargo:rustc-env=QUOIN_QUIRE_ENGINE_VERSION={}",
        version.trim_start_matches('=')
    );
    println!("cargo:rustc-env=QUOIN_QUIRE_ENGINE_REVISION={revision}");
}
