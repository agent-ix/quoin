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

use std::path::{Path, PathBuf};
use std::process::Command;

use ix_trace_rs::trace;
use quoin_anomaly::{
    Context, DiagnosticCode, History, Severity, Signature, check_document, read_history,
    references_to,
};
use tempfile::TempDir;

const INSTALLED: Context = Context {
    hazard_installed: true,
};

fn fixtures() -> PathBuf {
    let root = std::env::var_os("SPEC_ARTIFACTS_PROCESS_DIR").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../spec-artifacts-process"),
        PathBuf::from,
    );
    let dir = root.join("tests/fixtures/anomaly");
    assert!(
        dir.is_dir(),
        "the spec-artifacts-process fixtures are not at {}; set SPEC_ARTIFACTS_PROCESS_DIR",
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

    fn commit(&mut self, name: &str, text: &str) -> String {
        std::fs::write(self.path(name), text).unwrap();
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

/// Commit `statuses` in order, then check the document as it stands.
fn check(
    repo: &mut Repo,
    base: &str,
    statuses: &[&str],
    working: Option<&str>,
) -> Vec<DiagnosticCode> {
    for status in statuses {
        repo.commit("AN-001.md", &with_status(base, status));
    }
    let text = working.map_or_else(
        || with_status(base, statuses.last().unwrap()),
        |status| with_status(base, status),
    );
    std::fs::write(repo.path("AN-001.md"), &text).unwrap();
    check_document(&repo.path("AN-001.md"), &text, INSTALLED)
        .unwrap()
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
    let mut repo = Repo::new();
    let base = fixture("valid", "closed-fixed");
    let first = repo.commit("AN-001.md", &with_status(&base, "identified"));
    repo.commit("AN-001.md", &with_status(&base, "assessed"));
    repo.git(&["mv", "AN-001.md", "AN-001-renamed.md"]);
    repo.commit("AN-001-renamed.md", &with_status(&base, "assessed"));
    repo.commit("AN-001-renamed.md", &with_status(&base, "resolved"));
    let last = repo.commit("AN-001-renamed.md", &with_status(&base, "closed"));
    let (entries, _) = history(&repo.path("AN-001-renamed.md"));
    let statuses = entries
        .iter()
        .map(|e| e.status.as_str())
        .collect::<Vec<_>>();
    assert_eq!(statuses, ["identified", "assessed", "resolved", "closed"]);
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

#[trace("FR-136-AC-1")]
#[test]
fn an_author_typed_into_the_document_is_not_read() {
    let mut repo = Repo::new();
    let base = fixture("valid", "assessed").replace(
        "type: Anomaly",
        "type: Anomaly\nauthor: Someone Else\ndate: 1999-01-01",
    );
    repo.commit("AN-001.md", &base);
    let (entries, _) = history(&repo.path("AN-001.md"));
    assert_eq!(entries[0].author.as_deref(), Some("Fixture Author"));
}

#[trace("FR-136-AC-2")]
#[test]
fn a_record_schema_failure_still_yields_a_record_and_no_field_diagnostic() {
    let mut repo = Repo::new();
    let base = fixture("invalid", "closed-no-closed-by");
    assert!(quoin_anomaly::parse_record(&base).is_ok());
    let found = check(&mut repo, &base, &["closed"], None);
    assert!(found.is_empty(), "{found:?}");
}

#[trace("FR-136-AC-2")]
#[test]
fn a_mapping_error_is_no_record_and_the_history_is_still_checked() {
    let mut repo = Repo::new();
    let base = fixture("valid", "assessed").replace("| Kind | Target |", "| Kind | Aim |");
    assert_eq!(
        quoin_anomaly::parse_record(&base).unwrap_err().code,
        "columns"
    );
    let found = check(&mut repo, &base, &["closed", "assessed"], None);
    assert_eq!(
        found,
        [DiagnosticCode::NoRecord, DiagnosticCode::IllegalTransition]
    );
}

#[trace("FR-136-AC-3")]
#[test]
fn illegal_and_legal_moves_are_judged_by_reachability() {
    let base = fixture("valid", "closed-fixed");
    let illegal = [
        ["assessed", "identified"],
        ["resolved", "identified"],
        ["closed", "identified"],
        ["closed", "assessed"],
        ["closed", "resolved"],
    ];
    for pair in illegal {
        let found = check(&mut Repo::new(), &base, &pair, None);
        assert_eq!(found, [DiagnosticCode::IllegalTransition], "{pair:?}");
    }
    let legal = [
        ["identified", "resolved"],
        ["identified", "closed"],
        ["assessed", "closed"],
        ["resolved", "assessed"],
    ];
    for pair in legal {
        let found = check(&mut Repo::new(), &base, &pair, None);
        assert!(
            !found.contains(&DiagnosticCode::IllegalTransition),
            "{pair:?}: {found:?}"
        );
    }
}

#[trace("FR-136-AC-4")]
#[test]
fn a_first_entry_of_any_status_is_legal_and_the_pending_entry_is_judged() {
    let base = fixture("valid", "closed-fixed");
    for first in ["resolved", "closed"] {
        let found = check(&mut Repo::new(), &base, &[first], None);
        assert!(found.is_empty(), "{first}: {found:?}");
    }
    let found = check(&mut Repo::new(), &base, &["closed"], Some("assessed"));
    assert_eq!(found, [DiagnosticCode::IllegalTransition]);
    let found = check(&mut Repo::new(), &base, &["assessed"], Some("resolved"));
    assert!(found.is_empty(), "{found:?}");
}

#[trace("FR-136-AC-5")]
#[test]
fn a_reopen_with_an_unchanged_assessment_is_reported_and_an_edit_clears_it() {
    let base = fixture("valid", "resolved-fixed");
    let mut repo = Repo::new();
    repo.commit("AN-001.md", &with_status(&base, "resolved"));
    let reopen = repo.commit("AN-001.md", &with_status(&base, "assessed"));
    repo.commit("AN-001.md", &with_status(&base, "resolved"));
    let text = with_status(&base, "resolved");
    let found = check_document(&repo.path("AN-001.md"), &text, INSTALLED).unwrap();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, DiagnosticCode::ReassessmentMissing);
    assert_eq!(found[0].commits, [reopen]);

    let mut repo = Repo::new();
    repo.commit("AN-001.md", &with_status(&base, "resolved"));
    repo.commit("AN-001.md", &with_status(&base, "assessed"));
    let final_text =
        with_status(&base, "resolved").replace("is not enforced", "is no longer enforced");
    repo.commit("AN-001.md", &final_text);
    let found = check_document(&repo.path("AN-001.md"), &final_text, INSTALLED).unwrap();
    assert!(found.is_empty(), "{found:?}");
}

#[trace("FR-136-AC-6")]
#[test]
fn a_file_outside_git_has_no_history_and_no_transition_diagnostic() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("AN-001.md");
    let text = fixture("valid", "closed-fixed");
    std::fs::write(&path, &text).unwrap();
    assert_eq!(read_history(&path).unwrap(), History::NotApplicable);
    assert!(check_document(&path, &text, INSTALLED).unwrap().is_empty());
}

fn scope_with(anomalies: &[(&str, &str, &str)]) -> TempDir {
    let scope = tempfile::tempdir().unwrap();
    let corpus = fixtures().join("corpus/spec");
    for entry in walk(&corpus) {
        let target = scope.path().join(entry.strip_prefix(&corpus).unwrap());
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(&entry, target).unwrap();
    }
    for (id, group, name) in anomalies {
        let text = fixture(group, name).replace("id: AN-001", &format!("id: {id}"));
        std::fs::write(scope.path().join(format!("{id}.md")), text).unwrap();
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
    for id in ["FR-999", "FR-001", "HAZ-001"] {
        let refs = references_to(scope.path(), id).unwrap();
        assert!(refs.iter().all(|r| r.impact != "Affects" || id != "FR-999"));
    }
    assert!(references_to(scope.path(), "FR-999").unwrap().is_empty());
    let text = fixture("invalid", "dangling-requirement");
    let path = scope.path().join("AN-001.md");
    assert!(check_document(&path, &text, INSTALLED).unwrap().is_empty());
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
    let found = check_document(&path, &text, missing).unwrap();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, DiagnosticCode::MissingModule);
    assert!(check_document(&path, &text, INSTALLED).unwrap().is_empty());
}

#[trace("FR-136-AC-11")]
#[test]
fn every_diagnostic_carries_its_code_severity_reason_and_the_status_line() {
    let base = fixture("valid", "two-hazards");
    let status_line = base.lines().position(|l| l.starts_with("status:")).unwrap() + 1;
    let mut repo = Repo::new();
    repo.commit("AN-001.md", &with_status(&base, "closed"));
    let text = with_status(&base, "assessed");
    std::fs::write(repo.path("AN-001.md"), &text).unwrap();
    let missing = Context {
        hazard_installed: false,
    };
    let found = check_document(&repo.path("AN-001.md"), &text, missing).unwrap();
    assert_eq!(found.len(), 2, "{found:?}");
    for diagnostic in &found {
        assert_eq!(diagnostic.severity, Severity::Error);
        assert_eq!(diagnostic.line, status_line);
    }
    let reasons = found.iter().map(|d| d.reason.as_str()).collect::<Vec<_>>();
    assert_eq!(reasons, ["transition", "spec-objects-safety"]);
    assert_eq!(found[0].code.as_str(), "anomaly.illegal-transition");
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
