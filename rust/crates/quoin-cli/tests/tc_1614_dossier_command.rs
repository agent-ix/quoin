// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! The `quoin dossier` command surface (FR-191, PLAT-1614).
//!
//! The documents are the test-only stub generators of `quoin-dossier`, which
//! reach the spawned `quoin` binary through its `test-stubs` feature. The
//! `quire` the export tests run is a recording script written into the test's
//! temporary directory; the release-record integration test uses installed
//! Quire and the real PDF and DOCX exporters.
//!
//! Provenance: PLAT-1614, PLAT-1436.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;
use serde_json::Value;

mod dropped_relation_support;

const APPROVAL_SCOPE: &str = "Approval entries in this export are treated as unbound to the exact bytes of this DOS-REL document because Quoin has not compared their source-record digests to those bytes. They report only the source records and verdicts in their inputs.";
const VERIFIER_IDENTITY: &str =
    "Verifier identity: not provided as a distinct field by the source approval contracts.";

fn quoin(work: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .arg("dossier")
        .args(args)
        .current_dir(work)
        .output()
        .expect("quoin starts")
}

fn code(output: &Output) -> i32 {
    output.status.code().expect("quoin exits with a status")
}

fn version() -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .arg("--version")
        .output()
        .expect("quoin version command starts");
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

fn json(output: &Output) -> Value {
    serde_json::from_str(&stdout(output)).unwrap_or_else(|error| {
        panic!("stdout is not JSON ({error}): {}", stdout(output));
    })
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

/// A `quire` that records its arguments, one per line, and either writes
/// `--output` and succeeds or refuses with `stderr` and `status`.
fn fake_quire(work: &Path, status: i32, refusal: &str) -> PathBuf {
    let script = work.join("fake-quire");
    let body = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"{args}\"\nout=\nwhile [ $# -gt 0 ]; do\n  if [ \"$1\" = --output ]; then out=$2; fi\n  shift\ndone\nif [ {status} -ne 0 ]; then\n  echo partial > \"$out\"\n  printf '%s' '{refusal}' >&2\n  exit {status}\nfi\necho bundle > \"$out\"\n",
        args = work.join("quire-args").display(),
    );
    fs::write(&script, body).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    script
}

fn recorded_args(work: &Path) -> Vec<String> {
    read(&work.join("quire-args"))
        .lines()
        .map(str::to_owned)
        .collect()
}

fn run_git(work: &Path, args: &[&str]) {
    let result = Command::new("git")
        .args(args)
        .current_dir(work)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

fn init_temp_git(work: &Path) {
    run_git(work, &["init", "-q"]);
    run_git(work, &["config", "user.name", "Dossier test"]);
    run_git(work, &["config", "user.email", "dossier@example.invalid"]);
}

fn init_temp_git_sha256(work: &Path) {
    run_git(work, &["init", "-q", "--object-format=sha256"]);
    run_git(work, &["config", "user.name", "Dossier test"]);
    run_git(work, &["config", "user.email", "dossier@example.invalid"]);
}

fn commit_paths(work: &Path, message: &str, paths: &[&str]) {
    let mut add = vec!["add"];
    add.extend_from_slice(paths);
    run_git(work, &add);
    run_git(work, &["commit", "-qm", message]);
}

fn generate_stubs(work: &Path, docs: &[&str]) {
    init_temp_git(work);
    let mut args = vec!["generate", "--version", "1.0"];
    for doc in docs {
        args.extend(["--doc", doc]);
    }
    let output = quoin(work, &args);
    assert!(matches!(code(&output), 0 | 1), "{}", stderr(&output));
    commit_paths(work, "dossier fixtures", &["dossier"]);
}

/// Every dossier document carries the exact version its executable reports
/// (FR-191-AC-31), independently of its document version.
#[trace("FR-191-AC-31")]
#[test]
fn generated_documents_carry_the_running_quoin_version() {
    let version = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .arg("--version")
        .output()
        .expect("quoin starts");
    assert!(version.status.success());
    let version = String::from_utf8(version.stdout)
        .unwrap()
        .trim_end()
        .to_owned();
    assert!(!version.is_empty());

    let work = tempfile::tempdir().unwrap();
    generate_stubs(work.path(), &["stub-a", "stub-b"]);
    for document in ["DOS-STUB-A", "DOS-STUB-B"] {
        let text = read(&work.path().join("dossier").join(format!("{document}.md")));
        let frontmatter = text.split("---").nth(1).unwrap();
        let producer_version = frontmatter
            .split_once("producer_identity:\n")
            .and_then(|(_, identity)| {
                identity
                    .lines()
                    .find_map(|line| line.strip_prefix("  version: "))
            })
            .unwrap_or_else(|| panic!("{document} has no producer version: {frontmatter}"));
        assert_eq!(producer_version, format!("\"{version}\""));
        assert!(frontmatter.contains("version: \"1.0\""));
    }
}

/// Architecture generation carries valid design output and a relation gap
/// through a dropped required relation instead of refusing before generation.
#[trace("FR-193-AC-10")]
#[test]
fn architecture_dossier_keeps_valid_entries_and_names_dropped_relations() {
    let work = tempfile::tempdir().unwrap();
    fs::write(
        work.path().join("manifest.yaml"),
        dropped_relation_support::MODULE,
    )
    .unwrap();
    fs::create_dir_all(work.path().join("spec")).unwrap();
    fs::write(
        work.path().join("spec/FR-001.md"),
        dropped_relation_support::SPEC,
    )
    .unwrap();
    let output = quoin(
        work.path(),
        &[
            "generate",
            "--version",
            "1.0",
            "--doc",
            "architecture",
            "--scope",
            ".",
            "--module",
            ".",
        ],
    );
    assert_eq!(code(&output), 1, "{}", stderr(&output));
    let document = read(&work.path().join("dossier/DOS-ARCH.md"));
    assert!(document.contains("FR-001"), "{document}");
    assert!(
        document.contains("requirement-without-design"),
        "{document}"
    );
    assert!(document.contains("design_relation_dropped"), "{document}");
    assert!(
        document.contains("connection-carries-classification"),
        "{document}"
    );
    assert!(
        !document.contains("design_relations_unavailable"),
        "{document}"
    );
}

/// A dropped design relation still explains why no design block was returned.
#[trace("FR-193-AC-11")]
#[test]
fn architecture_dossier_reports_a_dropped_relation_without_design_blocks() {
    let work = tempfile::tempdir().unwrap();
    let module = dropped_relation_support::MODULE.replace(
        "design_relations: [requirement-has-design]",
        "design_relations: []",
    );
    fs::write(work.path().join("manifest.yaml"), module).unwrap();
    fs::create_dir_all(work.path().join("spec")).unwrap();
    fs::write(
        work.path().join("spec/FR-001.md"),
        dropped_relation_support::SPEC,
    )
    .unwrap();
    let output = quoin(
        work.path(),
        &[
            "generate",
            "--version",
            "1.0",
            "--doc",
            "architecture",
            "--scope",
            ".",
            "--module",
            ".",
        ],
    );
    assert_eq!(code(&output), 1, "{}", stderr(&output));
    let document = read(&work.path().join("dossier/DOS-ARCH.md"));
    assert!(document.contains("design_relation_dropped"), "{document}");
    assert!(
        document.contains("connection-carries-classification"),
        "{document}"
    );
    assert!(
        !document.contains("design_relations_unavailable"),
        "{document}"
    );
    assert!(
        !document.contains("requirement-without-design"),
        "no design block was returned: {document}"
    );
}

/// Another command's failure still prints nothing on stdout: the dossier
/// report is the only one that does at exit 2 to 4.
#[trace("FR-191")]
#[test]
fn a_failure_of_another_command_prints_nothing_on_stdout() {
    let work = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args([
            "soup",
            "promote",
            "/nonexistent/sbom.json",
            "--repo",
            "/nonexistent",
        ])
        .current_dir(work.path())
        .output()
        .unwrap();
    assert_ne!(code(&output), 0);
    assert_eq!(stdout(&output), "");
}

/// A dossier failure at exit 3 states its report on stdout and exits 3.
#[trace("FR-191-AC-12")]
#[test]
fn a_dossier_source_failure_report_is_on_stdout_and_stderr_holds_the_diagnostic() {
    let work = tempfile::tempdir().unwrap();
    let output = quoin(
        work.path(),
        &["check", "--version", "1.0", "--json", "--doc", "stub-fail"],
    );
    assert_eq!(code(&output), 3);
    assert_eq!(json(&output)["source_failure"]["source"], "requirements");
    assert!(
        stderr(&output).contains("DOSSIER_SOURCE_FAILED"),
        "{}",
        stderr(&output)
    );
}

/// Provenance: PLAT-1614 acceptance 1.
#[trace("FR-191-AC-1")]
#[test]
fn generate_writes_exactly_the_selected_files_and_nothing_else() {
    let work = tempfile::tempdir().unwrap();
    fs::create_dir(work.path().join("dossier")).unwrap();
    fs::write(work.path().join("dossier/notes.md"), "keep").unwrap();
    let output = quoin(
        work.path(),
        &[
            "generate",
            "--version",
            "1.0",
            "--doc",
            "stub-a",
            "--doc",
            "stub-b",
        ],
    );
    assert_eq!(code(&output), 1, "stub-b has a gap: {}", stderr(&output));
    assert_eq!(
        names(&work.path().join("dossier")),
        ["DOS-STUB-A.md", "DOS-STUB-B.md", "notes.md"]
    );
    assert_eq!(names(work.path()), ["dossier"]);
    assert_eq!(read(&work.path().join("dossier/notes.md")), "keep");
}

#[trace("FR-191-AC-3")]
#[test]
fn an_unknown_kind_exits_2_names_it_and_writes_nothing() {
    let work = tempfile::tempdir().unwrap();
    let output = quoin(
        work.path(),
        &[
            "generate",
            "--version",
            "1.0",
            "--doc",
            "stub-a",
            "--doc",
            "nosuch",
        ],
    );
    assert_eq!(code(&output), 2);
    assert!(stderr(&output).contains("nosuch"), "{}", stderr(&output));
    assert_eq!(names(work.path()), Vec::<String>::new());
}

#[trace("FR-191-AC-14", "FR-191-AC-15")]
#[test]
fn gaps_are_bullets_in_gap_order_the_same_in_json_and_none_when_complete() {
    let work = tempfile::tempdir().unwrap();
    let output = quoin(
        work.path(),
        &[
            "generate",
            "--version",
            "1.0",
            "--json",
            "--doc",
            "stub-a",
            "--doc",
            "stub-gaps",
        ],
    );
    assert_eq!(code(&output), 1);
    let report = json(&output);
    let documents = report["documents"].as_array().unwrap();
    assert_eq!(documents[0]["status"], "complete");
    assert_eq!(documents[1]["status"], "incomplete");

    let complete = read(&work.path().join("dossier/DOS-STUB-A.md"));
    assert!(complete.ends_with("## Gaps\n\nGaps: none\n"), "{complete}");

    let gapped = read(&work.path().join("dossier/DOS-STUB-GAPS.md"));
    let expected = [
        "- `no_clause_binding` this document: no clause binding given",
        "- `no_acceptance_criteria` FR-99: has no acceptance criterion",
        "- `no_evidence` FR-100-AC-2: no evidence",
        "- `no_evidence` FR-100-AC-10: no evidence",
    ];
    let bullets: Vec<_> = gapped
        .lines()
        .filter(|line| line.starts_with("- "))
        .collect();
    assert_eq!(bullets, expected);
    assert!(gapped.trim_end().ends_with(expected[3]));

    let subjects: Vec<_> = documents[1]["gaps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|gap| gap["subject"].as_str().unwrap())
        .collect();
    assert_eq!(
        subjects,
        ["this document", "FR-99", "FR-100-AC-2", "FR-100-AC-10"]
    );
    assert_eq!(documents[1]["gaps"][0]["gap_kind"], "no_clause_binding");
}

#[trace("FR-191-AC-4", "FR-191-AC-5")]
#[test]
fn check_matches_then_differs_after_an_edit_then_differs_when_deleted_and_writes_nothing() {
    let work = tempfile::tempdir().unwrap();
    generate_stubs(work.path(), &["stub-a"]);
    let file = work.path().join("dossier/DOS-STUB-A.md");
    let check = || {
        quoin(
            work.path(),
            &["check", "--version", "1.0", "--doc", "stub-a", "--json"],
        )
    };

    let current = check();
    assert_eq!(code(&current), 0);
    assert_eq!(json(&current)["documents"][0]["comparison"], "matches");

    let mut bytes = fs::read(&file).unwrap();
    let last = bytes.len() - 2;
    bytes[last] = b'!';
    fs::write(&file, &bytes).unwrap();
    let edited = check();
    assert_eq!(code(&edited), 1);
    assert_eq!(json(&edited)["documents"][0]["comparison"], "differs");
    assert_eq!(fs::read(&file).unwrap(), bytes, "check rewrote the file");

    fs::remove_file(&file).unwrap();
    let deleted = check();
    assert_eq!(code(&deleted), 1);
    assert_eq!(json(&deleted)["documents"][0]["comparison"], "differs");
    assert!(!file.exists(), "check recreated the file");
}

#[trace("NFR-040-AC-1")]
#[test]
fn two_generate_runs_produce_identical_files_and_json() {
    let work = tempfile::tempdir().unwrap();
    let args = [
        "generate",
        "--version",
        "2",
        "--json",
        "--doc",
        "stub-b",
        "--doc",
        "stub-gaps",
    ];
    let first = quoin(work.path(), &args);
    let first_files = (
        read(&work.path().join("dossier/DOS-STUB-B.md")),
        read(&work.path().join("dossier/DOS-STUB-GAPS.md")),
    );
    let second = quoin(work.path(), &args);
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(
        first_files,
        (
            read(&work.path().join("dossier/DOS-STUB-B.md")),
            read(&work.path().join("dossier/DOS-STUB-GAPS.md"))
        )
    );
}

#[trace("FR-191-AC-12")]
#[test]
fn a_failing_source_exits_3_names_it_writes_nothing_and_keeps_existing_files() {
    let work = tempfile::tempdir().unwrap();
    fs::create_dir(work.path().join("dossier")).unwrap();
    fs::write(work.path().join("dossier/DOS-STUB-A.md"), "before").unwrap();
    let output = quoin(
        work.path(),
        &[
            "generate",
            "--version",
            "1.0",
            "--json",
            "--doc",
            "stub-a",
            "--doc",
            "stub-fail",
        ],
    );
    assert_eq!(code(&output), 3);
    let report = json(&output);
    assert_eq!(report["source_failure"]["source"], "requirements");
    assert_eq!(read(&work.path().join("dossier/DOS-STUB-A.md")), "before");
    assert_eq!(names(&work.path().join("dossier")), ["DOS-STUB-A.md"]);
}

#[trace("FR-191-AC-13", "FR-191-AC-24", "FR-191-AC-29", "FR-207-AC-10")]
#[test]
fn records_of_processing_needs_no_version_and_exports_only_with_release() {
    let work = tempfile::tempdir().unwrap();
    let spec = work.path().join("spec");
    fs::create_dir(&spec).unwrap();
    let module = spec.join("empty-module");
    fs::create_dir(&module).unwrap();
    fs::write(module.join("manifest.yaml"), "name: dossier-empty-flow\n").unwrap();
    let module_path = module.to_str().unwrap();
    assert_stub_actions_need_version(work.path());

    init_temp_git(work.path());
    let generated = quoin(
        work.path(),
        &[
            "generate",
            "--doc",
            "records-of-processing",
            "--module",
            module_path,
            "--json",
        ],
    );
    assert_eq!(code(&generated), 1, "{}", stderr(&generated));
    let report = json(&generated);
    assert_eq!(report["documents"][0]["id"], "DOS-ROP");
    let document = read(&work.path().join("dossier/DOS-ROP.md"));
    assert!(!document.lines().any(|line| line.starts_with("version:")));
    commit_paths(work.path(), "dossier", &["dossier"]);

    let checked = quoin(
        work.path(),
        &[
            "check",
            "--doc",
            "records-of-processing",
            "--module",
            module_path,
            "--json",
        ],
    );
    assert_eq!(code(&checked), 1, "{}", stderr(&checked));
    let report = json(&checked);
    assert_eq!(report["documents"][0]["id"], "DOS-ROP");
    assert_eq!(report["documents"][0]["comparison"], "matches");
    assert_eq!(
        report["documents"][0]["gaps"][0]["gap_kind"],
        "no_processing_activity"
    );

    fs::write(work.path().join("out.pdf"), b"sentinel").unwrap();
    let refusing = fake_quire(work.path(), 3, "a release or version is required");
    let refused = quoin(
        work.path(),
        &[
            "export",
            "--doc",
            "records-of-processing",
            "--module",
            module_path,
            "--to",
            "pdf",
            "--output",
            "out.pdf",
            "--quire",
            refusing.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(code(&refused), 4, "{}", stderr(&refused));
    assert_eq!(json(&refused)["quire"]["exit_code"], 3);
    assert_eq!(fs::read(work.path().join("out.pdf")).unwrap(), b"sentinel");
    assert!(
        !recorded_args(work.path())
            .iter()
            .any(|arg| arg == "--release")
    );

    let accepting = fake_quire(work.path(), 0, "");
    let exported = quoin(
        work.path(),
        &[
            "export",
            "--doc",
            "records-of-processing",
            "--module",
            module_path,
            "--release",
            "v1",
            "--to",
            "pdf",
            "--output",
            "out.pdf",
            "--quire",
            accepting.to_str().unwrap(),
            "--json",
        ],
    );
    // The empty fixture leaves DOS-ROP incomplete (`no_processing_activity`),
    // which exits 1 even though `quire export` succeeded (FR-191-AC-16).
    assert_eq!(code(&exported), 1, "{}", stderr(&exported));
    assert!(json(&exported).get("quire").is_none_or(Value::is_null));
    assert_eq!(read(&work.path().join("out.pdf")), "bundle\n");
    let args = recorded_args(work.path());
    assert!(args.windows(2).any(|pair| pair == ["--release", "v1"]));
    assert!(!args.iter().any(|arg| arg == "--version"));
}

fn assert_stub_actions_need_version(work: &Path) {
    for args in [
        &["generate", "--doc", "stub-a"][..],
        &["check", "--doc", "stub-a"][..],
        &[
            "export", "--doc", "stub-a", "--to", "pdf", "--output", "out.pdf",
        ][..],
    ] {
        let output = quoin(work, args);
        assert_eq!(code(&output), 2, "{args:?}: {}", stderr(&output));
        assert_eq!(names(work), ["spec"], "{args:?}");
    }
}

#[trace("FR-191")]
#[test]
fn as_of_is_required_by_a_document_that_records_time_and_must_be_rfc_3339() {
    let work = tempfile::tempdir().unwrap();
    let missing = quoin(
        work.path(),
        &["generate", "--version", "1", "--doc", "stub-time"],
    );
    assert_eq!(code(&missing), 2);
    let invalid = quoin(
        work.path(),
        &[
            "generate",
            "--version",
            "1",
            "--doc",
            "stub-time",
            "--as-of",
            "yesterday",
        ],
    );
    assert_eq!(code(&invalid), 2);
    assert_eq!(names(work.path()), Vec::<String>::new());
    let valid = quoin(
        work.path(),
        &[
            "generate",
            "--version",
            "1",
            "--doc",
            "stub-time",
            "--as-of",
            "2026-11-15T00:00:00Z",
        ],
    );
    assert_eq!(code(&valid), 0, "{}", stderr(&valid));
    assert!(
        read(&work.path().join("dossier/DOS-STUB-TIME.md"))
            .contains("as_of: \"2026-11-15T00:00:00Z\"\n")
    );
}

#[trace("FR-191-AC-17")]
#[test]
fn an_out_path_that_cannot_be_created_exits_2_and_writes_nothing() {
    let work = tempfile::tempdir().unwrap();
    fs::write(work.path().join("blocker"), "x").unwrap();
    let output = quoin(
        work.path(),
        &[
            "generate",
            "--version",
            "1",
            "--doc",
            "stub-a",
            "--out",
            "blocker/sub",
        ],
    );
    assert_eq!(code(&output), 2);
    assert_eq!(names(work.path()), ["blocker"]);
    assert_eq!(read(&work.path().join("blocker")), "x");
}

#[trace("FR-191-AC-6")]
#[test]
fn export_with_a_missing_quire_exits_4_and_names_the_path() {
    let work = tempfile::tempdir().unwrap();
    generate_stubs(work.path(), &["stub-a"]);
    let output = quoin(
        work.path(),
        &[
            "export",
            "--version",
            "1.0",
            "--doc",
            "stub-a",
            "--to",
            "pdf",
            "--output",
            "out.pdf",
            "--quire",
            "/nonexistent/quire",
            "--json",
        ],
    );
    assert_eq!(code(&output), 4);
    assert!(
        stderr(&output).contains("/nonexistent/quire"),
        "{}",
        stderr(&output)
    );
    let report = json(&output);
    assert!(
        report["quire"]["stderr"]
            .as_str()
            .unwrap()
            .contains("/nonexistent/quire")
    );
    assert!(!work.path().join("out.pdf").exists());
}

#[trace("FR-191-AC-7")]
#[test]
fn a_quire_refusal_exits_4_passes_through_its_status_and_stderr_and_leaves_no_output() {
    let work = tempfile::tempdir().unwrap();
    generate_stubs(work.path(), &["stub-a"]);
    let quire = fake_quire(
        work.path(),
        7,
        "working file differs from the committed blob",
    );
    let output = quoin(
        work.path(),
        &[
            "export",
            "--version",
            "1.0",
            "--doc",
            "stub-a",
            "--to",
            "pdf",
            "--output",
            "out.pdf",
            "--quire",
            quire.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(code(&output), 4);
    assert!(stderr(&output).contains("working file differs from the committed blob"));
    let report = json(&output);
    assert_eq!(report["quire"]["exit_code"], 7);
    assert_eq!(
        report["quire"]["stderr"],
        "working file differs from the committed blob"
    );
    assert!(
        !work.path().join("out.pdf").exists(),
        "a partial output was left"
    );
}

#[trace("FR-191-AC-26")]
#[test]
fn failed_quire_preserves_preexisting_output_bytes() {
    let work = tempfile::tempdir().unwrap();
    generate_stubs(work.path(), &["stub-a"]);
    let sentinel = b"preserve this output exactly\0\xff";
    fs::write(work.path().join("out.pdf"), sentinel).unwrap();
    let quire = fake_quire(work.path(), 7, "refused");
    let output = quoin(
        work.path(),
        &[
            "export",
            "--version",
            "1.0",
            "--doc",
            "stub-a",
            "--to",
            "pdf",
            "--output",
            "out.pdf",
            "--quire",
            quire.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(code(&output), 4, "{}", stderr(&output));
    assert_eq!(json(&output)["quire"]["exit_code"], 7);
    assert_eq!(fs::read(work.path().join("out.pdf")).unwrap(), sentinel);
}

#[trace("FR-191-AC-7")]
#[test]
fn failed_quire_does_not_recursively_delete_a_nonempty_output_directory() {
    let work = tempfile::tempdir().unwrap();
    generate_stubs(work.path(), &["stub-a"]);
    let quire = work.path().join("quire-directory-output");
    fs::write(
        &quire,
        "#!/bin/sh\nout=\nwhile [ $# -gt 0 ]; do\n  if [ \"$1\" = --output ]; then out=$2; fi\n  shift\ndone\nmkdir -p \"$out/inner\"\nprintf 'keep this file' > \"$out/inner/keep\"\nprintf 'refused' >&2\nexit 7\n",
    )
    .unwrap();
    fs::set_permissions(&quire, fs::Permissions::from_mode(0o755)).unwrap();

    let output = quoin(
        work.path(),
        &[
            "export",
            "--version",
            "1.0",
            "--doc",
            "stub-a",
            "--to",
            "pdf",
            "--output",
            "out.pdf",
            "--quire",
            quire.to_str().unwrap(),
            "--json",
        ],
    );

    assert_eq!(code(&output), 4, "{}", stderr(&output));
    let failure = &json(&output)["quire"];
    assert_eq!(failure["exit_code"], 7);
    let failure_stderr = failure["stderr"].as_str().unwrap();
    assert!(failure_stderr.contains("refused"), "{failure_stderr}");
    assert!(
        failure_stderr.contains("could not remove the partial output"),
        "{failure_stderr}"
    );
    assert_eq!(
        fs::read(work.path().join("out.pdf/inner/keep")).unwrap(),
        b"keep this file"
    );
}

#[trace("FR-191-AC-8", "FR-191-AC-9", "FR-191-AC-25")]
#[test]
fn export_runs_quire_once_as_a_bundle_for_two_documents_and_plain_for_one() {
    let work = tempfile::tempdir().unwrap();
    generate_stubs(work.path(), &["stub-a", "stub-b"]);
    let quire = fake_quire(work.path(), 0, "");
    let quire = quire.to_str().unwrap();

    let bundle = quoin(
        work.path(),
        &[
            "export",
            "--version",
            "1.0",
            "--doc",
            "stub-b",
            "--doc",
            "stub-a",
            "--to",
            "docx",
            "--output",
            "out.docx",
            "--release",
            "v1",
            "--approval",
            "a1.json",
            "--approval",
            "a2.json",
            "--template",
            "t.docx",
            "--quire",
            quire,
        ],
    );
    assert_eq!(
        code(&bundle),
        1,
        "stub-b is incomplete: {}",
        stderr(&bundle)
    );
    let revision = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(work.path())
        .output()
        .unwrap();
    let revision = String::from_utf8(revision.stdout)
        .unwrap()
        .trim()
        .to_owned();
    assert_eq!(
        recorded_args(work.path()),
        [
            "export",
            "dossier/DOS-STUB-A.md",
            "dossier/DOS-STUB-B.md",
            "--to",
            "docx",
            "--output",
            "out.docx",
            "--source-rev",
            revision.as_str(),
            "--release",
            "v1",
            "--approval",
            "a1.json",
            "--approval",
            "a2.json",
            "--template",
            "t.docx",
            "--bundle-title",
            "Submission Dossier",
            "--bundle-id",
            "DOS",
        ]
    );
    assert_eq!(read(&work.path().join("out.docx")), "bundle\n");

    let single = quoin(
        work.path(),
        &[
            "export",
            "--version",
            "1.0",
            "--doc",
            "stub-a",
            "--to",
            "pdf",
            "--output",
            "one.pdf",
            "--quire",
            quire,
        ],
    );
    assert_eq!(code(&single), 0, "{}", stderr(&single));
    let args = recorded_args(work.path());
    assert_eq!(args[..2], ["export", "dossier/DOS-STUB-A.md"]);
    assert!(args.contains(&"--source-rev".to_owned()) && args.contains(&revision));
    assert!(
        !args.iter().any(|arg| arg.starts_with("--bundle")),
        "{args:?}"
    );
}

#[trace("FR-191-AC-8")]
#[test]
fn export_passes_module_roots_to_quire_in_the_supplied_order() {
    let work = tempfile::tempdir().unwrap();
    generate_stubs(work.path(), &["stub-a"]);
    let quire = fake_quire(work.path(), 0, "");
    let output = quoin(
        work.path(),
        &[
            "export",
            "--version",
            "1.0",
            "--doc",
            "stub-a",
            "--module",
            "module-one",
            "--module",
            "module-two",
            "--to",
            "pdf",
            "--output",
            "out.pdf",
            "--quire",
            quire.to_str().unwrap(),
        ],
    );
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let args = recorded_args(work.path());
    let modules = args
        .windows(2)
        .filter(|pair| pair[0] == "--module")
        .map(|pair| pair[1].as_str())
        .collect::<Vec<_>>();
    assert_eq!(modules, ["module-one", "module-two"]);
}

#[trace("FR-201-AC-9", "FR-201-AC-10")]
#[test]
fn release_verify_result_is_appended_after_user_approvals_once() {
    let work = tempfile::tempdir().unwrap();
    let (snapshot, verify, quire) = prepare_release_approval_inputs(work.path());
    let git = |args: &[&str]| {
        let result = Command::new("git")
            .args(args)
            .current_dir(work.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    };
    let generated = quoin(
        work.path(),
        &[
            "generate",
            "--version",
            "1.0",
            "--doc",
            "release-record",
            "--snapshot",
            snapshot.to_str().unwrap(),
            "--release-verify",
            verify.to_str().unwrap(),
        ],
    );
    assert_eq!(code(&generated), 0, "{}", stderr(&generated));
    let generated_stub = quoin(
        work.path(),
        &["generate", "--version", "1.0", "--doc", "stub-a"],
    );
    assert_eq!(code(&generated_stub), 0, "{}", stderr(&generated_stub));
    git(&["add", "dossier/DOS-REL.md"]);
    git(&["add", "dossier/DOS-STUB-A.md"]);
    git(&["commit", "-qm", "release record"]);
    let generated_text = read(&work.path().join("dossier/DOS-REL.md"));
    let decision_at = generated_text.find("## Decision").unwrap();
    let evidence_at = generated_text.find("## Evidence at release").unwrap();
    let approval_at = generated_text.find(APPROVAL_SCOPE).unwrap();
    let identity_at = generated_text.find(VERIFIER_IDENTITY).unwrap();
    assert!(decision_at < approval_at);
    assert!(approval_at < identity_at);
    assert!(identity_at < evidence_at);
    assert!(generated_text.contains(APPROVAL_SCOPE));
    assert!(generated_text.contains(VERIFIER_IDENTITY));
    let exported = quoin(
        work.path(),
        &[
            "export",
            "--version",
            "1.0",
            "--doc",
            "release-record",
            "--doc",
            "stub-a",
            "--snapshot",
            snapshot.to_str().unwrap(),
            "--release-verify",
            verify.to_str().unwrap(),
            "--approval",
            "approval-one.json",
            "--approval",
            "approval-two.json",
            "--to",
            "pdf",
            "--output",
            "out.pdf",
            "--quire",
            quire.to_str().unwrap(),
        ],
    );
    assert_eq!(code(&exported), 0, "{}", stderr(&exported));
    let export_args = recorded_args(work.path());
    assert!(export_args[1].ends_with("DOS-REL.md"), "{export_args:?}");
    assert!(export_args[2].ends_with("DOS-STUB-A.md"), "{export_args:?}");
    let approvals: Vec<_> = export_args
        .windows(2)
        .filter(|pair| pair[0] == "--approval")
        .map(|pair| pair[1].as_str())
        .collect();
    assert_eq!(
        approvals,
        [
            "approval-one.json",
            "approval-two.json",
            verify.to_str().unwrap()
        ]
    );

    let control = quoin(
        work.path(),
        &[
            "export",
            "--version",
            "1.0",
            "--doc",
            "stub-a",
            "--snapshot",
            snapshot.to_str().unwrap(),
            "--release-verify",
            verify.to_str().unwrap(),
            "--approval",
            "approval-one.json",
            "--approval",
            "approval-two.json",
            "--to",
            "pdf",
            "--output",
            "out-without-release-record.pdf",
            "--quire",
            quire.to_str().unwrap(),
        ],
    );
    assert_eq!(code(&control), 0, "{}", stderr(&control));
    let control_args = recorded_args(work.path());
    let control_approvals: Vec<_> = control_args
        .windows(2)
        .filter(|pair| pair[0] == "--approval")
        .map(|pair| pair[1].as_str())
        .collect();
    assert_eq!(
        control_approvals,
        ["approval-one.json", "approval-two.json"]
    );
    let document = read(&work.path().join("dossier/DOS-REL.md"));
    assert!(document.contains("Approval entries in this export are treated as unbound to the exact bytes of this DOS-REL document"));
    assert!(document.contains(
        "Verifier identity: not provided as a distinct field by the source approval contracts."
    ));
}

fn prepare_release_approval_inputs(work: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let git = |args: &[&str]| {
        let result = Command::new("git")
            .args(args)
            .current_dir(work)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["config", "user.name", "Dossier test"]);
    git(&["config", "user.email", "dossier@example.invalid"]);
    let snapshot = work.join("snapshot.json");
    fs::write(
        &snapshot,
        r#"{"protocol":"quoin.release-snapshot/v1","tool_identity":{"name":"quoin","version":"0.0.0"},"clean_tree_precondition":"not_verified","as_of":"2026-11-01T00:00:00Z","baseline_commit":"baseline-commit","previous_release":null,"coverage":[],"evidence_freshness":{"bound":0,"stale":0,"suspect":0,"undischarged":0,"no run evidence":0},"open_findings":[],"soup":{"rows":[],"not_assessed":[]},"vulnerabilities":[],"anomalies":[],"change_records":[]}"#,
    )
    .unwrap();
    let verify = work.join("release-verify.json");
    fs::write(
        &verify,
        r#"{"protocol":"engineering-assurance.release-verify-result/v1","status":"released","tag":"v1","reasons":[],"approval":{"protocol":"engineering-assurance.approval-verify-result/v1","status":"approved","tag":"v1","record_id":"REC-1","trust":{"allowed_signers":"/trust/allowed_signers"},"approver":"Ada Example","meaning":["NestedReleaseMeaning"],"signed_at":"2026-11-02T03:04:05Z","signature":{"status":"signed_valid","kind":"tag"}},"release":{"id":"REL-1","name":"Release one","snapshot_path":"release/snapshot.json"},"baseline":{"name":"Baseline one","commit":"baseline-commit","items":[]},"approver":"Ada Example","meaning":["ReleaseMeaning"],"signed_at":"2026-11-02T03:04:05Z","signed_at_source":"tagger_date","accepted_anomalies":[]}"#,
    )
    .unwrap();
    for (file, principal, meaning) in [
        ("approval-one.json", "Rae One", "StandaloneMeaningOne"),
        ("approval-two.json", "Rae Two", "StandaloneMeaningTwo"),
    ] {
        fs::write(
            work.join(file),
            format!(r#"{{"protocol":"engineering-assurance.approval-verify-result/v1","status":"approved","tag":"v1","record_id":"{file}","approver":"{principal}","meaning":["{meaning}"],"signed_at":"2026-11-02T03:04:05Z","signature":{{"status":"signed_valid","kind":"tag"}}}}"#),
        )
        .unwrap();
    }
    let quire = fake_quire(work, 0, "");
    (snapshot, verify, quire)
}

#[trace("FR-201-AC-3")]
#[test]
fn missing_release_record_inputs_leave_gaps_and_exit_partial() {
    let work = tempfile::tempdir().unwrap();
    let output = quoin(
        work.path(),
        &[
            "generate",
            "--version",
            "1.0",
            "--doc",
            "release-record",
            "--json",
        ],
    );
    assert_eq!(code(&output), 1, "{}", stderr(&output));
    let report = json(&output);
    assert_eq!(report["documents"][0]["status"], "incomplete");
    let kinds: Vec<_> = report["documents"][0]["gaps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|gap| gap["gap_kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["no_snapshot", "no_release_verify_input"]);
    assert!(work.path().join("dossier/DOS-REL.md").exists());
}

#[trace("FR-201-AC-6")]
#[test]
fn malformed_release_sources_fail_before_the_selected_document_is_written() {
    let work = tempfile::tempdir().unwrap();
    let source = work.path().join("bad-snapshot.json");
    fs::write(&source, "{broken").unwrap();
    let output = quoin(
        work.path(),
        &[
            "generate",
            "--version",
            "1.0",
            "--doc",
            "release-record",
            "--snapshot",
            source.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(code(&output), 3, "{}", stderr(&output));
    let report = json(&output);
    assert!(
        report["source_failure"]["source"]
            .as_str()
            .unwrap()
            .contains("bad-snapshot.json")
    );
    assert!(!work.path().join("dossier/DOS-REL.md").exists());
}

#[trace("FR-201-AC-6")]
#[test]
fn malformed_release_verify_reasons_fail_before_the_selected_document_is_written() {
    let work = tempfile::tempdir().unwrap();
    let source = work.path().join("bad-release-verify.json");
    fs::write(
        &source,
        r#"{"protocol":"engineering-assurance.release-verify-result/v1","status":"not_released","tag":"v1","reasons":[{}]}"#,
    )
    .unwrap();
    let output = quoin(
        work.path(),
        &[
            "generate",
            "--version",
            "1.0",
            "--doc",
            "release-record",
            "--release-verify",
            source.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(code(&output), 3, "{}", stderr(&output));
    let report = json(&output);
    assert!(
        report["source_failure"]["source"]
            .as_str()
            .unwrap()
            .contains("bad-release-verify.json")
    );
    assert!(!work.path().join("dossier/DOS-REL.md").exists());
}

#[trace("FR-201-AC-8")]
#[test]
fn release_record_generation_needs_no_git_and_is_repeatable() {
    let work = tempfile::tempdir().unwrap();
    let empty_path = work.path().join("empty-path");
    fs::create_dir(&empty_path).unwrap();
    let snapshot = work.path().join("snapshot.json");
    fs::write(
        &snapshot,
        r#"{"protocol":"quoin.release-snapshot/v1","tool_identity":{"name":"quoin","version":"0.0.0"},"clean_tree_precondition":"not_verified","as_of":"2026-11-01T00:00:00Z","baseline_commit":"baseline-commit","previous_release":null,"coverage":[],"evidence_freshness":{"bound":0,"stale":0,"suspect":0,"undischarged":0,"no run evidence":0},"open_findings":[],"soup":{"rows":[],"not_assessed":[]},"vulnerabilities":[],"anomalies":[],"change_records":[]}"#,
    )
    .unwrap();
    let verify = work.path().join("release-verify.json");
    fs::write(
        &verify,
        r#"{"protocol":"engineering-assurance.release-verify-result/v1","status":"not_released","tag":"v1","reasons":["approval_not_found"]}"#,
    )
    .unwrap();
    let generate = || {
        Command::new(env!("CARGO_BIN_EXE_quoin"))
            .args([
                "dossier",
                "generate",
                "--version",
                "1.0",
                "--doc",
                "release-record",
                "--snapshot",
                snapshot.to_str().unwrap(),
                "--release-verify",
                verify.to_str().unwrap(),
            ])
            .env("PATH", &empty_path)
            .current_dir(work.path())
            .output()
            .expect("quoin starts without git on PATH")
    };
    let first = generate();
    assert_eq!(
        first.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let document = work.path().join("dossier/DOS-REL.md");
    let first_bytes = fs::read(&document).unwrap();
    let second = generate();
    assert_eq!(
        second.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert_eq!(fs::read(document).unwrap(), first_bytes);
}

#[trace("FR-201-AC-7", "FR-201-AC-9")]
#[test]
fn release_record_exports_pdf_and_docx_with_the_ea_approval_text() {
    let work = tempfile::tempdir().unwrap();
    let (snapshot, verify) = prepare_release_record_export_inputs(work.path());
    let git = |args: &[&str]| {
        let result = Command::new("git")
            .args(args)
            .current_dir(work.path())
            .output()
            .expect("git starts");
        assert!(
            result.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    };
    let generated = quoin(
        work.path(),
        &[
            "generate",
            "--version",
            "1.0",
            "--doc",
            "release-record",
            "--snapshot",
            snapshot.to_str().unwrap(),
            "--release-verify",
            verify.to_str().unwrap(),
        ],
    );
    assert_eq!(code(&generated), 0, "{}", stderr(&generated));
    git(&["add", "dossier/DOS-REL.md"]);
    git(&["commit", "-qm", "release record"]);

    for (format, output) in [("pdf", "out.pdf"), ("docx", "out.docx")] {
        let exported = quoin(
            work.path(),
            &[
                "export",
                "--version",
                "1.0",
                "--doc",
                "release-record",
                "--snapshot",
                snapshot.to_str().unwrap(),
                "--release-verify",
                verify.to_str().unwrap(),
                "--approval",
                "approval-one.json",
                "--approval",
                "approval-two.json",
                "--to",
                format,
                "--output",
                output,
                "--quire",
                "quire",
            ],
        );
        assert_eq!(code(&exported), 0, "{}", stderr(&exported));
        let extracted = if format == "pdf" {
            Command::new("pdftotext")
                .args([output, "-"])
                .current_dir(work.path())
                .output()
                .expect("pdftotext starts")
        } else {
            Command::new("pandoc")
                .args([output, "-t", "plain"])
                .current_dir(work.path())
                .output()
                .expect("pandoc starts")
        };
        assert!(
            extracted.status.success(),
            "{}",
            String::from_utf8_lossy(&extracted.stderr)
        );
        let text = String::from_utf8_lossy(&extracted.stdout);
        let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let compact = text
            .chars()
            .filter(|character| character.is_alphanumeric())
            .collect::<String>();
        assert!(compact.contains("AdaExample"), "{normalized}");
        assert!(compact.contains("ReleaseMeaning"), "{normalized}");
        assert!(compact.contains("NestedReleaseMeaning"), "{normalized}");
        assert!(compact.contains("RaeOne"), "{normalized}");
        assert!(compact.contains("StandaloneMeaningOne"), "{normalized}");
        assert!(compact.contains("RaeTwo"), "{normalized}");
        assert!(compact.contains("StandaloneMeaningTwo"), "{normalized}");
        assert!(
            normalized.contains("unbound to the exact bytes"),
            "{normalized}"
        );
        assert!(normalized.contains("Verifier identity"), "{normalized}");
    }
}

fn prepare_release_record_export_inputs(work: &Path) -> (PathBuf, PathBuf) {
    let git = |args: &[&str]| {
        let result = Command::new("git")
            .args(args)
            .current_dir(work)
            .output()
            .expect("git starts");
        assert!(
            result.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["config", "user.name", "Dossier Test"]);
    git(&["config", "user.email", "dossier@example.invalid"]);
    let snapshot = work.join("snapshot.json");
    fs::write(
        &snapshot,
        r#"{"protocol":"quoin.release-snapshot/v1","tool_identity":{"name":"quoin","version":"0.0.0"},"clean_tree_precondition":"not_verified","as_of":"2026-11-01T00:00:00Z","baseline_commit":"baseline-commit","previous_release":null,"coverage":[],"evidence_freshness":{"bound":0,"stale":0,"suspect":0,"undischarged":0,"no run evidence":0},"open_findings":[],"soup":{"rows":[],"not_assessed":[]},"vulnerabilities":[],"anomalies":[],"change_records":[]}"#,
    )
    .unwrap();
    let verify = work.join("release-verify.json");
    fs::write(
        &verify,
        r#"{"protocol":"engineering-assurance.release-verify-result/v1","status":"released","tag":"v1","reasons":[],"approval":{"protocol":"engineering-assurance.approval-verify-result/v1","status":"approved","tag":"v1","record_id":"REC-1","trust":{"allowed_signers":"/trust/allowed_signers"},"approver":"Ada Example","meaning":["NestedReleaseMeaning"],"signed_at":"2026-11-02T03:04:05Z","signature":{"status":"signed_valid","kind":"tag"}},"release":{"id":"REL-1","name":"Release one","snapshot_path":"release/snapshot.json"},"baseline":{"name":"Baseline one","commit":"baseline-commit","items":[]},"approver":"Ada Example","meaning":["ReleaseMeaning"],"signed_at":"2026-11-02T03:04:05Z","signed_at_source":"tagger_date","accepted_anomalies":[]}"#,
    )
    .unwrap();
    for (file, principal, meaning) in [
        ("approval-one.json", "Rae One", "StandaloneMeaningOne"),
        ("approval-two.json", "Rae Two", "StandaloneMeaningTwo"),
    ] {
        fs::write(
            work.join(file),
            format!(r#"{{"protocol":"engineering-assurance.approval-verify-result/v1","status":"approved","tag":"v1","record_id":"{file}","approver":"{principal}","meaning":["{meaning}"],"signed_at":"2026-11-02T03:04:05Z","signature":{{"status":"signed_valid","kind":"tag"}}}}"#),
        )
        .unwrap();
    }
    (snapshot, verify)
}

#[trace("FR-191-AC-10")]
#[test]
fn export_looks_where_out_says_and_names_the_document_missing_under_the_default() {
    let work = tempfile::tempdir().unwrap();
    init_temp_git(work.path());
    let generated = quoin(
        work.path(),
        &[
            "generate",
            "--version",
            "1.0",
            "--doc",
            "stub-a",
            "--out",
            "other",
        ],
    );
    assert_eq!(code(&generated), 0);
    commit_paths(work.path(), "current dossier in other", &["other"]);
    let quire = fake_quire(work.path(), 0, "");
    let quire = quire.to_str().unwrap();
    let base = [
        "export",
        "--version",
        "1.0",
        "--doc",
        "stub-a",
        "--to",
        "pdf",
        "--output",
        "o.pdf",
        "--quire",
        quire,
    ];

    let found = quoin(work.path(), &[&base[..], &["--out", "other"]].concat());
    assert_eq!(code(&found), 0, "{}", stderr(&found));
    fs::remove_file(work.path().join("quire-args")).unwrap();

    let missing = quoin(work.path(), &base);
    assert_eq!(code(&missing), 1);
    assert!(
        stderr(&missing).contains("DOS-STUB-A"),
        "{}",
        stderr(&missing)
    );
    assert!(
        !work.path().join("quire-args").exists(),
        "quire was started"
    );
}

#[trace("FR-191-AC-11")]
#[test]
fn export_of_a_stale_file_exits_1_names_it_and_never_starts_quire() {
    let work = tempfile::tempdir().unwrap();
    generate_stubs(work.path(), &["stub-a"]);
    fs::write(work.path().join("dossier/DOS-STUB-A.md"), "hand edited\n").unwrap();
    let quire = fake_quire(work.path(), 0, "");
    let output = quoin(
        work.path(),
        &[
            "export",
            "--version",
            "1.0",
            "--doc",
            "stub-a",
            "--to",
            "pdf",
            "--output",
            "o.pdf",
            "--quire",
            quire.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(code(&output), 1);
    assert!(stderr(&output).contains("DOS-STUB-A"));
    assert_eq!(json(&output)["documents"][0]["comparison"], "differs");
    assert!(
        !work.path().join("quire-args").exists(),
        "quire was started"
    );
}

fn assert_currency_refusal(work: &Path, output: &Output) {
    const SENTINEL: &[u8] = b"currency sentinel\0\xff";
    assert_eq!(code(output), 1, "{}", stderr(output));
    let report = json(output);
    assert_eq!(report["documents"][0]["comparison"], "differs");
    assert!(report["quire"].is_null(), "Quire was reported: {report}");
    assert!(!work.join("quire-args").exists(), "Quire was started");
    assert_eq!(fs::read(work.join("out.pdf")).unwrap(), SENTINEL);
}

#[trace("FR-191-AC-7")]
#[test]
fn export_refuses_a_committed_stale_blob_when_the_working_document_is_current() {
    let work = tempfile::tempdir().unwrap();
    init_temp_git(work.path());
    fs::create_dir(work.path().join("dossier")).unwrap();
    fs::write(
        work.path().join("dossier/DOS-STUB-A.md"),
        "committed old bytes\n",
    )
    .unwrap();
    commit_paths(work.path(), "stale dossier document", &["dossier"]);
    let generated = quoin(
        work.path(),
        &["generate", "--version", "1.0", "--doc", "stub-a"],
    );
    assert_eq!(code(&generated), 0, "{}", stderr(&generated));
    fs::write(work.path().join("out.pdf"), b"currency sentinel\0\xff").unwrap();
    let quire = fake_quire(work.path(), 0, "");
    let output = quoin(
        work.path(),
        &[
            "export",
            "--version",
            "1.0",
            "--doc",
            "stub-a",
            "--to",
            "pdf",
            "--output",
            "out.pdf",
            "--quire",
            quire.to_str().unwrap(),
            "--json",
        ],
    );
    assert_currency_refusal(work.path(), &output);
}

#[trace("FR-191-AC-7")]
#[test]
fn export_refuses_a_current_untracked_document_without_starting_quire() {
    let work = tempfile::tempdir().unwrap();
    init_temp_git(work.path());
    run_git(
        work.path(),
        &["commit", "-q", "--allow-empty", "-m", "base"],
    );
    let generated = quoin(
        work.path(),
        &["generate", "--version", "1.0", "--doc", "stub-a"],
    );
    assert_eq!(code(&generated), 0, "{}", stderr(&generated));
    fs::write(work.path().join("out.pdf"), b"currency sentinel\0\xff").unwrap();
    let quire = fake_quire(work.path(), 0, "");
    let output = quoin(
        work.path(),
        &[
            "export",
            "--version",
            "1.0",
            "--doc",
            "stub-a",
            "--to",
            "pdf",
            "--output",
            "out.pdf",
            "--quire",
            quire.to_str().unwrap(),
            "--json",
        ],
    );
    assert_currency_refusal(work.path(), &output);
}

#[trace("FR-191-AC-7")]
#[test]
fn export_accepts_a_healthy_committed_document_from_a_non_root_scope() {
    let work = tempfile::tempdir().unwrap();
    generate_stubs(work.path(), &["stub-a"]);
    fs::create_dir(work.path().join("nested-scope")).unwrap();
    let quire = fake_quire(work.path(), 0, "");
    let output = quoin(
        work.path(),
        &[
            "export",
            "--version",
            "1.0",
            "--doc",
            "stub-a",
            "--scope",
            "nested-scope",
            "--to",
            "pdf",
            "--output",
            "out.pdf",
            "--quire",
            quire.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert_eq!(json(&output)["documents"][0]["comparison"], "matches");
    assert!(
        work.path().join("quire-args").exists(),
        "Quire was not started"
    );
}

#[trace("FR-191-AC-7", "FR-191-AC-25")]
#[test]
fn export_uses_a_full_sha256_revision_for_sha256_repositories() {
    let work = tempfile::tempdir().unwrap();
    init_temp_git_sha256(work.path());
    let generated = quoin(
        work.path(),
        &["generate", "--version", "1.0", "--doc", "stub-a"],
    );
    assert_eq!(code(&generated), 0, "{}", stderr(&generated));
    commit_paths(work.path(), "current dossier document", &["dossier"]);
    let quire = fake_quire(work.path(), 0, "");
    let output = quoin(
        work.path(),
        &[
            "export",
            "--version",
            "1.0",
            "--doc",
            "stub-a",
            "--to",
            "pdf",
            "--output",
            "out.pdf",
            "--quire",
            quire.to_str().unwrap(),
        ],
    );
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    let revision = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(work.path())
        .output()
        .unwrap();
    let revision = String::from_utf8(revision.stdout)
        .unwrap()
        .trim()
        .to_owned();
    assert_eq!(revision.len(), 64);
    let args = recorded_args(work.path());
    let source_rev = args
        .windows(2)
        .find(|pair| pair[0] == "--source-rev")
        .unwrap();
    assert_eq!(source_rev[1], revision);
}

#[trace("FR-191-AC-7")]
#[test]
fn export_reports_head_resolution_failure_separately_from_document_currency() {
    let work = tempfile::tempdir().unwrap();
    let generated = quoin(
        work.path(),
        &["generate", "--version", "1.0", "--doc", "stub-a"],
    );
    assert_eq!(code(&generated), 0, "{}", stderr(&generated));
    let quire = fake_quire(work.path(), 0, "");
    let output = quoin(
        work.path(),
        &[
            "export",
            "--version",
            "1.0",
            "--doc",
            "stub-a",
            "--to",
            "pdf",
            "--output",
            "out.pdf",
            "--quire",
            quire.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(code(&output), 1, "{}", stderr(&output));
    let report = json(&output);
    assert_eq!(report["documents"][0]["comparison"], "differs");
    assert!(
        report["export_currency_failure"]
            .as_str()
            .unwrap()
            .contains("git rev-parse")
    );
    assert!(
        !work.path().join("quire-args").exists(),
        "Quire was started"
    );
}

#[trace("FR-191-AC-16")]
#[test]
fn an_incomplete_current_document_is_exported_not_refused() {
    let work = tempfile::tempdir().unwrap();
    generate_stubs(work.path(), &["stub-b"]);
    let quire = fake_quire(work.path(), 0, "");
    let output = quoin(
        work.path(),
        &[
            "export",
            "--version",
            "1.0",
            "--doc",
            "stub-b",
            "--to",
            "pdf",
            "--output",
            "o.pdf",
            "--quire",
            quire.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(
        code(&output),
        1,
        "incomplete documents exit 1: {}",
        stderr(&output)
    );
    assert_eq!(json(&output)["documents"][0]["status"], "incomplete");
    assert!(
        work.path().join("quire-args").exists(),
        "quire was not started"
    );
    assert!(work.path().join("o.pdf").exists());
}

/// A generator the test registers itself, so the files validated below are
/// exactly what `quoin_dossier::generate` writes for the real kinds. It lives
/// here and not in the `test-stubs` set because the real `srs` and `rmm`
/// generators must be free to register under `--all-features`.
struct KindOnly(quoin_dossier::DocumentSpec);

impl quoin_dossier::Generator for KindOnly {
    fn spec(&self) -> quoin_dossier::DocumentSpec {
        self.0
    }

    fn generate(
        &self,
        _: &quoin_dossier::Inputs,
    ) -> Result<quoin_dossier::Generated, quoin_dossier::SourceFailure> {
        Ok(quoin_dossier::Generated {
            body: format!("# {}", self.0.title),
            document_version: None,
            gaps: Vec::new(),
        })
    }
}

/// Needs the installed `quire` and the `spec-artifacts-process` module that
/// defines the `DossierDocument` archetype (that repository's FR-018, not a
/// quoin requirement), read in place from
/// `SPEC_ARTIFACTS_PROCESS_DIR` (its `spec_artifacts_process` directory).
/// The `srs` and `rmm` files are written by `quoin_dossier::generate` and
/// validated unchanged; each bad file is a copy of a generated one with a
/// single change. Nothing of the module is copied here.
///
/// Provenance: PLAT-1614 acceptance 9.
#[trace("FR-191")]
#[test]
#[ignore = "needs quire and the spec-artifacts-process DossierDocument archetype"]
fn generated_frontmatter_validates_and_an_unknown_document_kind_does_not() {
    let module = std::env::var_os("SPEC_ARTIFACTS_PROCESS_DIR")
        .map(PathBuf::from)
        .expect("set SPEC_ARTIFACTS_PROCESS_DIR to the spec_artifacts_process module directory");
    let module = fs::canonicalize(&module)
        .unwrap_or_else(|error| panic!("the module is not at {}: {error}", module.display()));
    let work = tempfile::tempdir().unwrap();
    let mut registry = quoin_dossier::Registry::new();
    for (kind, id, needs_as_of) in [("srs", "DOS-SRS", false), ("rmm", "DOS-RMM", true)] {
        registry
            .register(Box::new(KindOnly(quoin_dossier::DocumentSpec {
                kind,
                id,
                title: "Dossier document",
                needs_as_of,
            })))
            .unwrap();
    }
    let out = work.path().join("dossier");
    let inputs = quoin_dossier::Inputs {
        scope: PathBuf::from("."),
        modules: Vec::new(),
        clause_bindings: Vec::new(),
        pack: None,
        soup_binding: None,
        soup_records: None,
        soup_support: None,
        snapshot: None,
        release_verify: None,
        as_of: Some("2026-11-15T00:00:00Z".to_owned()),
        version: "1.0".to_owned(),
        producer_version: "test-quoin-version".to_owned(),
    };
    quoin_dossier::generate(&registry, &[], &out, &inputs).unwrap();

    let validate = |path: &Path| {
        let output = Command::new("quire")
            .args(["validate", "--module"])
            .arg(&module)
            .arg(path)
            .current_dir(work.path())
            .output()
            .expect("quire starts");
        let diagnostics = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        (output.status.success(), diagnostics)
    };
    let srs = out.join("DOS-SRS.md");
    let rmm = out.join("DOS-RMM.md");
    for generated in [&srs, &rmm] {
        let (valid, diagnostics) = validate(generated);
        assert!(valid, "{}: {diagnostics}", generated.display());
    }

    let bad_kind = work.path().join("bad-kind.md");
    fs::write(
        &bad_kind,
        read(&srs).replace(
            "document_kind: \"srs\"",
            "document_kind: \"unknown-dossier-kind\"",
        ),
    )
    .unwrap();
    let (valid, diagnostics) = validate(&bad_kind);
    assert!(!valid, "{diagnostics}");
    assert!(diagnostics.contains("document_kind"), "{diagnostics}");

    let no_as_of = work.path().join("no-as-of.md");
    let without = read(&rmm)
        .lines()
        .filter(|line| !line.starts_with("as_of:"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    fs::write(&no_as_of, without).unwrap();
    let (valid, diagnostics) = validate(&no_as_of);
    assert!(!valid, "{diagnostics}");
    assert!(diagnostics.contains("as_of"), "{diagnostics}");
}

/// Each new M3 selector generates only its registered document.
#[trace("FR-191-AC-23")]
#[test]
fn each_m3_selector_writes_only_its_matching_document() {
    let root = tempfile::tempdir().unwrap();
    let producer_version = version();
    for (kind, id) in [
        ("software-description", "DOS-SW.md"),
        ("documentation-level-decision", "DOS-DL.md"),
        ("software-design-specification", "DOS-SDS.md"),
        ("device-description", "DOS-DEV.md"),
    ] {
        let work = root.path().join(kind);
        fs::create_dir_all(&work).unwrap();
        let output = quoin(&work, &["generate", "--version", "1.0", "--doc", kind]);
        assert_eq!(code(&output), 1, "{}", stderr(&output));
        assert_eq!(names(&work.join("dossier")), [id]);
        let document = read(&work.join("dossier").join(id));
        assert!(document.contains(&format!(
            "producer_identity:\n  name: quoin\n  version: \"{producer_version}\""
        )));
    }
}

/// M3 documents preserve the identities of every source they read and repeat byte-for-byte.
#[trace(
    "FR-191-AC-32",
    "FR-229-AC-5",
    "FR-230-AC-5",
    "FR-231-AC-4",
    "FR-232-AC-4"
)]
#[test]
fn m3_sources_and_producer_are_stable_across_generations() {
    let producer_version = version();
    let source_records = [
        (
            "software_description",
            "sw",
            r#"{"schema_version":1,"id":"sw","record_type":"software_description","software_id":"SW","name":"Software","purpose":"purpose","operating_environment":"environment","functions":["function"],"interfaces":["interface"]}"#,
        ),
        (
            "documentation_level_decision",
            "dl",
            r#"{"schema_version":1,"id":"dl","record_type":"documentation_level_decision","software_id":"SW","level":"Level 2","decided_by":"Person","decided_on":"2026-10-01","rationale":"rationale","source_records":[{"id":"sw","record_type":"software_description"}]}"#,
        ),
        (
            "device_description",
            "dev",
            r#"{"schema_version":1,"id":"dev","record_type":"device_description","device_id":"D","name":"Device","purpose":"purpose","intended_use":"use","operating_context":"context","software_ids":["SW"]}"#,
        ),
    ];
    let kinds = [
        (
            "software-description",
            "DOS-SW.md",
            &["software_description/sw"][..],
        ),
        (
            "documentation-level-decision",
            "DOS-DL.md",
            &["documentation_level_decision/dl", "software_description/sw"][..],
        ),
        (
            "device-description",
            "DOS-DEV.md",
            &["device_description/dev", "software_description/sw"][..],
        ),
        (
            "software-design-specification",
            "DOS-SDS.md",
            &["software_description/sw"][..],
        ),
    ];
    for (kind, filename, source_ids) in kinds {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        for work in [first.path(), second.path()] {
            fs::create_dir_all(work.join("dossier/sources")).unwrap();
            for (source_kind, id, value) in source_records {
                let source_dir = work.join("dossier/sources").join(source_kind);
                fs::create_dir_all(&source_dir).unwrap();
                fs::write(source_dir.join(format!("{id}.json")), value).unwrap();
            }
            let output = quoin(work, &["generate", "--version", "1.0", "--doc", kind]);
            let expected = i32::from(kind == "software-design-specification");
            assert_eq!(code(&output), expected, "{}", stderr(&output));
        }
        let generated = read(&first.path().join("dossier").join(filename));
        assert_eq!(
            generated,
            read(&second.path().join("dossier").join(filename))
        );
        assert!(generated.contains(&format!(
            "producer_identity:\n  name: quoin\n  version: \"{producer_version}\""
        )));
        for identity in source_ids {
            assert!(
                generated.contains(identity),
                "missing {identity} in {filename}"
            );
        }
        assert!(!generated.contains("source commit"));
        assert!(!generated.contains("source digest"));
        assert!(!generated.contains("source revision"));
    }
}

/// A malformed selected M3 source fails before replacing any output file.
#[trace("FR-229-AC-4", "FR-191-AC-12")]
#[test]
fn malformed_m3_source_preserves_existing_outputs() {
    let work = tempfile::tempdir().unwrap();
    let output_dir = work.path().join("dossier");
    fs::create_dir_all(output_dir.join("sources/software_description")).unwrap();
    fs::write(output_dir.join("DOS-SW.md"), "existing bytes\n").unwrap();
    let source = output_dir.join("sources/software_description/sw.json");
    fs::write(&source, "{").unwrap();
    let output = quoin(
        work.path(),
        &[
            "generate",
            "--version",
            "1.0",
            "--doc",
            "software-description",
            "--json",
        ],
    );
    assert_eq!(code(&output), 3, "{}", stdout(&output));
    assert_eq!(
        fs::read(output_dir.join("DOS-SW.md")).unwrap(),
        b"existing bytes\n"
    );
    let report = json(&output);
    assert_eq!(
        report["source_failure"]["source"],
        "./dossier/sources/software_description/sw.json"
    );
}
