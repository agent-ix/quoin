// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! `quoin anomaly check` fix-verification evidence findings (FR-175).
//!
//! Every test builds its temporary scope at test time from the
//! `agent-ix/spec-artifacts-process` checkout, read in place (from
//! `SPEC_ARTIFACTS_PROCESS_DIR` or the checkout next to this repository): the
//! anomaly fixtures and the corpus are copied into a temp dir, the evidence
//! store is written there, and nothing is committed here. The checkout must
//! hold the `Test (TC-001)` Verification cell for `FR-001-AC-1`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;
use quoin_evidence::paths::{RUNS_DIR, record_path};
use quoin_evidence::store::{bind, write_bindings, write_run};
use quoin_evidence::types::{Binding, RunRecord};
use quoin_evidence::{DiskEvidence, ObligationId, Outcome, RunEntry, SuiteId, SymbolId};

const FAILING: &str = "warning anomaly.fix-result-failing (failing-result)";
const MISSING: &str = "warning anomaly.fix-test-missing-result (missing-result)";

/// The resolved checkout, canonicalised: `quire` rejects `..` in `--module`.
fn process_root() -> PathBuf {
    let root = std::env::var_os("SPEC_ARTIFACTS_PROCESS_DIR").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../spec-artifacts-process"),
        PathBuf::from,
    );
    std::fs::canonicalize(&root).unwrap_or_else(|error| {
        panic!(
            "the spec-artifacts-process checkout is not at {}: {error}. These tests read \
             the documents that repository ships in place and keep no copy: check out \
             agent-ix/spec-artifacts-process next to this repository, or set \
             SPEC_ARTIFACTS_PROCESS_DIR to its checkout",
            root.display()
        )
    })
}

fn fixtures() -> PathBuf {
    process_root().join("tests/fixtures/anomaly")
}

fn fixture(group: &str, name: &str) -> String {
    std::fs::read_to_string(fixtures().join(group).join(format!("{name}.md"))).unwrap()
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
    out.sort();
    out
}

/// A scope under test: the shipped corpus, one anomaly and an evidence store.
struct Scope {
    dir: tempfile::TempDir,
    hazard_module: tempfile::TempDir,
}

impl Scope {
    /// The corpus plus the fixture `group/name` as `AN-001`.
    fn of(group: &str, name: &str) -> Self {
        Self::with_text(&fixture(group, name))
    }

    fn with_text(text: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let corpus = fixtures().join("corpus/spec");
        for entry in walk(&corpus) {
            let target = dir
                .path()
                .join("spec")
                .join(entry.strip_prefix(&corpus).unwrap());
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::copy(&entry, target).unwrap();
        }
        let anomalies = dir.path().join("spec/anomalies");
        std::fs::create_dir_all(&anomalies).unwrap();
        std::fs::write(anomalies.join("AN-001.md"), text).unwrap();
        let hazard_module = tempfile::tempdir().unwrap();
        std::fs::write(
            hazard_module.path().join("manifest.yaml"),
            "name: spec-objects-safety\nversion: 0.0.0\nobject_types:\n- name: hazard\n",
        )
        .unwrap();
        Self { dir, hazard_module }
    }

    fn store(&self) -> DiskEvidence {
        DiskEvidence::new(self.dir.path())
    }

    /// Write one run of `suite` at `timestamp`; each entry carries the trace ids
    /// and the outcome given.
    fn run(&self, suite: &str, timestamp: &str, entries: &[(&[&str], Outcome)]) {
        let entries = entries
            .iter()
            .enumerate()
            .map(|(index, (ids, outcome))| {
                let mut entry = RunEntry::new(format!("test_{index}"), *outcome);
                entry.trace_ids = Some(ids.iter().map(|id| (*id).to_owned()).collect());
                entry
            })
            .collect();
        write_run(
            &mut self.store(),
            &RunRecord {
                schema_version: 1,
                suite: SuiteId::new(suite),
                tool: "fixture 1".to_owned(),
                evidence_kind: None,
                timestamp: timestamp.to_owned(),
                entries,
            },
        )
        .unwrap();
    }

    fn check(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quoin"))
            .args(["anomaly", "check"])
            .arg(self.dir.path())
            .arg("--module")
            .arg(process_root().join("spec_artifacts_process"))
            .arg("--module")
            .arg(self.hazard_module.path())
            .output()
            .unwrap()
    }

    /// The check's standard output, with the exit status asserted to be 0.
    fn report(&self) -> String {
        let out = self.check();
        let stdout = String::from_utf8(out.stdout).unwrap();
        assert_eq!(
            out.status.code(),
            Some(0),
            "{stdout}{}",
            String::from_utf8_lossy(&out.stderr)
        );
        stdout
    }

    fn files(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        walk(self.dir.path())
            .into_iter()
            .map(|path| {
                let bytes = std::fs::read(&path).unwrap();
                (path, bytes)
            })
            .collect()
    }
}

fn count(report: &str, needle: &str) -> usize {
    report.lines().filter(|line| line.contains(needle)).count()
}

fn neither(report: &str) -> bool {
    !report.contains("anomaly.fix-result-failing")
        && !report.contains("anomaly.fix-test-missing-result")
}

const TEST_ONLY: (&str, &str) = ("valid", "closed-fixed-test-only");
const CRITERION_ONLY: (&str, &str) = ("valid", "closed-fixed-criterion-only");

#[trace("FR-175-AC-1")]
#[test]
fn a_newest_passing_entry_for_the_test_yields_neither_code() {
    let scope = Scope::of(TEST_ONLY.0, TEST_ONLY.1);
    scope.run(
        "SUITE-1",
        "2026-01-01T00:00:00Z",
        &[(&["TC-001"], Outcome::Pass)],
    );
    assert!(neither(&scope.report()));
}

#[trace("FR-175-AC-2")]
#[test]
fn a_failing_or_erroring_entry_yields_a_warning_and_leaves_the_exit_status() {
    let baseline = Scope::of(TEST_ONLY.0, TEST_ONLY.1).check().status.code();
    for outcome in [Outcome::Fail, Outcome::Error] {
        let scope = Scope::of(TEST_ONLY.0, TEST_ONLY.1);
        scope.run("SUITE-1", "2026-01-01T00:00:00Z", &[(&["TC-001"], outcome)]);
        let out = scope.check();
        let report = String::from_utf8(out.stdout).unwrap();
        assert_eq!(count(&report, FAILING), 1, "{report}");
        assert!(report.contains("`TC-001`"), "{report}");
        assert!(
            !report.contains("anomaly.fix-test-missing-result"),
            "{report}"
        );
        assert_eq!(out.status.code(), baseline);
        assert_eq!(out.status.code(), Some(0));
    }
}

#[trace("FR-175-AC-3")]
#[test]
fn no_entry_only_a_skip_or_no_store_yields_the_missing_result_code() {
    let no_store = Scope::of(TEST_ONLY.0, TEST_ONLY.1);
    assert!(!no_store.dir.path().join("spec/evidence").exists());
    let skipped = Scope::of(TEST_ONLY.0, TEST_ONLY.1);
    skipped.run(
        "SUITE-1",
        "2026-01-01T00:00:00Z",
        &[(&["TC-001"], Outcome::Skip)],
    );
    let other = Scope::of(TEST_ONLY.0, TEST_ONLY.1);
    other.run(
        "SUITE-1",
        "2026-01-01T00:00:00Z",
        &[(&["TC-002"], Outcome::Pass)],
    );
    for scope in [no_store, skipped, other] {
        let report = scope.report();
        assert_eq!(count(&report, MISSING), 1, "{report}");
        assert!(report.contains("`TC-001`"), "{report}");
        assert!(!report.contains("anomaly.fix-result-failing"), "{report}");
    }
}

#[trace("FR-175-AC-4")]
#[test]
fn the_newest_run_of_a_suite_decides_and_a_binding_alone_does_not_pass() {
    let regressed = Scope::of(TEST_ONLY.0, TEST_ONLY.1);
    regressed.run(
        "SUITE-1",
        "2026-01-01T00:00:00Z",
        &[(&["TC-001"], Outcome::Pass)],
    );
    regressed.run(
        "SUITE-1",
        "2026-01-02T00:00:00Z",
        &[(&["TC-001"], Outcome::Fail)],
    );
    assert_eq!(count(&regressed.report(), FAILING), 1);

    let repaired = Scope::of(TEST_ONLY.0, TEST_ONLY.1);
    repaired.run(
        "SUITE-1",
        "2026-01-01T00:00:00Z",
        &[(&["TC-001"], Outcome::Fail)],
    );
    repaired.run(
        "SUITE-1",
        "2026-01-02T00:00:00Z",
        &[(&["TC-001"], Outcome::Pass)],
    );
    assert!(neither(&repaired.report()));

    let bound = Scope::of(TEST_ONLY.0, TEST_ONLY.1);
    let outcome = bind(
        &[],
        &Binding {
            obligation: ObligationId::new("TC-001"),
            suite: SuiteId::new("SUITE-1"),
            symbols: vec![SymbolId::new("test_0")],
            lineage: None,
        },
    );
    write_bindings(&mut bound.store(), &outcome.bindings).unwrap();
    assert!(
        bound
            .dir
            .path()
            .join("spec/evidence/bindings.json")
            .exists()
    );
    let report = bound.report();
    assert_eq!(count(&report, MISSING), 1, "{report}");
}

#[trace("FR-175-AC-5")]
#[test]
fn a_failure_in_any_suites_newest_run_wins() {
    let scope = Scope::of(TEST_ONLY.0, TEST_ONLY.1);
    scope.run(
        "SUITE-1",
        "2026-01-01T00:00:00Z",
        &[(&["TC-001"], Outcome::Pass)],
    );
    scope.run(
        "SUITE-2",
        "2026-01-01T00:00:00Z",
        &[(&["TC-001"], Outcome::Fail)],
    );
    let report = scope.report();
    assert_eq!(count(&report, FAILING), 1, "{report}");
    assert!(
        !report.contains("anomaly.fix-test-missing-result"),
        "{report}"
    );
}

#[trace("FR-175-AC-6")]
#[test]
fn an_unreadable_run_file_leaves_its_suite_without_an_entry_whichever_is_older() {
    for (broken_at, readable_at) in [
        ("2026-01-01T00:00:00Z", "2026-01-02T00:00:00Z"),
        ("2026-01-02T00:00:00Z", "2026-01-01T00:00:00Z"),
    ] {
        let scope = Scope::of(TEST_ONLY.0, TEST_ONLY.1);
        scope.run("SUITE-1", readable_at, &[(&["TC-001"], Outcome::Pass)]);
        let broken = scope.dir.path().join("spec/evidence").join(record_path(
            RUNS_DIR,
            &SuiteId::new("SUITE-1"),
            broken_at,
        ));
        std::fs::write(broken, "{ not json").unwrap();
        let report = scope.report();
        assert_eq!(count(&report, MISSING), 1, "{report}");
    }
}

#[trace("FR-175-AC-7")]
#[test]
fn a_criterion_only_table_is_reported_only_for_a_failing_result() {
    let none = Scope::of(CRITERION_ONLY.0, CRITERION_ONLY.1);
    assert!(neither(&none.report()));

    let passing = Scope::of(CRITERION_ONLY.0, CRITERION_ONLY.1);
    passing.run(
        "SUITE-1",
        "2026-01-01T00:00:00Z",
        &[(&["FR-001-AC-1"], Outcome::Pass)],
    );
    assert!(neither(&passing.report()));

    let failing = Scope::of(CRITERION_ONLY.0, CRITERION_ONLY.1);
    failing.run(
        "SUITE-1",
        "2026-01-01T00:00:00Z",
        &[(&["FR-001-AC-1"], Outcome::Fail)],
    );
    let report = failing.report();
    assert_eq!(count(&report, FAILING), 1, "{report}");
    assert!(report.contains("`FR-001-AC-1`"), "{report}");
}

#[trace("FR-175-AC-8")]
#[test]
fn a_criterion_matches_the_test_case_its_verification_cell_names() {
    let failing = Scope::of(CRITERION_ONLY.0, CRITERION_ONLY.1);
    failing.run(
        "SUITE-1",
        "2026-01-01T00:00:00Z",
        &[(&["TC-001"], Outcome::Fail)],
    );
    let report = failing.report();
    assert_eq!(count(&report, FAILING), 1, "{report}");
    assert!(report.contains("`FR-001-AC-1`"), "{report}");

    let passing = Scope::of(CRITERION_ONLY.0, CRITERION_ONLY.1);
    passing.run(
        "SUITE-1",
        "2026-01-01T00:00:00Z",
        &[(&["TC-001"], Outcome::Pass)],
    );
    assert!(neither(&passing.report()));

    // The newest run is the one that counts on the indirect route too.
    let repaired = Scope::of(CRITERION_ONLY.0, CRITERION_ONLY.1);
    repaired.run(
        "SUITE-1",
        "2026-01-01T00:00:00Z",
        &[(&["TC-001"], Outcome::Fail)],
    );
    repaired.run(
        "SUITE-1",
        "2026-01-02T00:00:00Z",
        &[(&["TC-001"], Outcome::Pass)],
    );
    assert!(neither(&repaired.report()));
}

#[trace("FR-175-AC-9")]
#[test]
fn a_criterion_row_and_a_test_row_are_judged_independently() {
    let both = || Scope::of("valid", "closed-fixed");

    let passing_test = both();
    passing_test.run(
        "SUITE-1",
        "2026-01-01T00:00:00Z",
        &[(&["TC-001"], Outcome::Pass)],
    );
    assert!(neither(&passing_test.report()));

    let failing_criterion = both();
    failing_criterion.run(
        "SUITE-1",
        "2026-01-01T00:00:00Z",
        &[
            (&["TC-001"], Outcome::Pass),
            (&["FR-001-AC-1"], Outcome::Fail),
        ],
    );
    let report = failing_criterion.report();
    assert_eq!(count(&report, FAILING), 1, "{report}");
    assert!(report.contains("Criterion row `FR-001-AC-1`"), "{report}");
    assert!(
        !report.contains("anomaly.fix-test-missing-result"),
        "{report}"
    );

    let missing_test = both();
    missing_test.run(
        "SUITE-1",
        "2026-01-01T00:00:00Z",
        &[(&["FR-001-AC-1"], Outcome::Pass)],
    );
    let report = missing_test.report();
    assert_eq!(count(&report, MISSING), 1, "{report}");
    assert!(report.contains("Test row `TC-001`"), "{report}");
    assert_eq!(count(&report, FAILING), 0, "{report}");
}

#[trace("FR-175-AC-10")]
#[test]
fn a_criterion_row_naming_a_document_is_not_evaluated() {
    for target in ["FR-001", "NFR-002", "HAZ-001"] {
        let text = fixture(CRITERION_ONLY.0, CRITERION_ONLY.1).replace(
            "| Criterion | FR-001-AC-1 |",
            &format!("| Criterion | {target} |"),
        );
        assert!(text.contains(&format!("| Criterion | {target} |")));
        let scope = Scope::with_text(&text);
        scope.run(
            "SUITE-1",
            "2026-01-01T00:00:00Z",
            &[(&[target], Outcome::Fail), (&["TC-001"], Outcome::Fail)],
        );
        assert!(neither(&scope.report()), "{target}");
    }
}

#[trace("FR-175-AC-11")]
#[test]
fn only_a_fixed_resolved_or_closed_anomaly_is_evaluated() {
    let failing = |scope: &Scope| {
        scope.run(
            "SUITE-1",
            "2026-01-01T00:00:00Z",
            &[(&["TC-001"], Outcome::Fail)],
        );
    };
    let resolved = Scope::of("valid", "resolved-fixed");
    failing(&resolved);
    assert!(
        resolved.report().contains("anomaly.fix-result-failing"),
        "a resolved fixed anomaly is evaluated like a closed one"
    );

    let table = "\n## Fix Verification\n\n| Kind | Target | Rationale |\n|------|--------|-----------|\n| Test | TC-001 | The fix is demonstrated here. |\n";
    for name in [
        "closed-not-a-defect",
        "closed-duplicate",
        "closed-wont-fix",
        "assessed",
    ] {
        let text = format!("{}{table}", fixture("valid", name));
        let scope = Scope::with_text(&text);
        failing(&scope);
        assert!(neither(&scope.report()), "{name}");
    }
}

#[trace("FR-175-AC-12")]
#[test]
fn a_dangling_row_is_skipped_and_a_broken_table_yields_no_record() {
    let dangling = Scope::of("invalid", "verification-dangling-test");
    dangling.run(
        "SUITE-1",
        "2026-01-01T00:00:00Z",
        &[(&["TC-999"], Outcome::Fail)],
    );
    let out = dangling.check();
    let report = String::from_utf8(out.stdout).unwrap();
    assert!(neither(&report), "{report}");
    assert!(!report.to_lowercase().contains("dangling"), "{report}");
    assert!(!report.contains("TC-999"), "{report}");

    for name in ["verification-bad-headers", "verification-no-rows"] {
        let scope = Scope::of("invalid", name);
        scope.run(
            "SUITE-1",
            "2026-01-01T00:00:00Z",
            &[(&["TC-001"], Outcome::Fail)],
        );
        let out = scope.check();
        let report = String::from_utf8(out.stdout).unwrap();
        assert!(
            report.contains("error anomaly.no-record"),
            "{name}: {report}"
        );
        assert!(neither(&report), "{name}: {report}");
        assert_eq!(out.status.code(), Some(1), "{name}");
    }
}

#[trace("FR-175-AC-13")]
#[test]
fn a_finding_carries_code_severity_reason_status_line_and_target() {
    let failing = Scope::of(TEST_ONLY.0, TEST_ONLY.1);
    failing.run(
        "SUITE-1",
        "2026-01-01T00:00:00Z",
        &[(&["TC-001"], Outcome::Fail)],
    );
    let missing = Scope::of(TEST_ONLY.0, TEST_ONLY.1);
    let status_line = fixture(TEST_ONLY.0, TEST_ONLY.1)
        .lines()
        .position(|line| line.starts_with("status:"))
        .unwrap()
        + 1;
    for (scope, header) in [(failing, FAILING), (missing, MISSING)] {
        let report = scope.report();
        let finding = report
            .lines()
            .find(|line| line.contains(header))
            .unwrap_or_else(|| panic!("{report}"));
        assert!(
            finding.contains(&format!("AN-001.md:{status_line}: {header}")),
            "{finding}"
        );
        assert!(finding.contains("`TC-001`"), "{finding}");
    }
}

#[trace("FR-175-AC-14")]
#[test]
fn the_check_writes_nothing_is_deterministic_and_never_says_a_fix_is_good() {
    for (outcomes, expected) in [
        (vec![Outcome::Fail], FAILING),
        (vec![Outcome::Skip], MISSING),
    ] {
        let scope = Scope::of("valid", "closed-fixed");
        scope.run(
            "SUITE-1",
            "2026-01-01T00:00:00Z",
            &[
                (&["TC-001"], outcomes[0]),
                (&["FR-001-AC-1"], Outcome::Fail),
            ],
        );
        let before = scope.files();
        let first = scope.check();
        let second = scope.check();
        assert_eq!(before, scope.files());
        assert_eq!(first.stdout, second.stdout);
        let report = String::from_utf8(first.stdout).unwrap().to_lowercase();
        assert!(report.contains(expected), "{report}");
        for word in ["verified", "accepted", "acceptable"] {
            assert!(!report.contains(word), "{word}: {report}");
        }
    }
}

#[trace("FR-175-AC-15")]
#[test]
fn the_scope_is_built_from_the_sibling_checkout_at_a_state_with_the_test_cell() {
    let criterion = std::fs::read_to_string(
        fixtures().join("corpus/spec/functional/FR-001-verify-checksums.md"),
    )
    .unwrap();
    assert!(
        criterion.contains("| Test (TC-001) |"),
        "the spec-artifacts-process checkout predates the `Test (TC-001)` cell for FR-001-AC-1"
    );
    let scope = Scope::of(TEST_ONLY.0, TEST_ONLY.1);
    assert!(
        scope
            .dir
            .path()
            .join("spec/functional/FR-001-verify-checksums.md")
            .is_file()
    );
}
