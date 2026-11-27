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
#[trace("FR-234-AC-1", "FR-234-AC-9")]
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
    assert_eq!(refused.stdout, [] as [u8; 0]);
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
            assert_eq!(output.stdout, [] as [u8; 0]);
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
    assert_eq!(output.stdout, [] as [u8; 0]);
    assert!(text(&output.stderr).contains("could not write SACM report"));
}

fn successful_text(output: &Output) -> String {
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    text(&output.stdout)
}

fn authored_export_args(repo: &Path, argument: &str) -> Vec<String> {
    [
        "--repo",
        repo.to_str().unwrap(),
        "--argument",
        argument,
        "--decisions",
        repo.join("decisions.json").to_str().unwrap(),
        "--as-of",
        "2026-08-15T00:00:00.000Z",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn run_authored_args(args: &[String], extra: &[&str]) -> Output {
    let arguments: Vec<_> = args
        .iter()
        .map(String::as_str)
        .chain(extra.iter().copied())
        .collect();
    quoin(&arguments)
}

#[trace("FR-228-AC-9", "FR-234-AC-5")]
#[test]
fn sacm_preserves_authored_citation_resolution_without_a_fabricated_run() {
    let repo = repo_with_argument("argument-unresolved-ref.md");
    let args = authored_export_args(repo.path(), "AA-10181");
    let evidence = write(
        repo.path(),
        "resolved.json",
        &read_fixture("evidence-resolved.json"),
    );
    let path = evidence.to_str().unwrap();
    let reference = "evidence://discharge/widget-1018";
    for (state, declaration, selection) in [
        ("resolved", "asserted", &["--evidence", path][..]),
        ("unresolved", "needsSupport", &[][..]),
    ] {
        let gsn_args: Vec<_> = std::iter::once("--gsn")
            .chain(selection.iter().copied())
            .collect();
        let gsn: serde_json::Value =
            serde_json::from_str(&successful_text(&run_authored_args(&args, &gsn_args)))
                .expect("GSN model");
        let nodes = gsn["nodes"].as_array().unwrap();
        let citation = nodes
            .iter()
            .find(|node| node["source"] == reference)
            .expect("citation retained");
        assert_eq!(citation["kind"], "solution");
        assert_eq!(
            citation["evidence"]["references"],
            serde_json::json!([reference])
        );
        if state == "resolved" {
            assert_eq!(citation["sourceData"]["state"], "resolved");
        } else {
            assert!(citation["sourceData"].get("state").is_none());
        }
        assert!(citation["evidence"].get("suites").is_none());
        let id = citation["id"].as_str().unwrap();
        let sacm_args: Vec<_> = std::iter::once("--sacm")
            .chain(selection.iter().copied())
            .collect();
        let xmi = successful_text(&run_authored_args(&args, &sacm_args));
        assert!(
            xmi.lines()
                .any(|line| line.contains(&format!("xmi:id=\"{id}\""))
                    && line.contains("sacm:ArtifactReference")
                    && !line.contains("referencedArtifactElement")),
            "{xmi}"
        );
        assert!(
            xmi.contains(&format!("name content=\"{reference}\"")),
            "{xmi}"
        );
        let declaration = format!("assertionDeclaration=\"{declaration}\"");
        assert!(
            xmi.lines().any(|line| line.contains("sacm:Claim")
                && line.contains("xmi:id=\"G1\"")
                && line.contains(&declaration)),
            "{xmi}"
        );
        assert!(
            xmi.lines()
                .any(|line| line.contains("sacm:AssertedEvidence")
                    && line.contains(&format!("source=\"{id}\""))
                    && line.contains("target=\"G1\"")
                    && line.contains("isCounter=\"false\"")
                    && line.contains(&declaration)),
            "{xmi}"
        );
        assert!(
            !xmi.contains("xmi:type=\"sacm:Artifact\"") && !xmi.contains("sha256:"),
            "{xmi}"
        );
        if state == "unresolved" {
            let reason = citation["sourceData"]["reason"]
                .as_str()
                .expect("authored resolution reason");
            assert!(xmi.contains(&format!("not supporting: {reason}")), "{xmi}");
        }
    }
}

#[trace("FR-226-AC-10")]
#[test]
fn sacm_preserves_goal_and_assumption_counter_inferences_and_reports_strategy_challenges() {
    let repo = repo_with_argument("argument-unresolved-ref.md");
    let document = String::from_utf8(read_fixture("argument-unresolved-ref.md")).unwrap();
    let mut additions = String::new();
    for (id, target) in [
        ("CH-OPEN-GOAL", "CLAIM-1018-B"),
        ("CH-OPEN-ASSUMPTION", "ASM-1018-B"),
        ("CH-OPEN-STRATEGY", "ARG-1018-B"),
    ] {
        use std::fmt::Write as _;
        write!(additions, "  - id: {id}\n    target: {target}\n    statement: A challenge requiring review.\n    status: open\n    owner: release-owner\n").unwrap();
    }
    write(
        repo.path(),
        "spec/argument.md",
        document
            .replace("relationships:\n", &format!("{additions}relationships:\n"))
            .as_bytes(),
    );
    let args = authored_export_args(repo.path(), "AA-10181");
    let gsn: serde_json::Value =
        serde_json::from_str(&successful_text(&run_authored_args(&args, &["--gsn"]))).unwrap();
    let nodes = gsn["nodes"].as_array().unwrap();
    let node_id = |source: &str| {
        nodes.iter().find(|node| node["source"] == source).unwrap()["id"]
            .as_str()
            .unwrap()
    };
    let output = run_authored_args(&args, &["--sacm"]);
    let xmi = successful_text(&output);
    for (counter, target) in [
        ("CH-OPEN-GOAL", "CLAIM-1018-B"),
        ("CH-OPEN-ASSUMPTION", "ASM-1018-B"),
    ] {
        assert!(
            xmi.lines()
                .any(|line| line.contains("sacm:AssertedInference")
                    && line.contains("isCounter=\"true\"")
                    && line.contains(&format!("source=\"{}\"", node_id(counter)))
                    && line.contains(&format!("target=\"{}\"", node_id(target)))),
            "{xmi}"
        );
    }
    for source in [
        "CH-OPEN-GOAL",
        "CH-OPEN-ASSUMPTION",
        "CH-OPEN-STRATEGY",
        "CLAIM-1018-B",
    ] {
        assert!(
            xmi.lines().any(|line| line.contains("sacm:Claim")
                && line.contains("assertionDeclaration=\"needsSupport\"")
                && line.contains(&format!("xmi:id=\"{}\"", node_id(source)))),
            "{xmi}"
        );
    }
    assert!(
        !xmi.contains(&format!("source=\"{}\"", node_id("CH-OPEN-STRATEGY"))),
        "{xmi}"
    );
    assert!(text(&output.stderr).contains(
        "unmapped: CH-OPEN-STRATEGY: challenges ARG-1018-B: strategy target is not a SACM Claim"
    ));
}

#[trace("FR-227-AC-5")]
#[test]
fn sacm_reports_nested_authored_edges_without_approximating_them() {
    let repo = repo_with_argument("argument-no-refs.md");
    let document = String::from_utf8(read_fixture("argument-no-refs.md")).unwrap();
    let nested = r"  - id: ARG-NESTED
    statement: Describe an inner argument.
    supports: ARG-1018-A
    sufficiency_criteria:
      - The inner criterion remains open.
  - id: ARG-DIRECT
    statement: Describe an independent argument.
    supports: CLAIM-1018-A
    sufficiency_criteria:
      - The independent criterion remains open.
";
    write(
        repo.path(),
        "spec/argument.md",
        document
            .replace("assumptions:\n", &format!("{nested}assumptions:\n"))
            .as_bytes(),
    );
    let args = authored_export_args(repo.path(), "AA-10180");
    let gsn: serde_json::Value =
        serde_json::from_str(&successful_text(&run_authored_args(&args, &["--gsn"]))).unwrap();
    let nodes = gsn["nodes"].as_array().unwrap();
    let node_id = |source: &str| {
        nodes.iter().find(|node| node["source"] == source).unwrap()["id"]
            .as_str()
            .unwrap()
    };
    let report = repo.path().join("report.json");
    let output = run_authored_args(
        &args,
        &["--sacm", "--sacm-report", report.to_str().unwrap()],
    );
    let xmi = successful_text(&output);
    for source in [
        "ARG-1018-A",
        "ARG-NESTED",
        "ARG-DIRECT",
        "CLAIM-1018-A",
        "ARG-NESTED:The inner criterion remains open.",
    ] {
        assert!(
            xmi.contains(&format!("xmi:id=\"{}\"", node_id(source))),
            "{xmi}"
        );
    }
    let inner = node_id("ARG-NESTED:The inner criterion remains open.");
    let direct = node_id("ARG-DIRECT");
    assert!(!xmi.contains(&format!("source=\"{inner}\"")), "{xmi}");
    assert!(
        xmi.lines()
            .any(|line| line.contains("sacm:AssertedInference")
                && line.contains(&format!("reasoning=\"{direct}\""))),
        "{xmi}"
    );
    let reason =
        "supported_by ARG-NESTED: nested reasoning has no SACM relationship between strategies";
    assert!(text(&output.stderr).contains(&format!("unmapped: ARG-1018-A: {reason}")));
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
    assert!(
        report["unmapped"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["source"] == "ARG-1018-A" && entry["reason"] == reason),
        "{report}"
    );
}
