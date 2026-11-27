// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Negative architecture constraints on the in-process Rust boundary (FR-096).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the boundary constraint assertions are test failures"
)]

use ix_trace_rs::trace;
use quoin_core::dispatch::OPERATIONS;
use quoin_core::runtime::{RuntimeSettings, dispatch};
use std::path::{Path, PathBuf};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate is under rust/crates")
        .parent()
        .expect("crates is under rust")
        .parent()
        .expect("rust workspace is inside the repository")
        .to_path_buf()
}

#[trace("FR-096-CON-1")]
#[test]
fn boundary_routes_are_command_shaped_not_function_exports() {
    assert!(
        !OPERATIONS.is_empty(),
        "the boundary operation set is populated"
    );
    for operation in OPERATIONS {
        let mut parts = operation.split('.');
        assert!(
            parts.next().is_some_and(|domain| !domain.is_empty()),
            "{operation}"
        );
        assert!(
            parts.next().is_some_and(|name| !name.is_empty()),
            "{operation}"
        );
        assert!(
            parts.next().is_none(),
            "operation has extra function-shaped segments: {operation}"
        );
        assert!(
            !operation
                .chars()
                .any(|character| matches!(character, '(' | ')' | ':')),
            "operation exposes a function signature: {operation}"
        );
    }

    let primitive = dispatch(
        "core.canonical_json",
        &serde_json::json!({"value": {}}),
        &RuntimeSettings::default(),
    )
    .expect_err("canonical JSON serialization is not a boundary operation");
    assert_eq!(primitive.code.as_str(), "CORE_UNKNOWN_OP");
    assert_eq!(primitive.outcome().code(), 3);
}

#[trace("FR-096-CON-2")]
#[test]
fn caller_verdict_comes_from_structured_outcomes_and_diagnostics() {
    let success = dispatch(
        "core.ping",
        &serde_json::json!({"echo": "stderr: refused"}),
        &RuntimeSettings::default(),
    )
    .expect("the operation returns a typed response");
    assert_eq!(success.outcome.code(), 0);
    assert!(success.outcome.carries_payload());
    assert_eq!(success.payload["echo"], "stderr: refused");
    assert!(success.diagnostics.is_empty());

    let refusal = dispatch(
        "core.ping",
        &serde_json::json!({"echo": "x".repeat(quoin_core::ops::core::MAX_ECHO_BYTES + 1)}),
        &RuntimeSettings::default(),
    )
    .expect_err("oversize echo is returned as a typed boundary error");
    assert_eq!(refusal.code.as_str(), "CORE_REFUSED");
    assert_eq!(refusal.outcome().code(), 2);
}

#[trace("FR-096-CON-3")]
#[test]
fn rust_workspace_root_is_nested_inside_the_quoin_repository() {
    let repository = repository_root();
    let rust_workspace = repository.join("rust");
    let manifest = rust_workspace.join("Cargo.toml");

    assert!(
        repository.join(".git").exists(),
        "repository marker exists at {}",
        repository.display()
    );
    assert!(
        manifest.is_file(),
        "workspace manifest exists at {}",
        manifest.display()
    );
    assert!(rust_workspace.starts_with(&repository));
    assert_eq!(
        rust_workspace.file_name().and_then(|name| name.to_str()),
        Some("rust")
    );
}

#[trace("FR-096-CON-5")]
#[test]
fn quoin_core_manifest_declares_only_a_library_target() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = std::fs::read_to_string(crate_root.join("Cargo.toml"))
        .expect("quoin-core manifest is readable");
    assert!(manifest.contains("[lib]\nname = \"quoin_core\""));
    assert!(
        !manifest.contains("[[bin]]"),
        "explicit executable target added"
    );
    assert!(
        !crate_root.join("src/main.rs").exists(),
        "implicit executable target added"
    );
    assert!(
        !crate_root.join("src/bin").exists(),
        "nested executable target added"
    );
}
