// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! The anomaly lifecycle check over git history and the reverse references
//! (FR-136).
//!
//! The anomaly documents are the fixtures `spec-artifacts-process` ships
//! (FR-014, FR-015). They are read in place from that repository's source, from
//! `SPEC_ARTIFACTS_PROCESS_DIR` or the checkout next to this one, and are never
//! copied into this repository. Only the commit sequences are authored here, in
//! throwaway repositories.
//!
//! Prerequisite: a checkout of `agent-ix/spec-artifacts-process` that holds
//! `tests/fixtures/anomaly` (PLAT-1415, #140), next to this repository or named
//! by `SPEC_ARTIFACTS_PROCESS_DIR`. The tests that compare against the engine's
//! own reference findings also need `quire` on `PATH` and the
//! `spec-artifacts-iso` module installed under `~/.ix/filament/modules`.

use std::path::{Path, PathBuf};
use std::process::Command;

use ix_trace_rs::trace;
use quoin_anomaly::{
    Checked, Context, DiagnosticCode, History, Severity, Signature, Status, check_document,
    parse_record, read_history, references_to,
};
use tempfile::TempDir;

const INSTALLED: Context = Context {
    hazard_installed: true,
};

fn process_root() -> PathBuf {
    std::env::var_os("SPEC_ARTIFACTS_PROCESS_DIR").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../spec-artifacts-process"),
        PathBuf::from,
    )
}

fn fixtures() -> PathBuf {
    let dir = process_root().join("tests/fixtures/anomaly");
    assert!(
        dir.is_dir(),
        "the spec-artifacts-process anomaly fixtures are not at {}. These tests read \
         the documents that repository ships in place and keep no copy: check out \
         agent-ix/spec-artifacts-process (at or after PLAT-1415, #140) next to this \
         repository, or set SPEC_ARTIFACTS_PROCESS_DIR to its checkout",
        dir.display()
    );
    dir
}

fn fixture(group: &str, name: &str) -> String {
    std::fs::read_to_string(fixtures().join(group).join(format!("{name}.md"))).unwrap()
}

fn with_status(text: &str, status: &str) -> String {
    text.lines()
        .map(|line| {
            if line.starts_with("status: ") {
                format!("status: {status}")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

/// The document at the repository root, and below it as in a real `spec/` tree.
const LAYOUTS: [&str; 2] = ["AN-001.md", "spec/anomaly/AN-001.md"];

struct Repo {
    dir: TempDir,
    tick: u32,
}

impl Repo {
    fn new() -> Self {
        let repo = Self {
            dir: tempfile::tempdir().unwrap(),
            tick: 0,
        };
        repo.git(&["init", "-q", "-b", "main"]);
        repo.git(&["config", "user.name", "Fixture Author"]);
        repo.git(&["config", "user.email", "fixture@example.invalid"]);
        repo.git(&["config", "commit.gpgsign", "false"]);
        repo
    }

    fn git(&self, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(self.dir.path())
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    fn write(&self, name: &str, text: &str) {
        let path = self.path(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn commit(&mut self, name: &str, text: &str) -> String {
        self.write(name, text);
        self.git(&["add", "-A"]);
        self.tick += 1;
        let date = format!("2026-01-0{}T10:00:00+00:00", self.tick);
        let status = Command::new("git")
            .arg("-C")
            .arg(self.dir.path())
            .args(["commit", "-q", "-m", "step"])
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_DATE", &date)
            .status()
            .unwrap();
        assert!(status.success());
        let head = Command::new("git")
            .arg("-C")
            .arg(self.dir.path())
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        String::from_utf8(head.stdout).unwrap().trim().to_owned()
    }
}

fn check_working(repo: &Repo, rel: &str, text: &str, context: Context) -> Checked {
    repo.write(rel, text);
    check_document(&repo.path(rel), text, context).unwrap()
}

/// Commit `statuses` in order at `rel`, then check the document as it stands.
fn check(
    repo: &mut Repo,
    rel: &str,
    base: &str,
    statuses: &[&str],
    working: Option<&str>,
) -> Vec<DiagnosticCode> {
    for status in statuses {
        repo.commit(rel, &with_status(base, status));
    }
    let text = with_status(base, working.unwrap_or_else(|| statuses.last().unwrap()));
    check_working(repo, rel, &text, INSTALLED)
        .diagnostics
        .into_iter()
        .map(|d| d.code)
        .collect()
}

fn history(path: &Path) -> (Vec<quoin_anomaly::StatusEntry>, Vec<Option<String>>) {
    match read_history(path).unwrap() {
        History::Entries {
            entries,
            assessments,
        } => (entries, assessments),
        History::NotApplicable => panic!("expected a git history"),
    }
}

#[trace("FR-136-AC-1")]
#[test]
fn status_history_has_one_entry_per_status_change_and_follows_a_rename() {
    for rel in LAYOUTS {
        let renamed = rel.replace("AN-001", "AN-001-renamed");
        let mut repo = Repo::new();
        let base = fixture("valid", "closed-fixed");
        let first = repo.commit(rel, &with_status(&base, "identified"));
        repo.commit(rel, &with_status(&base, "assessed"));
        repo.git(&["mv", rel, &renamed]);
        repo.commit(&renamed, &with_status(&base, "assessed"));
        repo.commit(&renamed, &with_status(&base, "resolved"));
        let last = repo.commit(&renamed, &with_status(&base, "closed"));
        let (entries, _) = history(&repo.path(&renamed));
        let statuses = entries.iter().map(|e| e.status).collect::<Vec<_>>();
        assert_eq!(
            statuses,
            [
                Status::Identified,
                Status::Assessed,
                Status::Resolved,
                Status::Closed
            ],
            "{rel}"
        );
        assert_eq!(entries[0].commit.as_deref(), Some(first.as_str()));
        assert_eq!(entries[3].commit.as_deref(), Some(last.as_str()));
        for entry in &entries {
            assert_eq!(entry.author.as_deref(), Some("Fixture Author"));
            assert!(
                entry
                    .authored_at
                    .as_deref()
                    .unwrap()
                    .starts_with("2026-01-0")
            );
            assert_eq!(entry.signature_code, Some('N'));
            assert_eq!(entry.signature, Some(Signature::Unsigned));
            assert!(!entry.pending);
        }
    }
}

#[trace("FR-136-AC-1")]
#[test]
fn an_author_and_date_typed_into_the_document_are_not_read() {
    let mut repo = Repo::new();
    let base = fixture("valid", "assessed").replace(
        "type: Anomaly",
        "type: Anomaly\nauthor: Someone Else\ndate: 1999-01-01\nauthoredAt: 1999-01-01",
    );
    repo.commit("AN-001.md", &base);
    let (entries, _) = history(&repo.path("AN-001.md"));
    assert_eq!(entries[0].author.as_deref(), Some("Fixture Author"));
    let authored = entries[0].authored_at.as_deref().unwrap();
    assert!(authored.starts_with("2026-01-01"), "{authored}");
    assert!(!authored.contains("1999"));
}

#[trace("FR-136-AC-2")]
#[test]
fn a_record_schema_failure_still_yields_a_record_and_no_field_diagnostic() {
    let mut repo = Repo::new();
    let base = fixture("invalid", "closed-no-closed-by");
    assert!(parse_record(&base).is_ok());
    let found = check(&mut repo, "AN-001.md", &base, &["closed"], None);
    assert!(found.is_empty(), "{found:?}");
}

#[trace("FR-136-AC-2", "FR-136-AC-11")]
#[test]
fn a_mapping_error_is_no_record_at_the_status_line_and_the_history_is_still_checked() {
    for rel in LAYOUTS {
        let mut repo = Repo::new();
        let base = fixture("valid", "assessed").replace("| Kind | Target |", "| Kind | Aim |");
        let error = parse_record(&base).unwrap_err();
        assert_eq!(error.code.as_str(), "columns");
        repo.commit(rel, &with_status(&base, "closed"));
        let text = with_status(&base, "assessed");
        let found = check_working(&repo, rel, &text, INSTALLED).diagnostics;
        let codes = found.iter().map(|d| d.code).collect::<Vec<_>>();
        assert_eq!(
            codes,
            [DiagnosticCode::NoRecord, DiagnosticCode::IllegalTransition]
        );
        let status_line = text.lines().position(|l| l.starts_with("status:")).unwrap() + 1;
        assert_ne!(error.line, status_line, "the mapping error is elsewhere");
        for diagnostic in &found {
            assert_eq!(diagnostic.line, status_line, "{diagnostic:?}");
        }
    }
}

#[trace("FR-136-AC-2")]
#[test]
fn an_out_of_vocabulary_impact_is_not_a_mapping_error() {
    let base = fixture("invalid", "impact-outside-vocabulary");
    let record = parse_record(&base).unwrap();
    assert!(
        record
            .assessment
            .unwrap()
            .iter()
            .any(|r| r.impact == "Breaks")
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("AN-001.md");
    std::fs::write(&path, &base).unwrap();
    let missing = Context {
        hazard_installed: false,
    };
    let found = check_document(&path, &base, missing).unwrap().diagnostics;
    assert_eq!(
        found.iter().map(|d| d.code).collect::<Vec<_>>(),
        [DiagnosticCode::MissingModule],
        "the document is not dropped as no-record"
    );
}

#[trace("FR-136-AC-3")]
#[test]
fn illegal_and_legal_moves_are_judged_by_reachability() {
    let fixed = fixture("valid", "closed-fixed");
    let illegal = [
        ["assessed", "identified"],
        ["resolved", "identified"],
        ["closed", "identified"],
        ["closed", "assessed"],
        ["closed", "resolved"],
    ];
    for rel in LAYOUTS {
        for pair in illegal {
            let found = check(&mut Repo::new(), rel, &fixed, &pair, None);
            assert_eq!(found, [DiagnosticCode::IllegalTransition], "{rel} {pair:?}");
        }
        // (base fixture, history): the closing resolution varies as FR-136-AC-3 lists.
        let legal: [(&str, [&str; 2]); 6] = [
            ("closed-fixed", ["identified", "resolved"]),
            ("closed-fixed", ["identified", "closed"]),
            ("closed-fixed", ["assessed", "closed"]),
            ("closed-wont-fix", ["assessed", "closed"]),
            ("closed-duplicate", ["identified", "closed"]),
            ("closed-fixed", ["resolved", "assessed"]),
        ];
        for (name, pair) in legal {
            let base = fixture("valid", name);
            let found = check(&mut Repo::new(), rel, &base, &pair, None);
            assert!(
                !found.contains(&DiagnosticCode::IllegalTransition),
                "{rel} {name} {pair:?}: {found:?}"
            );
        }
    }
}

#[trace("FR-136-AC-4")]
#[test]
fn a_first_entry_of_any_status_is_legal_and_the_pending_entry_is_judged() {
    let base = fixture("valid", "closed-fixed");
    for rel in LAYOUTS {
        for first in ["resolved", "closed"] {
            let found = check(&mut Repo::new(), rel, &base, &[first], None);
            assert!(found.is_empty(), "{rel} {first}: {found:?}");
        }
        let found = check(&mut Repo::new(), rel, &base, &["closed"], Some("assessed"));
        assert_eq!(found, [DiagnosticCode::IllegalTransition], "{rel}");
        let found = check(
            &mut Repo::new(),
            rel,
            &base,
            &["assessed"],
            Some("resolved"),
        );
        assert!(found.is_empty(), "{rel}: {found:?}");
    }
}

#[trace("FR-136-AC-5")]
#[test]
fn a_reopen_with_an_unchanged_assessment_is_reported_and_an_edit_clears_it() {
    let base = fixture("valid", "resolved-fixed");
    for rel in LAYOUTS {
        let mut repo = Repo::new();
        repo.commit(rel, &with_status(&base, "resolved"));
        let reopen = repo.commit(rel, &with_status(&base, "assessed"));
        repo.commit(rel, &with_status(&base, "resolved"));
        let text = with_status(&base, "resolved");
        let found = check_working(&repo, rel, &text, INSTALLED).diagnostics;
        assert_eq!(found.len(), 1, "{rel}: {found:?}");
        assert_eq!(found[0].code, DiagnosticCode::ReassessmentMissing);
        assert_eq!(found[0].commits, [reopen]);

        let mut repo = Repo::new();
        repo.commit(rel, &with_status(&base, "resolved"));
        repo.commit(rel, &with_status(&base, "assessed"));
        let edited =
            with_status(&base, "resolved").replace("is not enforced", "is no longer enforced");
        repo.commit(rel, &edited);
        let found = check_working(&repo, rel, &edited, INSTALLED).diagnostics;
        assert!(found.is_empty(), "{rel}: {found:?}");
    }
}

#[trace("FR-136-AC-6")]
#[test]
fn a_file_outside_git_has_no_history_and_no_transition_diagnostic() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("AN-001.md");
    let text = fixture("valid", "closed-fixed");
    std::fs::write(&path, &text).unwrap();
    assert_eq!(read_history(&path).unwrap(), History::NotApplicable);
    let checked = check_document(&path, &text, INSTALLED).unwrap();
    assert!(checked.diagnostics.is_empty());
    assert!(!checked.history_applicable, "not applicable is reported");
}

fn scope_with(anomalies: &[(&str, &str, &str)]) -> TempDir {
    let scope = tempfile::tempdir().unwrap();
    let corpus = fixtures().join("corpus/spec");
    for entry in walk(&corpus) {
        let target = scope
            .path()
            .join("spec")
            .join(entry.strip_prefix(&corpus).unwrap());
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(&entry, target).unwrap();
    }
    for (id, group, name) in anomalies {
        let text = fixture(group, name).replace("id: AN-001", &format!("id: {id}"));
        let anomalies = scope.path().join("spec/anomalies");
        std::fs::create_dir_all(&anomalies).unwrap();
        std::fs::write(anomalies.join(format!("{id}.md")), text).unwrap();
    }
    scope
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk(&path));
        } else {
            out.push(path);
        }
    }
    out
}

#[trace("FR-136-AC-7")]
#[test]
fn reverse_references_list_each_anomaly_that_names_an_id() {
    let scope = scope_with(&[
        ("AN-002", "valid", "assessed"),
        ("AN-001", "valid", "two-hazards"),
        ("AN-003", "valid", "none"),
    ]);
    let refs = |id: &str| references_to(scope.path(), id).unwrap();
    let fr = refs("FR-001");
    assert_eq!(
        fr.iter()
            .map(|r| (r.anomaly.as_str(), r.impact.as_str()))
            .collect::<Vec<_>>(),
        [("AN-002", "Affects")]
    );
    let ac = refs("FR-001-AC-1");
    assert_eq!(ac.len(), 1);
    assert_eq!(ac[0].kind, "Requirement");
    let hazard = refs("HAZ-001");
    assert_eq!(
        hazard
            .iter()
            .map(|r| r.anomaly.as_str())
            .collect::<Vec<_>>(),
        ["AN-001", "AN-002"],
        "anomaly id order"
    );
    assert_eq!(hazard[0].impact, "Introduces");
    assert!(refs("HAZ-003").is_empty(), "a hazard no anomaly names");
    assert!(refs("-").is_empty(), "a None row names nothing");
}

#[trace("FR-136-AC-8")]
#[test]
fn a_dangling_target_is_in_no_listing_and_is_not_reported_by_check() {
    let scope = scope_with(&[("AN-001", "invalid", "dangling-requirement")]);
    assert!(references_to(scope.path(), "FR-999").unwrap().is_empty());
    assert!(
        references_to(scope.path(), "FR-001").unwrap().is_empty(),
        "no row of this anomaly names FR-001"
    );
    let hazard = references_to(scope.path(), "HAZ-001").unwrap();
    assert_eq!(hazard.len(), 1, "the resolvable Hazard row is listed");
    assert_eq!(hazard[0].kind, "Hazard");
    let text = fixture("invalid", "dangling-requirement");
    let path = scope.path().join("spec/anomalies/AN-001.md");
    assert!(
        check_document(&path, &text, INSTALLED)
            .unwrap()
            .diagnostics
            .is_empty()
    );
}

#[trace("FR-136-AC-10")]
#[test]
fn two_hazard_rows_with_the_module_missing_yield_exactly_one_diagnostic() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("AN-001.md");
    let text = fixture("valid", "two-hazards");
    std::fs::write(&path, &text).unwrap();
    let missing = Context {
        hazard_installed: false,
    };
    let found = check_document(&path, &text, missing).unwrap().diagnostics;
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, DiagnosticCode::MissingModule);
    assert!(
        check_document(&path, &text, INSTALLED)
            .unwrap()
            .diagnostics
            .is_empty()
    );
}

/// With the hazard documents absent from the bundle, `quire validate` still
/// reports its own finding for each of the two Hazard rows.
#[trace("FR-136-AC-10")]
#[test]
fn quire_validate_still_reports_its_own_per_row_findings() {
    let scope = scope_with(&[("AN-001", "valid", "two-hazards")]);
    std::fs::remove_dir_all(scope.path().join("spec/hazard")).unwrap();
    let process = process_root().join("spec_artifacts_process");
    let iso = dirs_modules().join("spec-artifacts-iso");
    // This clause compares against the engine's own findings, so the `quire` CLI
    // and the spec-artifacts-iso module are prerequisites. Like the rest of this
    // suite the test fails, rather than skips, when one is missing.
    assert!(
        iso.is_dir(),
        "the spec-artifacts-iso module is not installed at {}: install it \
         (`quoin module ensure-defaults`) before running this test",
        iso.display()
    );
    let output = Command::new("quire")
        .args(["validate", "--okf", "--scope"])
        .arg(scope.path())
        .arg("--module")
        .arg(&process)
        .arg("--module")
        .arg(&iso)
        .output()
        .unwrap_or_else(|error| {
            panic!(
                "this test runs the `quire` CLI and cannot: {error}. Install it \
                 (`npm install -g @agent-ix/quire-cli`) and put it on PATH"
            )
        });
    let stderr = String::from_utf8_lossy(&output.stderr);
    let findings = stderr
        .lines()
        .filter(|l| l.contains("[dangling-trace-reference]") && l.contains("AN-001.md"))
        .collect::<Vec<_>>();
    assert_eq!(findings.len(), 2, "{stderr}");
    assert!(findings.iter().any(|l| l.contains("'HAZ-001'")));
    assert!(findings.iter().any(|l| l.contains("'HAZ-002'")));
}

fn dirs_modules() -> PathBuf {
    let home = std::env::var_os("IX_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".ix")))
        .expect("HOME or IX_HOME locates the installed modules");
    home.join("filament/modules")
}

#[trace("FR-136-AC-11")]
#[test]
fn every_diagnostic_carries_its_code_severity_reason_and_the_status_line() {
    let base = fixture("valid", "two-hazards");
    let status_line = base.lines().position(|l| l.starts_with("status:")).unwrap() + 1;
    let missing = Context {
        hazard_installed: false,
    };
    // AC-3 and AC-10: an illegal transition and a missing module.
    let mut repo = Repo::new();
    repo.commit("AN-001.md", &with_status(&base, "closed"));
    let text = with_status(&base, "assessed");
    let found = check_working(&repo, "AN-001.md", &text, missing).diagnostics;
    assert_eq!(found.len(), 2, "{found:?}");
    let reasons = found
        .iter()
        .map(quoin_anomaly::Diagnostic::reason)
        .collect::<Vec<_>>();
    assert_eq!(reasons, ["transition", "spec-objects-safety"]);
    assert_eq!(found[0].code.as_str(), "anomaly.illegal-transition");
    // AC-5: a reopen that changed nothing.
    let resolved = fixture("valid", "resolved-fixed");
    let mut reopen = Repo::new();
    reopen.commit("AN-001.md", &with_status(&resolved, "resolved"));
    reopen.commit("AN-001.md", &with_status(&resolved, "assessed"));
    reopen.commit("AN-001.md", &with_status(&resolved, "resolved"));
    let reopened = check_working(
        &reopen,
        "AN-001.md",
        &with_status(&resolved, "resolved"),
        INSTALLED,
    )
    .diagnostics;
    assert_eq!(reopened.len(), 1);
    assert_eq!(reopened[0].reason(), "reopen");
    assert_eq!(reopened[0].code.as_str(), "anomaly.reassessment-missing");
    // AC-2: no record.
    let broken = fixture("valid", "assessed").replace("| Kind | Target |", "| Kind | Aim |");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("AN-001.md");
    std::fs::write(&path, &broken).unwrap();
    let no_record = check_document(&path, &broken, INSTALLED)
        .unwrap()
        .diagnostics;
    assert_eq!(no_record.len(), 1);
    assert_eq!(no_record[0].reason(), "mapping-error");
    assert_eq!(no_record[0].code.as_str(), "anomaly.no-record");
    let broken_status = broken
        .lines()
        .position(|l| l.starts_with("status:"))
        .unwrap()
        + 1;
    for diagnostic in found.iter().chain(&reopened).chain(&no_record) {
        assert_eq!(diagnostic.severity, Severity::Error);
    }
    assert!(found.iter().all(|d| d.line == status_line));
    assert!(reopened.iter().all(|d| d.line == status_line_of(&resolved)));
    assert!(no_record.iter().all(|d| d.line == broken_status));
}

fn status_line_of(text: &str) -> usize {
    text.lines().position(|l| l.starts_with("status:")).unwrap() + 1
}

#[trace("FR-136-AC-12")]
#[test]
fn signature_letters_map_to_signed_unsigned_or_unknown() {
    for code in ['G', 'U', 'X', 'Y'] {
        assert_eq!(Signature::from_code(code), Signature::Signed, "{code}");
    }
    assert_eq!(Signature::from_code('N'), Signature::Unsigned);
    for code in ['B', 'R', 'E'] {
        assert_eq!(Signature::from_code(code), Signature::Unknown, "{code}");
    }
}
