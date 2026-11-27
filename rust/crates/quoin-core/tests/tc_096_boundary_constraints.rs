// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! FR-096 checks for the request and result boundary constraints.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "test assertions intentionally panic on a violated contract"
)]

use ix_trace_rs::trace;
use std::path::{Path, PathBuf};

fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[trace("FR-096-AC-7")]
#[test]
fn generated_boundary_type_sources_exclude_foreign_runtime_types_and_reject_a_planted_type() {
    fn check_type_surface(source: &str) -> Result<(), String> {
        let forbidden = [
            "NodeRuntime",
            "NodeObject",
            "NpmRuntimeValue",
            "OclifCommand",
        ];
        forbidden
            .into_iter()
            .find(|name| {
                source
                    .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                    .any(|t| t == *name)
            })
            .map_or(Ok(()), |name| Err(name.to_owned()))
    }

    let root = crate_root().join("src");
    let mut pending = vec![root];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(directory).expect("boundary source directory is readable") {
            let path = entry.expect("directory entry is readable").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }
    assert!(!files.is_empty(), "boundary source population is non-empty");
    for path in files {
        let source = std::fs::read_to_string(&path).expect("boundary source is UTF-8");
        assert_eq!(check_type_surface(&source), Ok(()), "{}", path.display());
    }

    assert_eq!(
        check_type_surface("pub struct NodeRuntime { value: String }"),
        Err("NodeRuntime".to_owned())
    );
}

#[trace("FR-096-CON-1")]
#[test]
fn every_exposed_operation_is_command_shaped() {
    let operations = quoin_core::dispatch::OPERATIONS;
    assert!(!operations.is_empty(), "operation surface is non-empty");
    for operation in operations {
        let (domain, name) = operation
            .split_once('.')
            .unwrap_or_else(|| panic!("operation is not <domain>.<op>: {operation}"));
        assert!(!domain.is_empty() && !name.is_empty(), "{operation}");
        assert!(
            !name.contains('.'),
            "operation has more than one discriminator: {operation}"
        );
        assert!(
            domain
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
            "{operation}"
        );
        assert!(
            name.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
            "{operation}"
        );
        assert_ne!(
            *operation, "core.canonical_json",
            "primitive serialization is not an operation"
        );
    }
}

#[trace("FR-096-CON-2")]
#[test]
fn diagnostics_do_not_replace_or_reclassify_a_structured_payload() {
    use ix_cli_kit::exit::Outcome;
    use quoin_core::protocol::{Response, canonical_json};
    let response = Response::partial(
        serde_json::json!({"verdict":"incomplete"}),
        &quoin_core::error::CoreError::new(
            quoin_core::error::CoreErrorCode::Refused,
            "arbitrary text",
        ),
    );
    assert_eq!(response.outcome, Outcome::Partial);
    assert_eq!(response.outcome.code(), 1);
    assert_eq!(response.payload["verdict"], "incomplete");
    assert_eq!(response.diagnostics.len(), 1);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&canonical_json(&response.payload).unwrap())
            .unwrap()["verdict"],
        "incomplete"
    );
    assert_eq!(response.diagnostics[0].message, "arbitrary text");
}

#[trace("FR-096-CON-3")]
#[test]
fn workspace_manifest_and_core_crate_are_inside_this_repository() {
    let root = crate_root()
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let workspace_manifest = root
        .join("rust/Cargo.toml")
        .canonicalize()
        .expect("workspace manifest");
    assert!(
        workspace_manifest.starts_with(&root),
        "{}",
        workspace_manifest.display()
    );
    let manifest =
        std::fs::read_to_string(&workspace_manifest).expect("workspace manifest is readable");
    assert!(
        manifest.contains("members = [\"crates/*\"]"),
        "the workspace includes every in-repository crate"
    );
    assert!(
        crate_root()
            .canonicalize()
            .expect("crate root")
            .starts_with(root)
    );
}

#[trace("FR-096-CON-5")]
#[test]
fn quoin_core_manifest_declares_a_library_and_no_binary_target() {
    let manifest_path = crate_root().join("Cargo.toml");
    let manifest = std::fs::read_to_string(&manifest_path).expect("crate manifest is readable");
    assert!(
        manifest.contains("[lib]"),
        "quoin-core exposes a library target"
    );
    assert!(
        !manifest.contains("[[bin]]"),
        "quoin-core has no executable target"
    );
    assert!(
        !crate_root().join("src/main.rs").exists(),
        "quoin-core has no main.rs"
    );
}

#[trace("FR-100-CON-3")]
#[test]
fn measurement_portfolio_read_leaves_the_governed_corpus_fixture_unchanged() {
    use quoin_core::protocol::{Diagnostic, Response};
    use quoin_core::runtime::{RuntimeSettings, dispatch};
    use serde_json::{Value, json};

    fn snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
        let mut files = Vec::new();
        let mut pending = vec![root.to_path_buf()];
        while let Some(directory) = pending.pop() {
            for entry in std::fs::read_dir(directory).expect("fixture directory is readable") {
                let path = entry.expect("fixture entry is readable").path();
                if path.is_dir() {
                    pending.push(path);
                } else {
                    files.push(path);
                }
            }
        }
        files.sort();
        files
            .into_iter()
            .map(|path| {
                (
                    path.clone(),
                    std::fs::read(path).expect("fixture file is readable"),
                )
            })
            .collect()
    }

    let fixture = crate_root().join("../quoin-measurement/tests/fixtures/portfolio-tree");
    let before = snapshot(&fixture);
    assert!(
        !before.is_empty(),
        "the corpus fixture population is non-empty"
    );
    let request = json!({
        "locations": [
            fixture.join("alpha").to_string_lossy(),
            fixture.join("bravo").to_string_lossy(),
        ]
    });
    let response = dispatch(
        "measurement.build_portfolio",
        &request,
        &RuntimeSettings::default(),
    )
    .unwrap_or_else(|error| Response {
        payload: Value::Null,
        diagnostics: vec![Diagnostic::from(&error)],
        outcome: error.outcome(),
    });
    assert_eq!(response.outcome.code(), 0, "{:?}", response.diagnostics);
    assert!(
        response.payload["repositories"].as_array().is_some(),
        "{}",
        response.payload
    );
    assert_eq!(
        snapshot(&fixture),
        before,
        "measurement wrote a corpus byte"
    );
}
