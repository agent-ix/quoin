// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The catalog projection through the real `quoin-core` process (quoin#373).
//!
//! The crate tests own YAML projection decisions. This test owns the production
//! host seam: candidate discovery, one-level module-root location, skeleton
//! directory reading, semantic-contract publication, operation routing, and
//! canonical stdout are properties only the real subprocess can prove.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::fs;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

/// The repository root: `rust/crates/quoin-core` up three levels.
fn repo_root() -> PathBuf {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..3 {
        root.pop();
    }
    root
}

/// Run one catalog operation against the binary's real host grant.
fn run(op: &str, request: &Value) -> (i32, String, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_quoin-core"))
        .arg(op)
        .env("QUOIN_SEMANTIC_ROOT", repo_root().join("src/semantic"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("catalog binary starts");
    child
        .stdin
        .as_mut()
        .expect("stdin is piped")
        .write_all(request.to_string().as_bytes())
        .expect("request writes");
    let output = child.wait_with_output().expect("catalog binary exits");
    (
        output.status.code().expect("catalog has an exit status"),
        String::from_utf8(output.stdout).expect("stdout is UTF-8 JSON"),
        String::from_utf8(output.stderr).expect("stderr is UTF-8 JSON"),
    )
}

/// Trace: FR-096, FR-101
/// Provenance: quoin#373
#[test]
fn tc_373_catalog_load_discovers_a_nested_module_through_the_real_boundary() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let parent = temporary.path().join("candidates");
    fs::create_dir_all(parent.join("00-not-a-module")).expect("empty sibling");
    let module = parent.join("01-catalog-fixture");
    fs::create_dir_all(module.join("skeletons")).expect("module skeletons");
    fs::write(
        module.join("manifest.yaml"),
        "name: catalog-fixture\nversion: 1.0.0\nartifact_types:\n  - name: FR\n    frontmatter_schema_ref: schemas/fr.json\nobject_types:\n  - name: entity\n    data_schema: {type: object}\n",
    )
    .expect("manifest writes");
    fs::write(module.join("skeletons/FR.md"), "# FR\n").expect("skeleton writes");

    let (status, stdout, stderr) = run("catalog.load", &json!({ "roots": [parent] }));
    assert_eq!(status, 0, "{stderr}");
    assert_eq!(stderr, "");
    let payload: Value = serde_json::from_str(&stdout).expect("catalog payload");
    assert_eq!(payload["modules"][0]["name"], "catalog-fixture");
    assert_eq!(payload["modules"][0]["version"], "1.0.0");
    assert_eq!(payload["entries"][0]["name"], "FR");
    assert_eq!(
        payload["entries"][0]["skeletonPath"],
        module.join("skeletons/FR.md").to_string_lossy().as_ref()
    );
    assert_eq!(
        payload["entries"][1]["dataSchema"],
        json!({ "type": "object" })
    );
}
