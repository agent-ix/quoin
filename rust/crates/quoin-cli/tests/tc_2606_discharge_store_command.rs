// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin evidence record-discharge` and `quoin discharge --repo --inward`
//! against a real repository (FR-133).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use ix_trace_rs::trace;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod pack_support;

const BINDING: &str = r#"{
  "schemaVersion": "clause-binding-v1",
  "clauseSet": {"authority": "quire", "id": "spec", "version": "1"},
  "context": {},
  "clauses": [
    {"clauseId": "C-1", "force": "mandatory", "outcome": "binding", "reasons": [], "expectedOutputs": []},
    {"clauseId": "C-2", "force": "mandatory", "outcome": "binding", "reasons": [], "expectedOutputs": []}
  ]
}"#;

const FACTS: &str = r#"[{
  "kind": "direct", "clauseId": "C-1", "evidenceRefs": ["evidence://runs/SUITE-1/one"],
  "attestation": {"attestedBy": "r", "authority": "lead",
    "attestedAt": "2026-08-01T00:00:00Z", "expiresAt": "2026-09-01T00:00:00Z"}
}]"#;

const TEMPORARY_EXCEPTION_FACTS: &str = r#"[{
  "kind": "disposition", "clauseId": "C-1", "decision": "temporary_exception",
  "owner": "risk-owner-1", "rationale": "bounded synthetic exception",
  "approvalRef": "approval://test/one", "decisionExpiresAt": "2026-08-20T00:00:00Z",
  "attestation": {"attestedBy": "reviewer-1", "authority": "quality-lead",
    "attestedAt": "2026-08-01T00:00:00Z", "expiresAt": "2026-09-01T00:00:00Z"}
}]"#;

/// Retain the run record the facts cite.
fn retain_run(repo: &Path) {
    let run = repo.join("spec/evidence/runs/SUITE-1");
    std::fs::create_dir_all(&run).unwrap();
    std::fs::write(run.join("one.json"), "{}").unwrap();
}

/// An IX home with the clause set of [`BINDING`] installed.
fn installed_home() -> tempfile::TempDir {
    pack_support::home_with_clause_set("quire", "spec", "1", &["C-1", "C-2"])
}

fn quoin(home: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(arguments)
        .env("IX_HOME", home)
        .output()
        .unwrap()
}

fn text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn files_under(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(directory).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push(path);
            }
        }
    }
    files
}

#[trace("FR-133-AC-7")]
#[test]
fn tc_2606_recorded_facts_are_read_back_by_the_discharge_command() {
    let dir = tempfile::tempdir().unwrap();
    retain_run(dir.path());
    let home = installed_home();
    let binding = dir.path().join("binding.json");
    let facts = dir.path().join("facts.json");
    std::fs::write(&binding, BINDING).unwrap();
    std::fs::write(&facts, FACTS).unwrap();
    let (binding, facts, repo) = (
        binding.to_str().unwrap(),
        facts.to_str().unwrap(),
        dir.path().to_str().unwrap(),
    );

    let recorded = quoin(
        home.path(),
        &[
            "evidence",
            "record-discharge",
            "--repo",
            repo,
            "--binding",
            binding,
            "--facts",
            facts,
        ],
    );
    assert!(recorded.status.success(), "{recorded:?}");
    assert!(text(&recorded).starts_with("C-1 "), "{}", text(&recorded));

    let inward = quoin(
        home.path(),
        &[
            "discharge",
            "--binding",
            binding,
            "--repo",
            repo,
            "--as-of",
            "2026-08-15T00:00:00Z",
            "--inward",
        ],
    );
    assert!(inward.status.success(), "{inward:?}");
    let report = text(&inward);
    assert!(report.contains("- Binding clauses: 2"), "{report}");
    assert!(report.contains("- Discharged by evidence: 1"), "{report}");
    assert!(report.contains("- `C-2` — mandatory"), "{report}");

    let partition = quoin(
        home.path(),
        &[
            "discharge",
            "--binding",
            binding,
            "--repo",
            repo,
            "--as-of",
            "2026-08-15T00:00:00Z",
            "--json",
        ],
    );
    assert!(partition.status.success(), "{partition:?}");
    let json: serde_json::Value = serde_json::from_str(&text(&partition)).unwrap();
    assert_eq!(json["binding"]["direct"][0]["clauseId"], "C-1");
}

#[trace("FR-133-AC-5")]
#[test]
fn tc_2604_a_doubled_binding_clause_is_refused_not_counted_twice() {
    let dir = tempfile::tempdir().unwrap();
    retain_run(dir.path());
    let home = installed_home();
    let mut binding: serde_json::Value = serde_json::from_str(BINDING).unwrap();
    let first = binding["clauses"][0].clone();
    binding["clauses"].as_array_mut().unwrap().push(first);
    let path = dir.path().join("binding.json");
    std::fs::write(&path, binding.to_string()).unwrap();
    let output = quoin(
        home.path(),
        &[
            "discharge",
            "--binding",
            path.to_str().unwrap(),
            "--repo",
            dir.path().to_str().unwrap(),
            "--as-of",
            "2026-08-15T00:00:00Z",
            "--inward",
        ],
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("duplicate clause C-1"), "{stderr}");
}

#[trace("FR-133-AC-2", "FR-174-AC-6", "TC-004")]
#[test]
fn tc_2606_a_stored_temporary_exception_expires_at_its_decision_boundary() {
    let dir = tempfile::tempdir().unwrap();
    retain_run(dir.path());
    let home = installed_home();
    let binding = dir.path().join("binding.json");
    let facts = dir.path().join("facts.json");
    std::fs::write(&binding, BINDING).unwrap();
    std::fs::write(&facts, TEMPORARY_EXCEPTION_FACTS).unwrap();
    let (binding, facts, repo) = (
        binding.to_str().unwrap(),
        facts.to_str().unwrap(),
        dir.path().to_str().unwrap(),
    );

    let recorded = quoin(
        home.path(),
        &[
            "evidence",
            "record-discharge",
            "--repo",
            repo,
            "--binding",
            binding,
            "--facts",
            facts,
        ],
    );
    assert!(recorded.status.success(), "{recorded:?}");
    let stored = files_under(dir.path())
        .into_iter()
        .find(|path| path.ends_with("c-1.json"))
        .expect("the temporary exception is persisted for C-1");
    let stored_fact = std::fs::read_to_string(stored).unwrap();
    assert!(
        stored_fact.contains("2026-08-20T00:00:00Z"),
        "{stored_fact}"
    );

    let render = |as_of: &str| {
        quoin(
            home.path(),
            &[
                "discharge",
                "--binding",
                binding,
                "--repo",
                repo,
                "--as-of",
                as_of,
                "--rmm",
            ],
        )
    };
    let before = render("2026-08-19T23:59:59.999Z");
    assert!(before.status.success(), "{before:?}");
    let before = text(&before);
    assert!(
        before.contains("| C-1 | Applicable | Tailored |"),
        "{before}"
    );

    for as_of in ["2026-08-20T00:00:00Z", "2026-08-20T00:00:00.001Z"] {
        let expired = render(as_of);
        assert!(expired.status.success(), "{expired:?}");
        let report = text(&expired);
        assert!(
            report.contains("| C-1 | Applicable | Not compliant - temporary exception expired"),
            "{report}"
        );
    }
}

/// `evidence approval-subject` prints the record id and the RFC 8785 digest,
/// the digest ignores `approvalRef`, and a clause outside the binding report
/// exits non-zero with empty stdout.
#[trace("FR-211-AC-17", "FR-211-AC-18")]
#[test]
fn tc_211_approval_subject_prints_the_identity_a_person_signs() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let write = |name: &str, text: &str| {
        let path = dir.path().join(name);
        std::fs::write(&path, text).unwrap();
        path.to_str().unwrap().to_owned()
    };
    let binding = write("binding.json", BINDING);
    let fact: serde_json::Value = serde_json::from_str(FACTS).unwrap();
    let plain = write("plain.json", &fact[0].to_string());
    let mut tagged = fact[0].clone();
    tagged["approvalRef"] = "tag-1".into();
    let tagged = write("tagged.json", &tagged.to_string());
    let mut other = fact[0].clone();
    other["clauseId"] = "C-9".into();
    let other = write("other.json", &other.to_string());
    let subject = |fact: &str| {
        quoin(
            home.path(),
            &[
                "evidence",
                "approval-subject",
                "--binding",
                &binding,
                "--fact",
                fact,
            ],
        )
    };

    let plain_out = subject(&plain);
    assert!(plain_out.status.success(), "{plain_out:?}");
    let line = text(&plain_out);
    assert_eq!(line.lines().count(), 1);
    let parsed: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(parsed["record_id"], "quire/spec/1/C-1");
    assert!(
        parsed["record_digest"]
            .as_str()
            .unwrap()
            .starts_with("sha256:")
    );
    assert!(line.starts_with(r#"{"record_id":"#), "{line}");
    assert_eq!(text(&subject(&tagged)), line);

    let refused = subject(&other);
    assert!(!refused.status.success());
    assert!(refused.stdout.is_empty());
}

/// `--facts` beside `--repo` takes the facts from the file, ignores the stored
/// ones and resolves their refs against the store; without `--repo` the direct
/// fact is open with `no-store`.
#[trace("FR-211-AC-21", "FR-046-AC-6")]
#[test]
fn tc_211_facts_beside_repo_resolve_against_the_store_and_alone_are_no_store() {
    let dir = tempfile::tempdir().unwrap();
    retain_run(dir.path());
    let home = installed_home();
    let binding = dir.path().join("binding.json");
    let facts = dir.path().join("facts.json");
    std::fs::write(&binding, BINDING).unwrap();
    std::fs::write(&facts, FACTS).unwrap();
    let (binding, facts, repo) = (
        binding.to_str().unwrap(),
        facts.to_str().unwrap(),
        dir.path().to_str().unwrap(),
    );
    let report = |extra: &[&str]| {
        let mut arguments = vec![
            "discharge",
            "--binding",
            binding,
            "--facts",
            facts,
            "--as-of",
            "2026-08-15T00:00:00Z",
            "--json",
        ];
        arguments.extend_from_slice(extra);
        let output = quoin(home.path(), &arguments);
        assert!(output.status.success(), "{output:?}");
        serde_json::from_str::<serde_json::Value>(&text(&output)).unwrap()
    };
    let with_store = report(&["--repo", repo]);
    assert_eq!(with_store["binding"]["direct"][0]["clauseId"], "C-1");

    let alone = report(&[]);
    assert!(alone["binding"]["direct"].as_array().unwrap().is_empty());
    let open = alone["binding"]["open"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["clauseId"] == "C-1")
        .unwrap();
    assert!(open["reason"].as_str().unwrap().starts_with("no-store"));

    // Stored facts are not read when --facts is given.
    let stored = quoin(
        home.path(),
        &[
            "evidence",
            "record-discharge",
            "--repo",
            repo,
            "--binding",
            binding,
            "--facts",
            facts,
        ],
    );
    assert!(stored.status.success(), "{stored:?}");
    let empty = dir.path().join("none.json");
    std::fs::write(&empty, "[]").unwrap();
    let output = quoin(
        home.path(),
        &[
            "discharge",
            "--binding",
            binding,
            "--facts",
            empty.to_str().unwrap(),
            "--repo",
            repo,
            "--as-of",
            "2026-08-15T00:00:00Z",
            "--json",
        ],
    );
    let ignored: serde_json::Value = serde_json::from_str(&text(&output)).unwrap();
    assert!(ignored["binding"]["direct"].as_array().unwrap().is_empty());
}

/// After `quoin evidence gc` removes a record a stored fact cites, the next
/// report has the clause open as `unresolved` naming the ref, and the stored
/// fact file is unchanged (FR-211-AC-14).
#[trace("FR-211-AC-14")]
#[test]
fn tc_211_a_collected_record_reopens_the_clause_and_leaves_the_stored_fact_alone() {
    use quoin_evidence::store::write_run;
    use quoin_evidence::types::{RunRecord, STORE_SCHEMA_VERSION};
    use quoin_evidence::{DiskEvidence, SuiteId};

    let run = |timestamp: &str| RunRecord {
        schema_version: STORE_SCHEMA_VERSION,
        suite: SuiteId::new("SUITE-A"),
        tool: "cargo test".to_owned(),
        evidence_kind: None,
        timestamp: timestamp.to_owned(),
        entries: Vec::new(),
    };
    let dir = tempfile::tempdir().unwrap();
    let home = installed_home();
    let mut store = DiskEvidence::new(dir.path());
    let older = write_run(&mut store, &run("2026-07-01T00:00:00Z")).unwrap();
    write_run(&mut store, &run("2026-07-02T00:00:00Z")).unwrap();
    let stem = older
        .trim_end_matches(".json")
        .rsplit('/')
        .next()
        .unwrap()
        .to_owned();
    let reference = format!("evidence://runs/SUITE-A/{stem}");
    let mut facts: serde_json::Value = serde_json::from_str(FACTS).unwrap();
    facts[0]["evidenceRefs"] = serde_json::json!([reference]);
    let (binding, facts_path) = (
        dir.path().join("binding.json"),
        dir.path().join("facts.json"),
    );
    std::fs::write(&binding, BINDING).unwrap();
    std::fs::write(&facts_path, facts.to_string()).unwrap();
    let (binding, facts_path, repo) = (
        binding.to_str().unwrap(),
        facts_path.to_str().unwrap(),
        dir.path().to_str().unwrap(),
    );
    let recorded = quoin(
        home.path(),
        &[
            "evidence",
            "record-discharge",
            "--repo",
            repo,
            "--binding",
            binding,
            "--facts",
            facts_path,
        ],
    );
    assert!(recorded.status.success(), "{recorded:?}");
    let report = || {
        let output = quoin(
            home.path(),
            &[
                "discharge",
                "--binding",
                binding,
                "--repo",
                repo,
                "--as-of",
                "2026-08-15T00:00:00Z",
                "--json",
            ],
        );
        assert!(output.status.success(), "{output:?}");
        serde_json::from_str::<serde_json::Value>(&text(&output)).unwrap()
    };
    let stored_fact = || {
        let roots = files_under(&dir.path().join("spec/evidence/discharge"));
        assert_eq!(roots.len(), 1);
        std::fs::read(&roots[0]).unwrap()
    };
    assert_eq!(report()["binding"]["direct"][0]["clauseId"], "C-1");
    let before = stored_fact();

    let collected = quoin(home.path(), &["evidence", "gc", "--repo", repo]);
    assert!(collected.status.success(), "{collected:?}");
    let after = report();
    assert!(after["binding"]["direct"].as_array().unwrap().is_empty());
    let reason = after["binding"]["open"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["clauseId"] == "C-1")
        .unwrap()["reason"]
        .as_str()
        .unwrap();
    assert!(reason.starts_with("unresolved: "), "{reason}");
    assert!(reason.contains(&reference), "{reason}");
    assert_eq!(stored_fact(), before);
}

/// The engineering-assurance flags need `--repo` on the JSON and the text path
/// alike (FR-211-AC-21).
#[trace("FR-211-AC-21")]
#[test]
fn tc_211_approval_flags_without_repo_are_refused_on_both_paths() {
    let dir = tempfile::tempdir().unwrap();
    let home = installed_home();
    let (binding, facts) = (
        dir.path().join("binding.json"),
        dir.path().join("facts.json"),
    );
    std::fs::write(&binding, BINDING).unwrap();
    std::fs::write(&facts, FACTS).unwrap();
    for json in [true, false] {
        let mut arguments = vec![
            "discharge",
            "--binding",
            binding.to_str().unwrap(),
            "--facts",
            facts.to_str().unwrap(),
            "--as-of",
            "2026-08-15T00:00:00Z",
            "--engineering-assurance",
            "/bin/true",
            "--allowed-signers",
            "/dev/null",
        ];
        if json {
            arguments.push("--json");
        }
        let output = quoin(home.path(), &arguments);
        assert!(!output.status.success(), "json={json}: {output:?}");
        assert!(output.stdout.is_empty(), "json={json}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("need --repo"),
            "json={json}: {output:?}"
        );
    }
}

/// A `record_id` over 256 bytes has no subject: non-zero exit, empty stdout
/// (FR-211-AC-18).
#[trace("FR-211-AC-18")]
#[test]
fn tc_211_a_record_id_over_the_bound_prints_no_subject() {
    let dir = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let mut binding: serde_json::Value = serde_json::from_str(BINDING).unwrap();
    binding["clauseSet"]["authority"] = "a".repeat(300).into();
    let (binding_path, fact_path) = (
        dir.path().join("binding.json"),
        dir.path().join("fact.json"),
    );
    std::fs::write(&binding_path, binding.to_string()).unwrap();
    let fact: serde_json::Value = serde_json::from_str(FACTS).unwrap();
    std::fs::write(&fact_path, fact[0].to_string()).unwrap();
    let output = quoin(
        home.path(),
        &[
            "evidence",
            "approval-subject",
            "--binding",
            binding_path.to_str().unwrap(),
            "--fact",
            fact_path.to_str().unwrap(),
        ],
    );
    assert!(!output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
}
