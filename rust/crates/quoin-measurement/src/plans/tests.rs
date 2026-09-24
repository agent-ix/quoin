// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! `MeasurementPlan` loader unit fixtures.

use super::{PlanLoadOptions, load_measurement_plans};
use crate::error::MeasurementErrorCode;
use crate::source::MemoryMeasurement;
use crate::types::plan::{LifecycleStatus, MeasurementStage};

fn document(id: &str, metric: &str, stage: &str) -> String {
    format!(
        "---\ntype: MeasurementPlan\nid: {id}\ntitle: T\nstatus: active\nstage: \
             {stage}\nmetric: {metric}\ndefinition_version: v1\nowner: o\n---\n\n# {id}\n"
    )
}

#[test]
fn plans_come_back_sorted_by_metric_then_id() {
    let source = MemoryMeasurement::new()
        .with_document("spec/assurance/b.md", document("MP-2", "zeta", "observe"))
        .with_document("spec/assurance/a.md", document("MP-3", "alpha", "ratchet"))
        .with_document("assurance/c.md", document("MP-1", "alpha", "trend"));
    let plans = load_measurement_plans(&source, PlanLoadOptions::default()).unwrap();
    let order: Vec<&str> = plans.iter().map(|plan| plan.id.as_str()).collect();
    assert_eq!(order, ["MP-1", "MP-3", "MP-2"]);
    assert_eq!(plans[0].stage, MeasurementStage::Trend);
    assert_eq!(plans[0].status, LifecycleStatus::Active);
}

/// Trace: FR-114-AC-1
/// Provenance: PLAT-1043
#[test]
fn tc_1940_runnable_procedure_must_be_a_protected_repo_file() {
    let front = "---\ntype: MeasurementPlan\nid: MP-1\ntitle: T\nstatus: active\
            \nstage: observe\nmetric: m\ndefinition_version: v1\
            \nexecution_procedure: spec/assurance/procedure.json\n";
    let protected =
        format!("{front}protected_apparatus:\n  - spec/assurance/procedure.json\n---\n\n# T\n");
    let plans = load_measurement_plans(
        &MemoryMeasurement::new().with_document("spec/assurance/MP-1.md", protected),
        PlanLoadOptions::default(),
    )
    .expect("protected procedure is runnable");
    assert_eq!(
        plans[0].execution_procedure.as_deref(),
        Some("spec/assurance/procedure.json")
    );

    let unprotected = format!("{front}---\n\n# T\n");
    let error = load_measurement_plans(
        &MemoryMeasurement::new().with_document("spec/assurance/MP-1.md", unprotected),
        PlanLoadOptions::default(),
    )
    .expect_err("unprotected procedure cannot be used");
    assert_eq!(error.code(), MeasurementErrorCode::PlanInvalid);
}

#[test]
fn governance_members_are_carried_only_when_asked_for() {
    let source =
        MemoryMeasurement::new().with_document("assurance/a.md", document("MP-1", "m", "ratchet"));
    let without = load_measurement_plans(&source, PlanLoadOptions::default()).unwrap();
    assert_eq!(without[0].owner, None);
    let with = load_measurement_plans(
        &source,
        PlanLoadOptions {
            include_governance: true,
        },
    )
    .unwrap();
    assert_eq!(with[0].owner.as_deref(), Some("o"));
}

#[test]
fn a_document_of_another_type_is_skipped_and_a_bad_stage_is_refused() {
    let other = MemoryMeasurement::new()
        .with_document(
            "assurance/a.md",
            "---\ntype: AssuranceProfile\nid: AP-1\n---\n",
        )
        .with_document("assurance/b.md", "# no frontmatter\n");
    assert!(
        load_measurement_plans(&other, PlanLoadOptions::default())
            .unwrap()
            .is_empty()
    );

    let bad = MemoryMeasurement::new()
        .with_document("assurance/a.md", document("MP-1", "m", "speculate"));
    let error = load_measurement_plans(&bad, PlanLoadOptions::default()).unwrap_err();
    assert_eq!(error.code(), MeasurementErrorCode::PlanInvalid);
    assert!(error.subject().contains("unknown measurement stage"));
}

/// The digest of the normalized text `"Pass when agreement exceeds 0.8."`,
/// computed once with `sha256sum` and asserted the same way every other
/// literal-expectation test in this repo is: as a fixed value the test
/// carries, not a re-derivation of the code under test.
const BAR_DIGEST: &str = "sha256:369547844a3d9b02bc99dde37cf4628787a0d78c1d98aba19caf110c416dcb8e";

fn document_with_bar(bar_digest: &str, bar_text: &str) -> String {
    format!(
        "---\ntype: MeasurementPlan\nid: MP-1\ntitle: T\nstatus: active\nstage: ratchet\n\
             metric: m\ndefinition_version: v1\nowner: o\npreregistration:\n  bar_digest: \
             {bar_digest}\n---\n\n# MP-1\n\n## Comparison and Enforcement\n\n{bar_text}\n"
    )
}

#[test]
fn a_bar_digest_matching_the_section_text_loads_and_matches() {
    let source = MemoryMeasurement::new().with_document(
        "spec/assurance/a.md",
        document_with_bar(BAR_DIGEST, "Pass when agreement exceeds 0.8."),
    );
    let plans = load_measurement_plans(&source, PlanLoadOptions::default()).unwrap();
    let prereg = plans[0]
        .preregistration
        .as_ref()
        .expect("a preregistration block");
    assert!(prereg.matches());
}

#[test]
fn a_stale_bar_digest_still_loads_but_does_not_match() {
    // The bar text was edited after `BAR_DIGEST` was recorded; loading a
    // plan never refuses for this by itself (PLAT-936: that refusal
    // belongs to `measurement_collection`, per observation, not here).
    let source = MemoryMeasurement::new().with_document(
        "spec/assurance/a.md",
        document_with_bar(BAR_DIGEST, "Pass when agreement exceeds 0.9."),
    );
    let plans = load_measurement_plans(&source, PlanLoadOptions::default()).unwrap();
    let prereg = plans[0]
        .preregistration
        .as_ref()
        .expect("a preregistration block");
    assert!(!prereg.matches());
}

#[test]
fn a_preregistration_block_with_no_valid_bar_digest_is_refused() {
    let source = MemoryMeasurement::new().with_document(
        "spec/assurance/a.md",
        document_with_bar("not-a-digest", "Pass when agreement exceeds 0.8."),
    );
    let error = load_measurement_plans(&source, PlanLoadOptions::default()).unwrap_err();
    assert_eq!(error.code(), MeasurementErrorCode::PlanInvalid);
    assert!(error.subject().contains("bar_digest"));
}

#[test]
fn a_preregistration_block_with_no_bar_section_is_refused() {
    let source = MemoryMeasurement::new().with_document(
        "spec/assurance/a.md",
        format!(
            "---\ntype: MeasurementPlan\nid: MP-1\ntitle: T\nstatus: active\nstage: ratchet\n\
                 metric: m\ndefinition_version: v1\nowner: o\npreregistration:\n  bar_digest: \
                 {BAR_DIGEST}\n---\n\n# MP-1\n\nNo such heading here.\n"
        ),
    );
    let error = load_measurement_plans(&source, PlanLoadOptions::default()).unwrap_err();
    assert_eq!(error.code(), MeasurementErrorCode::PlanInvalid);
    assert!(error.subject().contains("Comparison and Enforcement"));
}

#[test]
fn a_plan_with_no_preregistration_block_loads_with_none() {
    let source =
        MemoryMeasurement::new().with_document("assurance/a.md", document("MP-1", "m", "ratchet"));
    let plans = load_measurement_plans(&source, PlanLoadOptions::default()).unwrap();
    assert_eq!(plans[0].preregistration, None);
}

#[test]
fn bar_section_extraction_trims_blank_edges_and_stops_at_the_next_heading() {
    let text = "# Title\n\n## Comparison and Enforcement\n\n\nLine one.  \nLine two.\n\n\
                     ## Next Section\n\nignored\n";
    assert_eq!(
        super::bar_section(text).as_deref(),
        Some("Line one.\nLine two.")
    );
}

#[test]
fn bar_section_extraction_stops_at_a_shallower_heading_but_not_a_deeper_one() {
    let text = "## Comparison and Enforcement\n\nIntro.\n\n### Detail\n\nMore.\n\n\
                     # Next Top-Level Section\n\nignored\n";
    assert_eq!(
        super::bar_section(text).as_deref(),
        Some("Intro.\n\n### Detail\n\nMore.")
    );
}

#[test]
fn bar_section_extraction_finds_nothing_when_the_heading_is_absent() {
    assert_eq!(super::bar_section("# Title\n\nNo such heading.\n"), None);
}

/// The blocking defect the second review round found: a `#`-prefixed
/// shell comment inside a fenced code block pasted into the section was
/// misread as a heading, silently truncating the digested span so
/// everything after it (including real bar text) sat outside the digest,
/// free to edit with no refusal.
#[test]
fn bar_section_extraction_is_not_fooled_by_a_hash_comment_inside_a_code_fence() {
    let text = "## Comparison and Enforcement\n\n\
                     Run the gate before merging.\n\n\
                     ```bash\n\
                     # run the gate\n\
                     make rust-gate\n\
                     ```\n\n\
                     Pass when the gate is green.\n\n\
                     ## Next Section\n\nignored\n";
    assert_eq!(
        super::bar_section(text).as_deref(),
        Some(
            "Run the gate before merging.\n\n\
                 ```bash\n\
                 # run the gate\n\
                 make rust-gate\n\
                 ```\n\n\
                 Pass when the gate is green."
        )
    );
}

#[test]
fn bar_section_extraction_is_not_fooled_by_a_hash_comment_in_a_tilde_fence() {
    let text = "## Comparison and Enforcement\n\n\
                     ~~~python\n\
                     # not a heading\n\
                     ~~~\n\n\
                     Pass when agreement exceeds 0.8.\n";
    assert_eq!(
        super::bar_section(text).as_deref(),
        Some("~~~python\n# not a heading\n~~~\n\nPass when agreement exceeds 0.8.")
    );
}

#[test]
fn bar_section_extraction_ignores_a_shorter_or_mismatched_closing_fence() {
    // A two-backtick line does not close a three-backtick fence, and a
    // tilde line does not close a backtick fence either — so the `#` line
    // that follows stays inside the (still-open) fence in both cases.
    let text = "## Comparison and Enforcement\n\n\
                     ```\n\
                     ``\n\
                     ~~~\n\
                     # still fenced\n\
                     ```\n\n\
                     Pass when agreement exceeds 0.8.\n";
    assert_eq!(
        super::bar_section(text).as_deref(),
        Some("```\n``\n~~~\n# still fenced\n```\n\nPass when agreement exceeds 0.8.")
    );
}

#[test]
fn bar_section_extraction_ignores_a_heading_indented_as_code() {
    // Four or more leading spaces is CommonMark's own indented-code-block
    // rule; `heading_text` must not be fooled by a `#`-prefixed line
    // sitting in one, fenced or not.
    let text = "## Comparison and Enforcement\n\n\
                     Some prose.\n\n    \
                     # not a heading, this is indented code\n\n\
                     Pass when agreement exceeds 0.8.\n";
    assert_eq!(
        super::bar_section(text).as_deref(),
        Some(
            "Some prose.\n\n    # not a heading, this is indented code\n\n\
                 Pass when agreement exceeds 0.8."
        )
    );
}

/// PLAT-936 review finding #4: an empty section must refuse rather than
/// admit a vacuous digest over nothing, which would pass forever.
#[test]
fn a_preregistration_block_over_an_empty_bar_section_is_refused() {
    let source = MemoryMeasurement::new().with_document(
        "spec/assurance/a.md",
        format!(
            "---\ntype: MeasurementPlan\nid: MP-1\ntitle: T\nstatus: active\nstage: ratchet\n\
                 metric: m\ndefinition_version: v1\nowner: o\npreregistration:\n  bar_digest: \
                 {BAR_DIGEST}\n---\n\n# MP-1\n\n## Comparison and Enforcement\n\n## Next\n"
        ),
    );
    let error = load_measurement_plans(&source, PlanLoadOptions::default()).unwrap_err();
    assert_eq!(error.code(), MeasurementErrorCode::PlanInvalid);
    assert!(error.subject().contains("missing or empty"));
}

#[test]
fn bar_section_extraction_returns_none_for_an_empty_section() {
    assert_eq!(
        super::bar_section("## Comparison and Enforcement\n\n## Next\n"),
        None
    );
}

/// A bare `#` with nothing after it is still a heading (empty title),
/// matching `CommonMark` — confirmed here because the doc comment on
/// `heading_text` previously claimed the opposite (review finding #1).
#[test]
fn bar_section_extraction_treats_a_bare_hash_as_a_heading_that_ends_the_section() {
    let text =
        "## Comparison and Enforcement\n\nPass when agreement exceeds 0.8.\n\n#\n\nignored\n";
    assert_eq!(
        super::bar_section(text).as_deref(),
        Some("Pass when agreement exceeds 0.8.")
    );
}
