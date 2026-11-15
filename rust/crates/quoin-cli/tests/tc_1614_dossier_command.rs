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
//! temporary directory; the tests marked `#[ignore]` use the installed `quire`
//! and run with an explicit `-- --ignored`.
//!
//! Provenance: PLAT-1614, PLAT-1436.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;
use serde_json::Value;

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

fn generate_stubs(work: &Path, docs: &[&str]) {
    let mut args = vec!["generate", "--version", "1.0"];
    for doc in docs {
        args.extend(["--doc", doc]);
    }
    let output = quoin(work, &args);
    assert!(matches!(code(&output), 0 | 1), "{}", stderr(&output));
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

#[trace("FR-191-AC-13")]
#[test]
fn generate_check_and_export_without_a_version_exit_2_and_write_nothing() {
    let work = tempfile::tempdir().unwrap();
    for args in [
        &["generate", "--doc", "stub-a"][..],
        &["check", "--doc", "stub-a"][..],
        &[
            "export", "--doc", "stub-a", "--to", "pdf", "--output", "out.pdf",
        ][..],
    ] {
        let output = quoin(work.path(), args);
        assert_eq!(code(&output), 2, "{args:?}: {}", stderr(&output));
        assert_eq!(names(work.path()), Vec::<String>::new(), "{args:?}");
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

#[trace("FR-191-AC-8", "FR-191-AC-9")]
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
            "HEAD",
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
    assert!(args.contains(&"--source-rev".to_owned()) && args.contains(&"HEAD".to_owned()));
    assert!(
        !args.iter().any(|arg| arg.starts_with("--bundle")),
        "{args:?}"
    );
}

#[trace("FR-191-AC-10")]
#[test]
fn export_looks_where_out_says_and_names_the_document_missing_under_the_default() {
    let work = tempfile::tempdir().unwrap();
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

/// Needs the installed `quire` and a `git`. A generated file that is not
/// committed is refused by quire's own FR-034 section A check, which quoin
/// passes through.
///
/// Provenance: PLAT-1614 acceptance 7.
#[trace("FR-191-AC-7")]
#[test]
#[ignore = "needs the installed quire; run with -- --ignored"]
fn real_quire_refuses_an_uncommitted_generated_file() {
    let work = tempfile::tempdir().unwrap();
    for args in [
        &["init", "-q"][..],
        &["config", "user.email", "t@example.com"][..],
        &["config", "user.name", "t"][..],
        &["commit", "-q", "--allow-empty", "-m", "base"][..],
    ] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(work.path())
                .status()
                .unwrap()
                .success()
        );
    }
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
            "--json",
        ],
    );
    assert_eq!(code(&output), 4, "{}", stderr(&output));
    assert!(json(&output)["quire"]["exit_code"].is_number());
    assert!(!work.path().join("out.pdf").exists());
}

/// Needs the installed `quire` and the `spec-artifacts-process` module that
/// defines the `DossierDocument` archetype (FR-018), read in place from
/// `SPEC_ARTIFACTS_PROCESS_DIR` (its `spec_artifacts_process` directory). The
/// stub kinds are not in the archetype's `document_kind` enum, so the test
/// rewrites that one value in the generated files to a real kind and leaves the
/// rest of the frontmatter as quoin wrote it. Nothing of the module is copied
/// here.
///
/// Provenance: PLAT-1614 acceptance 9.
#[trace("FR-018-AC-1", "FR-018-AC-3")]
#[test]
#[ignore = "needs quire and the spec-artifacts-process DossierDocument archetype"]
fn generated_frontmatter_validates_and_an_unknown_document_kind_does_not() {
    let module = std::env::var_os("SPEC_ARTIFACTS_PROCESS_DIR")
        .map(PathBuf::from)
        .expect("set SPEC_ARTIFACTS_PROCESS_DIR to the spec_artifacts_process module directory");
    let module = fs::canonicalize(&module)
        .unwrap_or_else(|error| panic!("the module is not at {}: {error}", module.display()));
    let work = tempfile::tempdir().unwrap();
    let generated = quoin(
        work.path(),
        &[
            "generate",
            "--version",
            "1.0",
            "--as-of",
            "2026-11-15T00:00:00Z",
            "--doc",
            "stub-a",
            "--doc",
            "stub-time",
        ],
    );
    assert_eq!(code(&generated), 0, "{}", stderr(&generated));
    let validate = |name: &str, text: &str| {
        let path = work.path().join(name);
        fs::write(&path, text).unwrap();
        Command::new("quire")
            .args(["validate", "--module"])
            .arg(&module)
            .arg(&path)
            .current_dir(work.path())
            .output()
            .expect("quire starts")
    };
    let plain = read(&work.path().join("dossier/DOS-STUB-A.md"));
    let timed = read(&work.path().join("dossier/DOS-STUB-TIME.md"));

    let srs = validate(
        "srs.md",
        &plain.replace("document_kind: \"stub-a\"", "document_kind: \"srs\""),
    );
    assert!(
        srs.status.success(),
        "{}",
        String::from_utf8_lossy(&srs.stderr)
    );
    let rmm = validate(
        "rmm.md",
        &timed.replace("document_kind: \"stub-time\"", "document_kind: \"rmm\""),
    );
    assert!(
        rmm.status.success(),
        "{}",
        String::from_utf8_lossy(&rmm.stderr)
    );

    let bad = validate(
        "bad.md",
        &plain.replace(
            "document_kind: \"stub-a\"",
            "document_kind: \"verification-report\"",
        ),
    );
    assert!(!bad.status.success());
    let diagnostics = format!(
        "{}{}",
        String::from_utf8_lossy(&bad.stdout),
        String::from_utf8_lossy(&bad.stderr)
    );
    assert!(diagnostics.contains("document_kind"), "{diagnostics}");
}
