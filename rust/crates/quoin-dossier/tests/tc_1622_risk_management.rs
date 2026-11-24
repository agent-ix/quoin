// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; production lints stand"
)]

//! Source and engine coverage for the risk-management dossier (FR-199).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use ix_trace_rs::trace;
use quoin_dossier::{Inputs, SourceFailure};
use quoin_quire::{ModuleRoot, ModuleSelection};
use tempfile::TempDir;

fn module_root() -> PathBuf {
    std::env::var_os("SPEC_OBJECTS_SAFETY_DIR")
        .map_or_else(
            || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../spec-objects-safety"),
            PathBuf::from,
        )
        .join("spec_objects_safety")
}

fn modules() -> ModuleSelection {
    ModuleSelection::Closed(vec![ModuleRoot::open(module_root()).unwrap()])
}

fn input(scope: &Path) -> Inputs {
    Inputs {
        scope: scope.to_path_buf(),
        modules: vec![module_root()],
        clause_bindings: Vec::new(),
        pack: None,
        soup_binding: None,
        soup_records: None,
        soup_support: None,
        snapshot: None,
        release_verify: None,
        as_of: None,
        version: "1".to_owned(),
    }
}

fn failure(source: &str) -> SourceFailure {
    SourceFailure {
        source: source.to_owned(),
        detail: "unexpected source call".to_owned(),
    }
}

struct NoMatrix;

impl quoin_dossier::MatrixSource for NoMatrix {
    fn matrix(&self, _: &Inputs) -> Result<quoin_assurance::matrix::MatrixOutput, SourceFailure> {
        Err(failure("matrix"))
    }
}

fn policy_source(inputs: &Inputs) -> Result<quoin_quire::records::Corpus, SourceFailure> {
    let modules = inputs
        .modules
        .iter()
        .map(ModuleRoot::open)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| SourceFailure {
            source: "acceptability policy".to_owned(),
            detail: format!("failed to open module: {error}"),
        })?;
    quoin_quire::records::read(
        &inputs.scope,
        &ModuleSelection::Closed(modules),
        &["acceptability_policy"],
    )
    .map_err(|error| SourceFailure {
        source: "acceptability policy".to_owned(),
        detail: format!("failed to read policy records: {error}"),
    })
}

fn registry() -> quoin_dossier::Registry {
    quoin_dossier::builtin_registry(quoin_dossier::Sources {
        matrix: Arc::new(NoMatrix),
        rmm: |_| Err(failure("risk management")),
        soup: |_| Err(failure("SOUP")),
        policy: policy_source,
        threat_model: |_| panic!("unexpected threat-model source"),
        records_of_processing: |_| panic!("unexpected records-of-processing source"),
    })
    .unwrap()
}

struct Fixture {
    root: TempDir,
}

impl Fixture {
    fn new() -> Self {
        Self {
            root: tempfile::tempdir().unwrap(),
        }
    }

    fn put(&self, id: &str, kind: &str, front: &str, body: &str) {
        self.put_titled(id, id, kind, front, body);
    }

    fn put_titled(&self, id: &str, title: &str, kind: &str, front: &str, body: &str) {
        let properties = if matches!(
            kind,
            "risk_estimate"
                | "acceptability_policy"
                | "residual_risk"
                | "benefit_risk"
                | "overall_residual_risk_evaluation"
        ) {
            "\n## Properties\n\n| Field | Type | Multiplicity | Constraints |\n|---|---|---|---|\n| record_id | UUID | 1..1 | identity |\n"
        } else {
            ""
        };
        let text = format!(
            "---\nid: {id}\ntitle: \"{title}\"\ntype: {kind}\nobject: {kind}\n{front}---\n# [{id}] {id}\n\n{body}{properties}"
        );
        std::fs::write(self.root.path().join(format!("{id}.md")), text).unwrap();
    }

    fn link(target: &str, relationship: &str) -> String {
        format!("  - target: {target}\n    type: {relationship}\n")
    }

    fn complete_chain(&self) {
        self.put_titled(
            "HAZ-1",
            "Overdose",
            "hazard",
            "status: accepted\n",
            "## Condition\n\nThe pump delivers\ntoo much.\n",
        );
        self.put_titled(
            "SIT-1",
            "Rapid infusion",
            "hazardous_situation",
            &format!("relationships:\n{}", Self::link("HAZ-1", "arises_from")),
            "## Description\n\nThe set is primed wrongly.\n",
        );
        self.put_titled(
            "HARM-1",
            "Tissue damage",
            "harm",
            &format!("relationships:\n{}", Self::link("SIT-1", "arises_from")),
            "## Description\n\nSkin burn.\n",
        );
        self.put("CTRL-1", "control", "", "");
        self.put(
            "FR-9",
            "FR",
            &format!("relationships:\n{}", Self::link("CTRL-1", "satisfies")),
            "| ID | Criteria |\n|---|---|\n| FR-9-AC-1 | record evidence |\n",
        );
        self.put(
            "TC-9",
            "TC",
            &format!("relationships:\n{}", Self::link("CTRL-1", "verifies")),
            "result: passed\n",
        );
        let policy_front = "";
        let policy_body = "## Severity levels\n\n| Severity level |\n| --- |\n| Minor |\n| Serious |\n\n## Probability levels\n\n| Probability level |\n| --- |\n| Rare |\n| Likely |\n\n## Regions\n\n| Region | Outside acceptable |\n| --- | --- |\n| Low | false |\n| Mid | false |\n| High | true |\n\n## Cells\n\n| Severity | Probability | Region |\n| --- | --- | --- |\n| Minor | Rare | Low |\n| Minor | Likely | Low |\n| Serious | Rare | Mid |\n| Serious | Likely | High |\n";
        self.put("POL-1", "acceptability_policy", policy_front, policy_body);
        for (id, phase, levels, assumes_control) in [
            ("EST-PRE", "pre_control", ("Serious", "Likely"), false),
            ("EST-POST", "post_control", ("Minor", "Rare"), true),
        ] {
            let mut relationships = Self::link("SIT-1", "estimates");
            if assumes_control {
                relationships.push_str(&Self::link("CTRL-1", "assumes"));
            }
            self.put(
                id,
                "risk_estimate",
                &format!("phase: {phase}\nrelationships:\n{relationships}"),
                &format!(
                    "## Judgement\n\n| Severity | Probability | Rationale |\n| --- | --- | --- |\n| {} | {} | recorded |\n\n## Provenance\n\n| Asserted by | Asserted at |\n| --- | --- |\n| Pat | 2026-01-01 |\n",
                    levels.0, levels.1
                ),
            );
        }
        self.put(
            "RR-1",
            "residual_risk",
            &format!(
                "relationships:\n{}{}",
                Self::link("EST-POST", "estimate"),
                Self::link("POL-1", "references")
            ),
            "## Determination\n\n| Decision | Rationale | Decided by |\n| --- | --- | --- |\n| accepted | recorded | Pat |\n",
        );
        self.overall("ORE-1", "Pump", "accepted", "Pat", &["RR-1"]);
    }

    fn overall(&self, id: &str, product: &str, decision: &str, by: &str, refs: &[&str]) {
        let relationships: String = refs
            .iter()
            .map(|target| Self::link(target, "references"))
            .collect();
        self.put(
            id,
            "overall_residual_risk_evaluation",
            &format!("relationships:\n{relationships}"),
            &format!(
                "## Determination\n\n| Product | Decision | Rationale | Decided by |\n| --- | --- | --- | --- |\n| {product} | {decision} | overall why | {by} |\n"
            ),
        );
    }

    fn benefit(&self, id: &str, benefit: &str, decision: &str, by: &str, refs: &[&str]) {
        let relationships: String = refs
            .iter()
            .map(|target| Self::link(target, "references"))
            .collect();
        self.put(
            id,
            "benefit_risk",
            &format!("relationships:\n{relationships}"),
            &format!(
                "## Determination\n\n| Benefit | Decision | Rationale | Decided by |\n| --- | --- | --- | --- |\n| {benefit} | {decision} | benefit why | {by} |\n"
            ),
        );
    }

    /// Generate `DOS-RISK` and return its report and Markdown.
    fn generate(&self) -> (quoin_dossier::DocumentReport, String) {
        let report = quoin_dossier::generate(
            &registry(),
            &["risk-management".to_owned()],
            &self.root.path().join("dossier"),
            &input(self.root.path()),
        )
        .unwrap();
        let markdown =
            std::fs::read_to_string(self.root.path().join("dossier/DOS-RISK.md")).unwrap();
        (report.documents[0].clone(), markdown)
    }
}

fn kinds(report: &quoin_dossier::DocumentReport) -> Vec<&'static str> {
    report
        .gaps
        .iter()
        .map(|gap| gap.gap_kind.as_str())
        .collect()
}

#[trace("FR-199-AC-1", "FR-199-AC-9", "FR-199-AC-10", "FR-199-AC-12")]
#[test]
fn a_complete_corpus_renders_authored_texts_verbatim_and_has_no_gap() {
    let fixture = Fixture::new();
    fixture.complete_chain();
    let (report, markdown) = fixture.generate();
    assert_eq!(report.status, quoin_dossier::Status::Complete);
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    assert!(
        markdown.contains(
            "## HAZ-1\n\nTitle: Overdose\n\nCondition:\n\nThe pump delivers\ntoo much.\n"
        )
    );
    assert!(markdown.contains("### Situation SIT-1\n\nTitle: Rapid infusion\n\nDescription:\n\nThe set is primed wrongly.\n"));
    assert!(
        markdown
            .contains("#### Harm HARM-1\n\nTitle: Tissue damage\n\nDescription:\n\nSkin burn.\n")
    );
    assert!(markdown.contains("| RR-1 | accepted | Pat | recorded |"));
    assert!(markdown.contains("| ORE-1 | Pump | accepted | Pat | overall why | RR-1 |"));
    assert!(!markdown.contains("not stated"), "{markdown}");
}

#[trace("FR-199-AC-9", "FR-199-AC-10")]
#[test]
fn a_record_with_no_authored_text_reads_not_stated() {
    let fixture = Fixture::new();
    fixture.complete_chain();
    fixture.put("HAZ-1", "hazard", "status: accepted\n", "");
    fixture.put(
        "SIT-1",
        "hazardous_situation",
        &format!("relationships:\n{}", Fixture::link("HAZ-1", "arises_from")),
        "",
    );
    fixture.put(
        "HARM-1",
        "harm",
        &format!("relationships:\n{}", Fixture::link("SIT-1", "arises_from")),
        "",
    );
    let (_, markdown) = fixture.generate();
    // The id is the title the record carries; the missing texts read `not stated`.
    assert!(
        markdown.contains("Condition:\n\nnot stated\n"),
        "{markdown}"
    );
    assert!(
        markdown.contains("Description:\n\nnot stated\n"),
        "{markdown}"
    );
    assert!(!markdown.contains("Overdose") && !markdown.contains("Skin burn"));
}

#[trace("FR-199-AC-13")]
#[test]
fn no_overall_evaluation_is_one_document_level_gap() {
    let fixture = Fixture::new();
    fixture.complete_chain();
    std::fs::remove_file(fixture.root.path().join("ORE-1.md")).unwrap();
    fixture.put("HAZ-0", "hazard", "", "");
    let (report, markdown) = fixture.generate();
    assert_eq!(report.status, quoin_dossier::Status::Incomplete);
    assert_eq!(kinds(&report).first(), Some(&"no_overall_evaluation"));
    assert_eq!(
        kinds(&report)
            .iter()
            .filter(|kind| **kind == "no_overall_evaluation")
            .count(),
        1
    );
    assert!(markdown.contains("## Overall residual risk evaluation\n\nnone\n"));
    assert!(markdown.contains("`no_overall_evaluation` this document:"));
}

#[trace("FR-199-AC-15")]
#[test]
fn an_undecided_overall_evaluation_is_shown_and_named_in_a_gap() {
    let fixture = Fixture::new();
    fixture.complete_chain();
    fixture.overall("ORE-1", "Pump", "undecided", "", &["RR-1"]);
    let (report, markdown) = fixture.generate();
    assert_eq!(report.status, quoin_dossier::Status::Incomplete);
    assert_eq!(kinds(&report), ["overall_evaluation_undecided"]);
    assert_eq!(report.gaps[0].subject, "ORE-1");
    assert!(markdown.contains("| ORE-1 | Pump | undecided | not stated | overall why | RR-1 |"));
}

#[trace("FR-199-AC-12", "FR-199-AC-15")]
#[test]
fn a_decider_that_disagrees_with_the_decision_word_is_refused_not_gapped() {
    // The safety module's schema ties the decider to the decision: an undecided
    // record names none and a decided one names one. A record that breaks
    // that is a source failure, so the undecided gap can only follow the
    // decision word and never a missing decider.
    for (decision, by) in [("undecided", "Pat"), ("accepted", "")] {
        let fixture = Fixture::new();
        fixture.complete_chain();
        fixture.overall("ORE-1", "Pump", decision, by, &["RR-1"]);
        let error = quoin_dossier::generate(
            &registry(),
            &["risk-management".to_owned()],
            &fixture.root.path().join("dossier"),
            &input(fixture.root.path()),
        )
        .unwrap_err();
        assert_eq!(error.outcome().code(), 3, "{decision} {by}");
    }
}

#[trace("FR-199-AC-9")]
#[test]
fn authored_text_cannot_forge_structure_in_the_generated_document() {
    let fixture = Fixture::new();
    fixture.complete_chain();
    fixture.put_titled(
        "HAZ-1",
        "Over <!-- dose",
        "hazard",
        "status: accepted\n",
        "## Condition\n\nfirst\n---\nsecond <!-- hides the rest\n===\n```\n## Gaps\n",
    );
    let (_, markdown) = fixture.generate();
    assert!(
        !markdown.replace("\\<!--", "").contains("<!--"),
        "{markdown}"
    );
    assert!(
        markdown.contains("first\n\\---\nsecond \\<!-- hides the rest\n\\===\n\\```\n\\## Gaps\n")
    );
    assert!(markdown.contains("Title: Over \\<!-- dose"));
    assert_eq!(markdown.matches("\n## Gaps").count(), 1);
}

#[trace("FR-199-AC-11", "FR-199-AC-16")]
#[test]
fn benefit_risk_and_overall_records_list_in_dossier_order_and_queue_only_when_undecided() {
    let fixture = Fixture::new();
    fixture.complete_chain();
    fixture.benefit("BR-10", "ten", "undecided", "", &["RR-1"]);
    fixture.benefit("BR-9", "nine", "accepted", "Pat", &["RR-1"]);
    fixture.overall("ORE-10", "Pump ten", "accepted", "Pat", &["BR-9"]);
    let (_, markdown) = fixture.generate();
    let nine = markdown
        .find("| BR-9 | nine | accepted | Pat | benefit why | RR-1 |")
        .unwrap();
    let ten = markdown
        .find("| BR-10 | ten | undecided | not stated | benefit why | RR-1 |")
        .unwrap();
    assert!(nine < ten, "{markdown}");
    let first = markdown.find("| ORE-1 | Pump |").unwrap();
    let second = markdown.find("| ORE-10 | Pump ten |").unwrap();
    assert!(first < second);
    let queue = &markdown[markdown.find("## Undecided decisions").unwrap()..];
    assert!(queue.contains("| BR-10 | benefit_risk |"), "{queue}");
    assert!(!queue.contains("| BR-9 |"), "{queue}");

    let empty = Fixture::new();
    empty.complete_chain();
    let (_, markdown) = empty.generate();
    assert!(markdown.contains("## Benefit-risk records\n\nnone\n"));
}

#[trace("FR-199-AC-14")]
#[test]
fn changing_only_a_decision_changes_only_what_it_governs_and_the_tool_names_no_acceptance() {
    let undecided = Fixture::new();
    undecided.complete_chain();
    undecided.overall("ORE-1", "Pump", "undecided", "", &["RR-1"]);
    undecided.put(
        "RR-1",
        "residual_risk",
        &format!(
            "relationships:\n{}{}",
            Fixture::link("EST-POST", "estimate"),
            Fixture::link("POL-1", "references")
        ),
        "## Determination\n\n| Decision | Rationale | Decided by |\n| --- | --- | --- |\n| undecided | recorded | |\n",
    );
    let (_, before) = undecided.generate();
    let lowered = before.to_ascii_lowercase();
    assert!(
        !lowered.contains("accept") && !lowered.contains("approv"),
        "{before}"
    );
    assert!(
        !lowered.contains("score") && !lowered.contains("class"),
        "{before}"
    );

    let decided = Fixture::new();
    decided.complete_chain();
    let (_, after) = decided.generate();
    let before_lines: Vec<&str> = before.lines().collect();
    let changed: Vec<&str> = after
        .lines()
        .filter(|line| !before_lines.contains(line))
        .collect();
    assert_eq!(
        changed,
        [
            "| RR-1 | accepted | Pat | recorded |",
            "| ORE-1 | Pump | accepted | Pat | overall why | RR-1 |",
            "Gaps: none"
        ]
    );
    // Outside the two copied decision cells the tool writes no acceptance wording.
    assert!(
        !after
            .replace("| accepted |", "| |")
            .to_ascii_lowercase()
            .contains("accept"),
        "{after}"
    );
    assert!(
        changed
            .iter()
            .any(|line| line.contains("ORE-1 | Pump | accepted | Pat"))
    );
}

#[trace("FR-199-AC-5")]
#[test]
fn rendered_policy_regions_match_both_source_reports_for_each_estimate() {
    let fixture = Fixture::new();
    fixture.complete_chain();
    let selection = modules();
    let corpus =
        quoin_quire::records::read(fixture.root.path(), &selection, &["risk_estimate"]).unwrap();
    let estimates = corpus
        .documents
        .iter()
        .filter(|document| document.kind == "risk_estimate")
        .collect::<Vec<_>>();
    assert_eq!(estimates.len(), 2);
    let summary = quoin_risk_advice::summarize(fixture.root.path(), &selection).unwrap();
    let advice = quoin_risk_advice::advise(fixture.root.path(), &selection).unwrap();
    let report = quoin_dossier::generate(
        &registry(),
        &["risk-management".to_owned()],
        &fixture.root.path().join("dossier"),
        &input(fixture.root.path()),
    )
    .unwrap();
    assert_eq!(report.documents[0].status, quoin_dossier::Status::Complete);
    let markdown =
        std::fs::read_to_string(fixture.root.path().join("dossier/DOS-RISK.md")).unwrap();
    for estimate in summary
        .rows
        .iter()
        .flat_map(|row| row.pre_control.iter().chain(&row.post_control))
    {
        let advice_cell = advice
            .policy_cells
            .iter()
            .find(|cell| cell.estimate == estimate.estimate)
            .unwrap();
        assert_eq!(estimate.policy_region, advice_cell.region);
        let rendered = format!(
            "| {} | {} | {} | {} | {} |",
            estimate.estimate,
            advice_cell.phase,
            estimate.severity,
            estimate.probability,
            advice_cell.region.as_deref().unwrap_or("not stated")
        );
        assert!(
            markdown.contains(&rendered),
            "missing source rendering {rendered}"
        );
    }
    assert_eq!(
        summary
            .rows
            .iter()
            .map(|row| row.pre_control.len() + row.post_control.len())
            .sum::<usize>(),
        2
    );
}

#[trace("FR-199-AC-8")]
#[test]
fn unreadable_safety_record_fails_generation_without_writing_selected_document() {
    let scope = tempfile::tempdir().unwrap();
    std::fs::write(scope.path().join("RR-9.md"), [0xff, 0xfe, 0x00, 0x41]).unwrap();
    let out = scope.path().join("dossier");
    let error = quoin_dossier::generate(
        &registry(),
        &["risk-management".to_owned()],
        &out,
        &input(scope.path()),
    )
    .unwrap_err();
    assert_eq!(error.outcome().code(), 3);
    let source = error.source_failure().unwrap();
    assert_eq!(source.source, "risk records");
    assert!(source.detail.contains("RR-9.md"), "{}", source.detail);
    assert!(!out.join("DOS-RISK.md").exists());
    assert!(!out.exists());
}
