// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! PLAT-966's unbacked-claim refusal, driven through the real `quoin
//! assurance --argument` binary end to end (PLAT-1018).
//!
//! Before this ticket, the refusal — a supported claim with no
//! `evidence_refs`, or one citing an unresolved reference, reads as `open`
//! rather than vacuously `supported` — was covered only by
//! `quoin-assurance`'s crate-internal `authored::build::tests` (`tc_1925_*`).
//! This file spawns the native `quoin` binary, exactly as a caller would:
//! a real `spec/` document on disk, real `--decisions`/`--evidence` files,
//! and an assertion on the real stdout and exit code.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The retained fixture bodies this file reads: the argument documents,
/// `decisions.json`, `evidence.json` (which does not resolve the cited
/// reference) and `evidence-resolved.json` (which does).
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/fixtures/assurance-argument-cli")
        .canonicalize()
        .expect("the retained assurance-argument CLI fixtures are present")
}

fn read_fixture(name: &str) -> Vec<u8> {
    std::fs::read(fixtures().join(name)).expect("the fixture is present")
}

/// A repository root that exists only for one test, with `spec/` holding one
/// `AssuranceArgument` document under it — the shape `quoin assurance
/// --argument` reads its frontmatter from.
fn repo_with_argument(document: &str) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("a temporary directory");
    write(root.path(), "spec/argument.md", &read_fixture(document));
    write(
        root.path(),
        "decisions.json",
        &read_fixture("decisions.json"),
    );
    root
}

/// Run one `quoin assurance <args>` invocation, capturing its output.
fn quoin(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .arg("assurance")
        .args(args)
        .output()
        .expect("the native quoin binary runs")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("the native shell emits UTF-8")
}

fn write(root: &Path, relative: &str, bytes: &[u8]) -> PathBuf {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, bytes).unwrap();
    path
}

/// A supported claim authored with an empty `evidence_refs` array reads as
/// `open`, naming "the top claim has no evidence references", through the
/// real binary — never vacuously `supported` because the caller cited
/// nothing.
///
/// Provenance: PLAT-966, PLAT-1018
#[trace("FR-047-AC-8")]
#[test]
fn tc_1018_a_claim_with_no_evidence_references_is_open_through_the_real_binary() {
    let root = repo_with_argument("argument-no-refs.md");

    let output = quoin(&[
        "--repo",
        root.path().to_str().unwrap(),
        "--argument",
        "AA-10180",
        "--decisions",
        root.path().join("decisions.json").to_str().unwrap(),
        "--as-of",
        "2026-08-15T00:00:00.000Z",
        "--json",
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "quoin assurance --argument answers a complete view, not a command \
         failure, even when the claim is unbacked: stderr {}",
        text(&output.stderr)
    );
    let rendered: serde_json::Value =
        serde_json::from_str(text(&output.stdout).trim()).expect("--json emits the view as JSON");
    assert_eq!(rendered["topClaim"]["status"], serde_json::json!("open"));
    assert_eq!(
        rendered["topClaim"]["reasons"],
        serde_json::json!(["the top claim has no evidence references"])
    );
}

/// A supported claim citing an evidence reference the `--evidence` index
/// never resolves reads as `open`, naming the specific reference, through the
/// real binary — distinctly from citing nothing at all.
///
/// Provenance: PLAT-966, PLAT-1018
#[trace("FR-047-AC-8")]
#[test]
fn tc_1018_a_claim_citing_an_unresolved_reference_is_open_through_the_real_binary() {
    let root = repo_with_argument("argument-unresolved-ref.md");
    let evidence = write(root.path(), "evidence.json", &read_fixture("evidence.json"));

    let output = quoin(&[
        "--repo",
        root.path().to_str().unwrap(),
        "--argument",
        "AA-10181",
        "--decisions",
        root.path().join("decisions.json").to_str().unwrap(),
        "--evidence",
        evidence.to_str().unwrap(),
        "--as-of",
        "2026-08-15T00:00:00.000Z",
        "--json",
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "quoin assurance --argument answers a complete view, not a command \
         failure, even when the claim is unbacked: stderr {}",
        text(&output.stderr)
    );
    let rendered: serde_json::Value =
        serde_json::from_str(text(&output.stdout).trim()).expect("--json emits the view as JSON");
    assert_eq!(rendered["topClaim"]["status"], serde_json::json!("open"));
    assert_eq!(
        rendered["topClaim"]["reasons"],
        serde_json::json!([
            "the top claim's evidence reference evidence://discharge/widget-1018 \
             does not resolve in the evidence store"
        ])
    );
}

/// The control for the case above: the SAME argument document, with an
/// `--evidence` index that resolves its reference, reads as `supported` with no
/// reasons. Without it, the unresolved-reference test would pass just as well
/// if the binary dropped `--evidence` on the floor — an absent index resolves
/// nothing either — so it is this case that proves the refusal comes from the
/// index's content and that the fixture is otherwise clean.
///
/// Provenance: PLAT-966, PLAT-1018
#[trace("FR-047-AC-8")]
#[test]
fn tc_1018_the_same_claim_with_its_reference_resolved_is_supported_through_the_real_binary() {
    let root = repo_with_argument("argument-unresolved-ref.md");
    let evidence = write(
        root.path(),
        "evidence.json",
        &read_fixture("evidence-resolved.json"),
    );

    let output = quoin(&[
        "--repo",
        root.path().to_str().unwrap(),
        "--argument",
        "AA-10181",
        "--decisions",
        root.path().join("decisions.json").to_str().unwrap(),
        "--evidence",
        evidence.to_str().unwrap(),
        "--as-of",
        "2026-08-15T00:00:00.000Z",
        "--json",
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr {}",
        text(&output.stderr)
    );
    let rendered: serde_json::Value =
        serde_json::from_str(text(&output.stdout).trim()).expect("--json emits the view as JSON");
    assert_eq!(
        rendered["topClaim"]["status"],
        serde_json::json!("supported")
    );
    assert_eq!(rendered["topClaim"]["reasons"], serde_json::json!([]));
}

/// Provenance: agent-ix/quoin#PLAT-1447, agent-ix/quoin#PLAT-1645.
#[trace("FR-221-AC-1", "FR-221-AC-9")]
#[test]
fn explicit_gsn_export_is_supported_and_conflicts_with_json() {
    let root = repo_with_argument("argument-no-refs.md");
    let decisions = root.path().join("decisions.json");
    let common = [
        "--repo",
        root.path().to_str().unwrap(),
        "--argument",
        "AA-10180",
        "--decisions",
        decisions.to_str().unwrap(),
        "--as-of",
        "2026-08-15T00:00:00.000Z",
    ];
    let mut args = common.to_vec();
    args.push("--gsn");
    let output = quoin(&args);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let model: serde_json::Value =
        serde_json::from_str(text(&output.stdout).trim()).expect("GSN output is JSON");
    assert_eq!(model["format"], "quoin.gsn-case/v1");
    let schema_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../quoin-assurance/schemas/gsn-case-v1.schema.json");
    let schema: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&schema_path).expect("read shipped GSN schema"))
            .expect("shipped GSN schema is JSON");
    let validator = quoin_jsonschema::SchemaValidator::compile_vendored(&schema_path, &schema, &[])
        .expect("shipped GSN schema compiles");
    assert!(validator.is_valid(&model), "{:?}", validator.errors(&model));

    let mut diagram_args = common.to_vec();
    diagram_args.push("--gsn-diagram");
    let diagram = quoin(&diagram_args);
    assert_eq!(diagram.status.code(), Some(0), "{}", text(&diagram.stderr));
    assert!(text(&diagram.stdout).starts_with("flowchart TB\n"));

    let mut conflicting = common.to_vec();
    conflicting.extend(["--gsn", "--json"]);
    let refused = quoin(&conflicting);
    assert_ne!(refused.status.code(), Some(0));
    assert!(refused.stdout.is_empty());
}

#[trace("FR-226-AC-5")]
#[test]
fn sacm_output_flags_are_exclusive() {
    let root = tempfile::tempdir().expect("scratch repository");
    let path = root.path().join("report.json");
    for incompatible in ["--json", "--gsn", "--gsn-diagram"] {
        for flags in [
            vec!["--sacm", incompatible],
            vec![
                "--sacm",
                "--sacm-report",
                path.to_str().expect("UTF-8 path"),
                incompatible,
            ],
            vec![
                "--sacm-report",
                path.to_str().expect("UTF-8 path"),
                incompatible,
            ],
        ] {
            let output = quoin(&flags);
            assert_eq!(output.status.code(), Some(3), "{}", text(&output.stderr));
            assert!(output.stdout.is_empty());
            assert!(text(&output.stderr).contains(incompatible));
            assert!(!path.exists());
        }
    }
}

#[trace("FR-226-AC-4", "FR-227-AC-2", "FR-227-AC-3", "FR-227-AC-4")]
#[test]
fn sacm_authored_command_retains_report_and_refuses_failed_publication() {
    let repo = repo_with_argument("argument-no-refs.md");
    let decisions = repo.path().join("decisions.json");
    let common = [
        "--repo",
        repo.path().to_str().expect("UTF-8 path"),
        "--argument",
        "AA-10180",
        "--decisions",
        decisions.to_str().expect("UTF-8 path"),
        "--as-of",
        "2026-08-15T00:00:00.000Z",
        "--sacm",
    ];
    let report_path = repo.path().join("report.json");
    let mut args = common.to_vec();
    args.extend(["--sacm-report", report_path.to_str().expect("UTF-8 path")]);
    let first = quoin(&args);
    assert_eq!(first.status.code(), Some(0), "{}", text(&first.stderr));
    let xmi = text(&first.stdout);
    assert!(xmi.contains("xmi:type=\"sacm:AssuranceCasePackage\""));
    assert!(xmi.contains("CLAIM-1018-A"));
    assert!(!xmi.contains("ix://example.invalid/widget/AP-1018"));
    assert_eq!(
        text(&first.stderr),
        "unmapped: ix://example.invalid/widget/AP-1018: authored relationship has no declared GSN edge mapping\n"
    );
    let report = std::fs::read(&report_path).expect("report retained");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&report).expect("report is JSON"),
        serde_json::json!({
            "unmapped": [{"source": "ix://example.invalid/widget/AP-1018",
                "reason": "authored relationship has no declared GSN edge mapping"}],
            "unreachable": [],
        })
    );
    write(
        repo.path(),
        "spec/other.md",
        &read_fixture("argument-unresolved-ref.md"),
    );
    std::fs::rename(
        repo.path().join("spec/argument.md"),
        repo.path().join("spec/zz-argument.md"),
    )
    .expect("change the order of corpus files");
    let repeated = quoin(&args);
    assert_eq!(repeated.status.code(), Some(0));
    assert_eq!(first.stdout, repeated.stdout);
    assert_eq!(first.stderr, repeated.stderr);
    assert_eq!(
        report,
        std::fs::read(report_path).expect("report retained again")
    );

    let mut failed = common.to_vec();
    failed.extend(["--sacm-report", repo.path().to_str().expect("UTF-8 path")]);
    let output = quoin(&failed);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(text(&output.stderr).contains("could not write SACM report"));
}
