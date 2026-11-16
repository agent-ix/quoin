// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! Fixtures for the FR-198 unresolved-anomalies document.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use ix_trace_rs::trace;
use quoin_anomaly::GateMode;
use quoin_dossier::{GapKind, Inputs, SourceFailure};
use tempfile::TempDir;

struct Repo {
    dir: TempDir,
    commit: u32,
}

impl Repo {
    fn new() -> Self {
        let repo = Self {
            dir: tempfile::tempdir().unwrap(),
            commit: 0,
        };
        repo.git(&["init", "-q", "-b", "main"]);
        repo.git(&["config", "user.name", "Fixture Author"]);
        repo.git(&["config", "user.email", "fixture@example.invalid"]);
        repo.git(&["config", "commit.gpgsign", "false"]);
        repo
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn file(&self, name: &str) -> PathBuf {
        self.path().join(name)
    }

    fn write(&self, name: &str, content: &str) {
        let path = self.file(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    fn git(&self, args: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(self.path())
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn commit(&mut self) {
        self.git(&["add", "-A"]);
        self.commit += 1;
        let date = format!("2026-01-01T00:{:02}:00+00:00", self.commit);
        let output = Command::new("git")
            .arg("-C")
            .arg(self.path())
            .args(["commit", "-q", "-m", "fixture"])
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_DATE", &date)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git commit: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn anomaly(
        &mut self,
        id: &str,
        status: &str,
        resolution: Option<&str>,
        safety_impact: &str,
        targets: &[(&str, &str, &str, &str)],
        rationales: &[(&str, &str)],
    ) {
        let mut frontmatter = format!(
            "---\nid: {id}\ntitle: \"Fixture {id}\"\ntype: Anomaly\nstatus: {status}\nsafety_impact: {safety_impact}\nseverity: major\n"
        );
        if let Some(resolution) = resolution {
            frontmatter.push_str(&format!("resolution: {resolution}\n"));
        }
        frontmatter.push_str("---\n# Fixture anomaly\n\n## Safety Impact Assessment\n\n| Kind | Target | Impact | Rationale |\n| --- | --- | --- | --- |\n");
        for (kind, target, impact, detail) in targets {
            frontmatter.push_str(&format!("| {kind} | {target} | {impact} | {detail} |\n"));
        }
        if !rationales.is_empty() {
            frontmatter
                .push_str("\n## Release Rationale\n\n| Author | Rationale |\n| --- | --- |\n");
            for (author, reason) in rationales {
                frontmatter.push_str(&format!("| {author} | {reason} |\n"));
            }
        }
        self.write(&format!("spec/anomalies/{id}.md"), &frontmatter);
        self.commit();
    }
}

fn inputs(repo: &Repo) -> Inputs {
    Inputs {
        scope: repo.path().to_path_buf(),
        modules: Vec::new(),
        clause_bindings: Vec::new(),
        pack: None,
        soup_binding: None,
        soup_records: None,
        soup_support: None,
        as_of: None,
        version: "1".to_owned(),
    }
}

fn registry() -> quoin_dossier::Registry {
    quoin_dossier::builtin_registry(quoin_dossier::Sources {
        matrix: Arc::new(NoMatrix),
        rmm: |_| {
            Err(SourceFailure {
                source: "risk management".to_owned(),
                detail: "unexpected source call".to_owned(),
            })
        },
        soup: |_| {
            Err(SourceFailure {
                source: "SOUP".to_owned(),
                detail: "unexpected source call".to_owned(),
            })
        },
    })
    .unwrap()
}

struct NoMatrix;

impl quoin_dossier::MatrixSource for NoMatrix {
    fn matrix(&self, _: &Inputs) -> Result<quoin_assurance::matrix::MatrixOutput, SourceFailure> {
        Err(SourceFailure {
            source: "matrix".to_owned(),
            detail: "unexpected source call".to_owned(),
        })
    }
}

fn generate(repo: &Repo) -> (String, quoin_dossier::Report) {
    let inputs = inputs(repo);
    let out = repo.path().join("dossier");
    let report =
        quoin_dossier::generate(&registry(), &["anomalies".to_owned()], &out, &inputs).unwrap();
    let document = std::fs::read_to_string(out.join("DOS-ANOM.md")).unwrap();
    (document, report)
}

#[trace("FR-198-AC-1", "FR-198-AC-5", "FR-198-AC-7", "FR-198-AC-8")]
#[test]
fn selected_rows_groups_assessments_and_ungrouped_values_match_the_source() {
    let mut repo = Repo::new();
    repo.anomaly(
        "AN-001",
        "open",
        None,
        "impacting",
        &[
            (
                "Requirement",
                "FR-001",
                "Affects",
                "The authored requirement rationale.",
            ),
            (
                "Hazard",
                "HAZ-001",
                "Introduces",
                "The authored hazard rationale.",
            ),
        ],
        &[
            (
                "Dara Imani",
                "The operator procedure addresses this release risk.",
            ),
            (
                "Lee Tan",
                "The monitoring procedure covers this remaining case.",
            ),
        ],
    );
    repo.anomaly(
        "AN-002",
        "resolved",
        Some("fixed"),
        "impacting",
        &[("Requirement", "FR-002-AC-1", "Affects", "Criterion impact.")],
        &[],
    );
    repo.anomaly(
        "AN-003",
        "closed",
        Some("wont-fix"),
        "impacting",
        &[("Requirement", "FR-003", "Affects", "Retained impact.")],
        &[(
            "Mina Patel",
            "The team accepts this documented residual issue.",
        )],
    );
    repo.anomaly(
        "AN-004",
        "closed",
        Some("fixed"),
        "impacting",
        &[("Requirement", "FR-004", "Affects", "Fixed impact.")],
        &[("Omar Chen", "The issue was fixed before release.")],
    );
    repo.anomaly(
        "AN-005",
        "open",
        None,
        "assessed",
        &[("None", "-", "None", "No affected target is recorded.")],
        &[],
    );

    let inputs = inputs(&repo);
    let source =
        quoin_anomaly::build_report(&inputs.scope, &[GateMode::UnjustifiedImpacting]).unwrap();
    let (document, report) = generate(&repo);
    let listed = document
        .lines()
        .filter_map(|line| line.strip_prefix("### "))
        .filter_map(|heading| heading.split_once(':').map(|(id, _)| id.to_owned()))
        .collect::<Vec<_>>();
    let expected = source
        .anomalies
        .iter()
        .map(|row| row.id.clone())
        .collect::<Vec<_>>();
    assert_eq!(listed, expected);
    assert_eq!(listed, ["AN-001", "AN-002", "AN-003", "AN-005"]);
    assert!(!document.contains("AN-004"));
    assert!(document.contains("| HAZ-001 | AN-001 | Introduces |"));
    assert!(document.contains("| FR-001 | AN-001 | Affects |"));
    assert!(document.contains("| FR-002-AC-1 | AN-002 | Affects |"));
    assert!(
        document
            .contains("| Requirement | FR-001 | Affects | The authored requirement rationale. |")
    );
    assert!(
        document.contains("| Hazard | HAZ-001 | Introduces | The authored hazard rationale. |")
    );
    assert!(document.contains("Dara Imani"));
    assert!(document.contains("The operator procedure addresses this release risk."));
    assert!(document.contains("Lee Tan"));
    assert!(document.contains("The monitoring procedure covers this remaining case."));
    assert!(
        document.find("Dara Imani").unwrap() < document.find("Lee Tan").unwrap(),
        "authored rationale order changed"
    );
    assert!(document.contains("| AN-005 | no-affected-id |"));
    assert_eq!(document.matches("### AN-001:").count(), 1);
    assert_eq!(document, generate(&repo).0);
    assert_eq!(report.documents[0].kind, "anomalies");
}

#[trace("FR-198-AC-2", "FR-198-AC-3")]
#[test]
fn unassessed_and_source_gate_failures_become_gaps_without_suggested_rationale() {
    let mut repo = Repo::new();
    repo.anomaly(
        "AN-010",
        "open",
        None,
        "impacting",
        &[("Requirement", "FR-010", "Affects", "Impact is recorded.")],
        &[],
    );
    repo.anomaly(
        "AN-011",
        "closed",
        Some("wont-fix"),
        "unassessed",
        &[("None", "-", "None", "No assessment is recorded.")],
        &[],
    );
    repo.anomaly(
        "AN-012",
        "open",
        None,
        "impacting",
        &[("Requirement", "FR-012", "Affects", "Impact is recorded.")],
        &[("Tess Gray", "TBD")],
    );

    let (document, report) = generate(&repo);
    let gaps = &report.documents[0].gaps;
    assert!(
        gaps.iter()
            .any(|gap| { gap.gap_kind == GapKind::AnomalyUnassessed && gap.subject == "AN-011" })
    );
    assert!(gaps.iter().any(|gap| {
        gap.gap_kind == GapKind::AnomalyNoReleaseRationale && gap.subject == "AN-010"
    }));
    assert!(gaps.iter().any(|gap| {
        gap.gap_kind == GapKind::AnomalyNoReleaseRationale && gap.subject == "AN-012"
    }));
    assert!(document.contains("### AN-010: Fixture AN-010"));
    assert!(document.contains("Release rationale:\n\nnone"));
    assert!(document.contains("| Tess Gray | TBD |"));
    assert!(!document.contains("suggested rationale"));
    assert!(document.contains("| AN-011 | unassessed |"));
}

#[trace("FR-198-AC-6")]
#[test]
fn an_unreadable_anomaly_is_a_source_failure_before_any_document_is_written() {
    let repo = Repo::new();
    repo.write(
        "spec/anomalies/AN-020.md",
        "---\nid: AN-020\ntype: Anomaly\nstatus: open\n---\n# Missing assessment table\n",
    );
    let out = repo.path().join("dossier");
    let error =
        quoin_dossier::generate(&registry(), &["anomalies".to_owned()], &out, &inputs(&repo))
            .unwrap_err();
    assert_eq!(error.outcome().code(), 3);
    assert_eq!(
        error.source_failure().unwrap().source,
        "quoin anomaly report"
    );
    assert!(
        error
            .source_failure()
            .unwrap()
            .detail
            .contains("spec/anomalies/AN-020.md")
    );
    assert!(!out.exists());
}
