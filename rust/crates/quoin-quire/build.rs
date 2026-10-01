// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Compile the linked engine version into the crate from `Cargo.toml`, the one
//! place the dependency is declared.

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

// The manifest line parser.
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

    println!(
        "cargo:rustc-env=QUOIN_QUIRE_ENGINE_VERSION={}",
        version.trim_start_matches('=')
    );
}
