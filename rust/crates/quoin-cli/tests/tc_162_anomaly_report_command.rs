// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! The `quoin anomaly report` command surface (FR-162, FR-163, NFR-035).
//!
//! Anomaly documents are the fixtures `spec-artifacts-process` ships
//! (FR-014 to FR-016), read in place from `SPEC_ARTIFACTS_PROCESS_DIR` or the
//! checkout next to this one; nothing is copied into this repository. The
//! commit sequences are authored here, in a throwaway repository with fixed
//! author and dates, so the output is reproducible; commit hashes are replaced
//! by their order of appearance before the snapshot comparison.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;

fn fixtures() -> PathBuf {
    let root = std::env::var_os("SPEC_ARTIFACTS_PROCESS_DIR").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../spec-artifacts-process"),
        PathBuf::from,
    );
    let root = std::fs::canonicalize(&root).unwrap_or_else(|error| {
        panic!(
            "the spec-artifacts-process checkout is not at {}: {error}. These tests read \
             the documents that repository ships in place and keep no copy: check out \
             agent-ix/spec-artifacts-process next to this repository, or set \
             SPEC_ARTIFACTS_PROCESS_DIR to its checkout",
            root.display()
        )
    });
    let dir = root.join("tests/fixtures/anomaly");
    assert!(
        dir.is_dir(),
        "the anomaly fixtures are not at {}: update the spec-artifacts-process checkout \
         to a commit that ships them (FR-014 to FR-016)",
        dir.display()
    );
    dir
}

fn fixture(group: &str, name: &str, id: &str) -> String {
    std::fs::read_to_string(fixtures().join(group).join(format!("{name}.md")))
        .unwrap()
        .replace("id: AN-001", &format!("id: {id}"))
}

struct Repo {
    dir: tempfile::TempDir,
    tick: u32,
}

impl Repo {
    fn new() -> Self {
        let repo = Self {
            dir: tempfile::tempdir().unwrap(),
            tick: 0,
        };
        for args in [
            &["init", "-q", "-b", "main"][..],
            &["config", "user.name", "Fixture Author"],
            &["config", "user.email", "fixture@example.invalid"],
            &["config", "commit.gpgsign", "false"],
        ] {
            repo.git(args);
        }
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

    fn write(&self, rel: &str, text: &str) {
        let path = self.dir.path().join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn commit(&mut self, rel: &str, text: &str) {
        self.write(rel, text);
        self.git(&["add", "-A"]);
        self.tick += 1;
        let date = format!("2026-01-01T00:{:02}:00+00:00", self.tick);
        let status = Command::new("git")
            .arg("-C")
            .arg(self.dir.path())
            .args(["commit", "-q", "-m", "step"])
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_DATE", &date)
            .status()
            .unwrap();
        assert!(status.success());
    }

    fn quoin(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quoin"))
            .current_dir(self.dir.path())
            .args(args)
            .output()
            .unwrap()
    }
}

/// The shipped corpus plus one anomaly for each case FR-162 names, committed.
fn repo() -> Repo {
    let mut repo = Repo::new();
    let corpus = fixtures().join("corpus/spec");
    let mut files = Vec::new();
    collect(&corpus, &mut files);
    files.sort();
    for file in &files {
        let rel = Path::new("spec").join(file.strip_prefix(&corpus).unwrap());
        repo.write(
            rel.to_str().unwrap(),
            &std::fs::read_to_string(file).unwrap(),
        );
    }
    repo.commit("README.md", "corpus\n");
    let cases = [
        ("AN-001", "valid", "identified-severity"),
        ("AN-002", "valid", "identified"),
        ("AN-003", "valid", "assessed"),
        ("AN-004", "valid", "rationale-resolved-with"),
        ("AN-005", "valid", "closed-fixed"),
        ("AN-006", "valid", "closed-wont-fix-rationale"),
        ("AN-007", "valid", "closed-wont-fix"),
        ("AN-008", "valid", "closed-duplicate"),
        ("AN-009", "valid", "two-hazards"),
        ("AN-010", "valid", "none"),
        ("AN-011", "invalid", "closed-unassessed"),
        ("AN-012", "invalid", "dangling-requirement"),
        ("AN-013", "valid", "rationale-assessed-without"),
    ];
    for (id, group, name) in cases {
        repo.commit(
            &format!("spec/anomalies/{id}.md"),
            &fixture(group, name, id),
        );
    }
    // A mapping error: no `safety_impact`, so no record.
    let broken = fixture("valid", "assessed", "AN-014")
        .lines()
        .filter(|line| !line.starts_with("safety_impact:"))
        .collect::<Vec<_>>()
        .join("\n");
    repo.commit("spec/anomalies/AN-014.md", &broken);
    // A working-tree-only edit of AN-003's status and rationale: pending.
    repo.write(
        "spec/anomalies/AN-003.md",
        &fixture("valid", "rationale-resolved-with", "AN-003"),
    );
    repo
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// Replace each 40-hex commit hash by `<commit-N>`, N by order of first appearance.
fn normalise(text: &str) -> String {
    let mut seen: Vec<String> = Vec::new();
    let mut out = String::new();
    let mut rest = text;
    while !rest.is_empty() {
        let hex = rest.chars().take_while(char::is_ascii_hexdigit).count();
        if hex == 40 {
            let hash = &rest[..40];
            let at = seen.iter().position(|h| h == hash).unwrap_or_else(|| {
                seen.push(hash.to_owned());
                seen.len() - 1
            });
            let _ = write!(out, "<commit-{}>", at + 1);
            rest = &rest[40..];
        } else {
            let step = rest.chars().next().unwrap().len_utf8().max(hex);
            out.push_str(&rest[..step]);
            rest = &rest[step..];
        }
    }
    out
}

fn snapshot(name: &str, actual: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(name);
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}; run with UPDATE_SNAPSHOTS=1", path.display()));
    assert_eq!(actual, expected, "snapshot {name}");
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

#[trace("FR-162-AC-13", "FR-162-AC-12")]
#[test]
fn the_markdown_and_json_match_their_snapshots_over_one_corpus() {
    let repo = repo();
    let markdown = repo.quoin(&["anomaly", "report"]);
    assert_eq!(markdown.status.code(), Some(1), "AN-014 is unreadable");
    snapshot("anomaly-report.md", &normalise(&stdout(&markdown)));
    let json = repo.quoin(&["anomaly", "report", "--json"]);
    assert_eq!(json.status.code(), Some(1));
    snapshot("anomaly-report.json", &normalise(&stdout(&json)));
}

#[trace("FR-162-AC-11", "NFR-035-AC-5")]
#[test]
fn two_runs_give_the_same_bytes_in_both_formats() {
    let repo = repo();
    for args in [&["anomaly", "report"][..], &["anomaly", "report", "--json"]] {
        assert_eq!(stdout(&repo.quoin(args)), stdout(&repo.quoin(args)));
    }
}

#[trace("FR-162-AC-9")]
#[test]
fn an_unreadable_document_exits_non_zero_and_an_empty_scope_exits_zero() {
    let repo = repo();
    let out = repo.quoin(&["anomaly", "report", "--json"]);
    assert_eq!(out.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(
        json["unreadable"],
        serde_json::json!([{ "path": "spec/anomalies/AN-014.md", "code": "missing" }])
    );
    let empty = tempfile::tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["anomaly", "report"])
        .arg(empty.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(stdout(&out).matches("\nnone\n").count(), 5);
}

#[trace("FR-163-AC-3", "NFR-035-AC-1")]
#[test]
fn no_anomaly_command_offers_to_write_a_rationale_and_a_run_writes_no_file() {
    let repo = repo();
    for args in [
        &["anomaly", "--help"][..],
        &["anomaly", "check", "--help"],
        &["anomaly", "refs", "--help"],
        &["anomaly", "report", "--help"],
    ] {
        let help = stdout(&repo.quoin(args)).to_lowercase();
        for word in ["write", "fill", "suggest", "create", "edit", "default"] {
            assert!(
                !help.contains(&format!("{word} a release rationale")),
                "{help}"
            );
            assert!(!help.contains(&format!("--{word}")), "{help}");
        }
    }
    let status = |repo: &Repo| {
        let out = Command::new("git")
            .arg("-C")
            .arg(repo.dir.path())
            .args(["status", "--porcelain", "-uall"])
            .output()
            .unwrap();
        String::from_utf8(out.stdout).unwrap()
    };
    let before = status(&repo);
    let _ = repo.quoin(&["anomaly", "report"]);
    assert_eq!(status(&repo), before);
}

#[trace("FR-136-AC-13")]
#[test]
fn check_prints_a_fixing_change_line_for_each_document() {
    let repo = repo();
    let out = repo.quoin(&["anomaly", "check", "spec", "--module", "."]);
    let text = stdout(&out);
    assert!(
        text.lines()
            .any(|l| l.starts_with("spec/anomalies/AN-004.md: fixingChange ")
                && l.contains("Fixture Author")
                && l.contains("unsigned (N)")),
        "{text}"
    );
    assert!(
        text.contains("spec/anomalies/AN-001.md: fixingChange absent"),
        "{text}"
    );
}
