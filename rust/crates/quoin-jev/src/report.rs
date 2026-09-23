// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rendering `FrVerdict`s into a `SpecReview` document body (PLAT-837).
//!
//! Matches the contract `spec-artifacts-process`'s `SpecReview.md` skeleton
//! states (read from the installed copy at
//! `~/.ix/filament/modules/spec-artifacts-process/skeletons/SpecReview.md`
//! while building this): a `## Summary` and a `## Findings` table whose
//! header is exactly `| ID | Severity | Summary | Refs |`, `FND-NNN` ids, and
//! `Severity` in `low`/`medium`/`high`. This module writes the body only --
//! frontmatter (`id`, `scope`, `review_set`) is a decision for whoever
//! selects the review's id and scope, not something to invent here.

use std::fmt::Write as _;

use crate::verdict::{Certainty, FrVerdict, SubQuestionCheck};

/// One FR's findings, paired with the FR id they are about (for `Refs`).
pub struct FrReport<'a> {
    /// The FR this verdict is about, e.g. `FR-079`.
    pub fr_id: &'a str,
    /// The verdict [`crate::lens::run`] produced for it.
    pub verdict: &'a FrVerdict,
}

/// Renders the `## Summary` and `## Findings` sections for every FR in
/// `reports`, numbering findings `FND-001`, `FND-002`, ... in the order
/// given.
///
/// A `sound` AC row contributes no table row (per `SpecReview`'s own
/// contract: only rows that name something worth a reader's attention
/// belong in `## Findings`); it IS counted in the summary line, so a review
/// with zero findings still says how many AC rows were sound rather than
/// reading like nothing ran.
#[must_use]
pub fn render(reports: &[FrReport<'_>]) -> String {
    let mut findings_rows = Vec::new();
    let mut next_id: u32 = 1;
    let mut sound_count = 0usize;
    let mut classifiers = std::collections::BTreeSet::new();

    for report in reports {
        classifiers.insert(report.verdict.classifier.clone());
        sound_count += report.verdict.sound.len();
        for finding in &report.verdict.findings {
            let summary = summary_line(finding);
            findings_rows.push(format!(
                "| FND-{next_id:03} | {} | {summary} | {} |",
                finding.severity.as_str(),
                finding.ac_id,
            ));
            next_id += 1;
        }
    }

    let total_ac = reports
        .iter()
        .map(|r| {
            r.verdict.findings.len()
                + r.verdict.sound.len()
                + r.verdict.unrecognized.len()
                + r.verdict.unanswered.len()
        })
        .sum::<usize>();
    let unrecognized_count: usize = reports.iter().map(|r| r.verdict.unrecognized.len()).sum();
    let unanswered_count: usize = reports.iter().map(|r| r.verdict.unanswered.len()).sum();
    let classifier_line = classifiers.into_iter().collect::<Vec<_>>().join(", ");
    let coverage_lines: Vec<String> = reports
        .iter()
        .filter_map(|r| {
            r.verdict
                .coverage
                .as_ref()
                .map(|coverage| coverage_summary_line(r.fr_id, coverage))
        })
        .collect();

    let mut unreviewed_tail = String::new();
    if unrecognized_count > 0 {
        let _ = write!(
            unreviewed_tail,
            ", {unrecognized_count} unrecognized label{}",
            if unrecognized_count == 1 { "" } else { "s" },
        );
    }
    if unanswered_count > 0 {
        let _ = write!(unreviewed_tail, ", {unanswered_count} unanswered");
    }

    let mut body = String::new();
    body.push_str("## Summary\n\n");
    let _ = write!(
        body,
        "Criterion-strength analysis over {} FR{}, {total_ac} acceptance-criteria row{} \
         reviewed: {} weak, {sound_count} sound{unreviewed_tail}. Classifier: {classifier_line}.\n\n",
        reports.len(),
        if reports.len() == 1 { "" } else { "s" },
        if total_ac == 1 { "" } else { "s" },
        findings_rows_that_are_weakness(reports),
    );
    if !coverage_lines.is_empty() {
        body.push_str("Adverse-case coverage (informational; not itself a finding):\n\n");
        for line in &coverage_lines {
            body.push_str("- ");
            body.push_str(line);
            body.push('\n');
        }
        body.push('\n');
    }
    body.push_str("## Findings\n\n");
    body.push_str("| ID | Severity | Summary | Refs |\n");
    body.push_str("| --- | --- | --- | --- |\n");
    if findings_rows.is_empty() {
        let _ = writeln!(
            body,
            "| FND-001 | low | No issues found; {sound_count} AC row{} reviewed sound | - |",
            if sound_count == 1 { "" } else { "s" },
        );
    } else {
        for row in &findings_rows {
            body.push_str(row);
            body.push('\n');
        }
    }
    body
}

fn findings_rows_that_are_weakness(reports: &[FrReport<'_>]) -> usize {
    reports.iter().map(|r| r.verdict.findings.len()).sum()
}

fn summary_line(finding: &crate::verdict::Finding) -> String {
    let base = format!(
        "{} classified {} (confidence {:.2})",
        finding.ac_id, finding.weakness_kind, finding.confidence
    );
    let base = annotate(base, finding.certainty);
    let base = format!(
        "{base} {}",
        sub_question_note(&finding.weakness_kind, &finding.label_sub_question)
    );
    // The five raw `noul` values are carried into the summary as supporting
    // evidence, explicitly labelled uncalibrated (see `verdict.rs`'s module
    // doc: there is no confidence field on a `noul` answer to threshold on,
    // so this is context for a reader, never a gate).
    if finding.noul.values.is_empty() {
        base
    } else {
        let noul = finding
            .noul
            .values
            .iter()
            .map(|(id, value)| format!("{id}={value:.2}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{base} [noul, uncalibrated: {noul}]")
    }
}

/// PLAT-984's "which sub-question decided this" for one finding, worded as
/// what it is: whether the label's own sub-question backs it. `weakness_kind`
/// is one Jev `choice` call, not a composite of the `noul` answers, so the
/// wording never says the sub-question decided the label (see
/// [`SubQuestionCheck`]).
fn sub_question_note(kind: &str, check: &SubQuestionCheck) -> String {
    match check {
        SubQuestionCheck::Agrees { question, noul } => {
            format!("[label's sub-question {question}={noul:.2} agrees]")
        }
        SubQuestionCheck::Disagrees { question, noul } => format!(
            "[label's sub-question {question}={noul:.2} DISAGREES: {kind} is not carried by it]"
        ),
        SubQuestionCheck::Unanswered { question } => {
            format!("[label's sub-question {question} unanswered]")
        }
        SubQuestionCheck::NoSubQuestion => format!("[no sub-question covers {kind}]"),
    }
}

fn coverage_summary_line(fr_id: &str, coverage: &crate::verdict::CoverageVerdict) -> String {
    let label = coverage.label.as_deref().unwrap_or("no rubric label");
    let base = format!(
        "{fr_id} adverse_case_coverage {:.1} ({label}, confidence {:.2})",
        coverage.score, coverage.confidence
    );
    annotate(base, coverage.certainty)
}

/// Appends the [`Certainty`] annotation to a summary line. One exhaustive
/// `match`, shared by findings and coverage, so a fourth bucket cannot be
/// rendered in one place and silently dropped in the other.
fn annotate(base: String, certainty: Certainty) -> String {
    match certainty {
        Certainty::Confident => base,
        Certainty::Unconfirmed => {
            format!("{base} -- unconfirmed, below the confidence threshold")
        }
        Certainty::Uncertain => {
            format!("{base} -- uncertain, top two answers within the margin")
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{FrReport, render};
    use crate::verdict::{
        Certainty, CoverageVerdict, Finding, FrVerdict, NoulSignals, Severity, SubQuestionCheck,
    };

    fn finding(certainty: Certainty) -> Finding {
        Finding {
            ac_id: "FR-001-AC-1".to_owned(),
            weakness_kind: "unfalsifiable".to_owned(),
            severity: Severity::High,
            confidence: if certainty == Certainty::Unconfirmed {
                0.3
            } else {
                0.9
            },
            certainty,
            probabilities: vec![("unfalsifiable".to_owned(), 0.9)],
            noul: NoulSignals { values: vec![] },
            label_sub_question: SubQuestionCheck::Unanswered {
                question: "falsifiable",
            },
        }
    }

    fn one_finding(finding: Finding) -> FrVerdict {
        FrVerdict {
            classifier: "jev-1.13.0".to_owned(),
            usage_input_tokens: 0,
            usage_output_tokens: 0,
            findings: vec![finding],
            sound: vec![],
            unrecognized: vec![],
            unanswered: vec![],
            coverage: None,
        }
    }

    fn one_finding_body(f: Finding) -> String {
        render(&[FrReport {
            fr_id: "FR-001",
            verdict: &one_finding(f),
        }])
    }

    /// Provenance: PLAT-984. A finding whose label its own sub-question
    /// contradicts says so in the rendered Findings row -- the exact line,
    /// so the wording cannot drift into claiming the sub-question decided it.
    #[test]
    fn a_disagreeing_sub_question_is_named_in_the_findings_row() {
        let mut f = finding(Certainty::Confident);
        f.label_sub_question = SubQuestionCheck::Disagrees {
            question: "falsifiable",
            noul: 0.9,
        };
        let body = one_finding_body(f);
        assert!(body.contains(
            "| FND-001 | high | FR-001-AC-1 classified unfalsifiable (confidence 0.90) \
             [label's sub-question falsifiable=0.90 DISAGREES: unfalsifiable is not carried by it] \
             | FR-001-AC-1 |\n"
        ));
    }

    /// Provenance: PLAT-984. An agreeing sub-question is rendered after the
    /// unconfirmed marker and before the raw `noul` list.
    #[test]
    fn an_agreeing_sub_question_is_rendered_between_the_marker_and_the_noul_list() {
        let mut f = finding(Certainty::Unconfirmed);
        f.label_sub_question = SubQuestionCheck::Agrees {
            question: "falsifiable",
            noul: 0.1,
        };
        f.noul = NoulSignals {
            values: vec![("falsifiable".to_owned(), 0.1)],
        };
        let body = one_finding_body(f);
        assert!(body.contains(
            "-- unconfirmed, below the confidence threshold \
             [label's sub-question falsifiable=0.10 agrees] \
             [noul, uncalibrated: falsifiable=0.10]"
        ));
    }

    /// Provenance: PLAT-984. The two outcomes that name no answer are
    /// rendered as themselves, never as agreement.
    #[test]
    fn an_unanswered_or_uncovered_sub_question_is_rendered_as_itself() {
        let body = one_finding_body(finding(Certainty::Confident));
        assert!(body.contains("[label's sub-question falsifiable unanswered]"));
        assert!(!body.contains("agrees"));

        let mut f = finding(Certainty::Confident);
        "happy_path_only".clone_into(&mut f.weakness_kind);
        f.label_sub_question = SubQuestionCheck::NoSubQuestion;
        let body = one_finding_body(f);
        assert!(body.contains("[no sub-question covers happy_path_only]"));
    }

    /// Provenance: PLAT-981. Each certainty bucket renders distinguishably:
    /// an `Uncertain` finding still appears as a row, and reads as
    /// "uncertain", never as "unconfirmed" -- the third bucket is visible to
    /// a reader of the review, not only to a JSON consumer.
    #[test]
    fn each_certainty_bucket_renders_its_own_annotation() {
        let render_one = |certainty| one_finding_body(finding(certainty));
        let confident = render_one(Certainty::Confident);
        assert!(!confident.contains("unconfirmed") && !confident.contains("uncertain"));

        let unconfirmed = render_one(Certainty::Unconfirmed);
        assert!(unconfirmed.contains("unconfirmed, below the confidence threshold"));
        assert!(!unconfirmed.contains("uncertain"));

        let uncertain = render_one(Certainty::Uncertain);
        assert!(uncertain.contains("FND-001"));
        assert!(uncertain.contains("uncertain, top two answers within the margin"));
        assert!(!uncertain.contains("unconfirmed"));
    }

    /// Provenance: PLAT-837. The Findings table header is exactly the
    /// contract `SpecReview` requires.
    #[test]
    fn the_findings_header_matches_the_spec_review_contract() {
        let verdict = FrVerdict {
            classifier: "jev-1.13.0".to_owned(),
            usage_input_tokens: 0,
            usage_output_tokens: 0,
            findings: vec![finding(Certainty::Confident)],
            sound: vec![],
            unrecognized: vec![],
            unanswered: vec![],
            coverage: None,
        };
        let body = render(&[FrReport {
            fr_id: "FR-001",
            verdict: &verdict,
        }]);
        assert!(body.contains("| ID | Severity | Summary | Refs |\n"));
        assert!(body.contains("| --- | --- | --- | --- |\n"));
    }

    /// Provenance: PLAT-837. This is the acceptance criterion itself: a
    /// low-confidence finding still appears as a row, marked unconfirmed --
    /// never dropped from the table.
    #[test]
    fn an_unconfirmed_finding_still_appears_as_a_row() {
        let verdict = FrVerdict {
            classifier: "jev-1.13.0".to_owned(),
            usage_input_tokens: 0,
            usage_output_tokens: 0,
            findings: vec![finding(Certainty::Unconfirmed)],
            sound: vec![],
            unrecognized: vec![],
            unanswered: vec![],
            coverage: None,
        };
        let body = render(&[FrReport {
            fr_id: "FR-001",
            verdict: &verdict,
        }]);
        assert!(body.contains("FND-001"));
        assert!(body.contains("unconfirmed"));
    }

    /// Provenance: PLAT-837. Every report names the classifier: the concrete
    /// model version appears in the rendered Summary.
    #[test]
    fn the_summary_names_the_classifier() {
        let verdict = FrVerdict {
            classifier: "jev-1.13.0".to_owned(),
            usage_input_tokens: 10,
            usage_output_tokens: 0,
            findings: vec![],
            sound: vec!["FR-001-AC-1".to_owned()],
            unrecognized: vec![],
            unanswered: vec![],
            coverage: None,
        };
        let body = render(&[FrReport {
            fr_id: "FR-001",
            verdict: &verdict,
        }]);
        assert!(body.contains("jev-1.13.0"));
        assert!(!body.contains("jev-latest"));
    }

    /// Provenance: PLAT-837. All-sound still emits a valid, non-empty
    /// Findings table row (the skeleton's own required shape for "nothing
    /// found"), not an empty table.
    #[test]
    fn all_sound_still_emits_a_findings_row() {
        let verdict = FrVerdict {
            classifier: "jev-1.13.0".to_owned(),
            usage_input_tokens: 0,
            usage_output_tokens: 0,
            findings: vec![],
            sound: vec!["FR-001-AC-1".to_owned(), "FR-001-AC-2".to_owned()],
            unrecognized: vec![],
            unanswered: vec![],
            coverage: None,
        };
        let body = render(&[FrReport {
            fr_id: "FR-001",
            verdict: &verdict,
        }]);
        assert!(body.contains("FND-001"));
        assert!(body.contains("No issues found"));
    }

    /// Provenance: PLAT-837. `adverse_case_coverage` is reported in the
    /// Summary as informational context -- it is not itself a Findings-table
    /// row, since the ticket ties `## Findings` rows to `weakness_kind`
    /// verdicts and this crate does not invent a threshold that would turn a
    /// low score into a fabricated finding.
    #[test]
    fn coverage_is_summarised_but_is_not_a_findings_row() {
        let verdict = FrVerdict {
            classifier: "jev-1.13.0".to_owned(),
            usage_input_tokens: 0,
            usage_output_tokens: 0,
            findings: vec![],
            sound: vec!["FR-001-AC-1".to_owned()],
            unrecognized: vec![],
            unanswered: vec![],
            coverage: Some(CoverageVerdict {
                score: 0.0,
                label: Some("happy path only".to_owned()),
                confidence: 0.9,
                certainty: Certainty::Confident,
            }),
        };
        let body = render(&[FrReport {
            fr_id: "FR-001",
            verdict: &verdict,
        }]);
        let (summary, findings) = body
            .split_once("## Findings")
            .expect("both sections render");
        assert!(summary.contains("adverse_case_coverage"));
        assert!(!findings.contains("adverse_case_coverage"));
    }

    /// Provenance: PLAT-837 review, finding 2. `unanswered` AC rows are
    /// surfaced in the rendered Summary, not silently absent from it --
    /// otherwise a reader has no way to tell "every AC row was reviewed"
    /// from "some rows were never reviewed at all".
    #[test]
    fn unanswered_and_unrecognized_rows_are_surfaced_in_the_summary() {
        let verdict = FrVerdict {
            classifier: "jev-1.13.0".to_owned(),
            usage_input_tokens: 0,
            usage_output_tokens: 0,
            findings: vec![],
            sound: vec!["FR-001-AC-1".to_owned()],
            unrecognized: vec![("FR-001-AC-2".to_owned(), "quantum_uncertainty".to_owned())],
            unanswered: vec!["FR-001-AC-3".to_owned()],
            coverage: None,
        };
        let body = render(&[FrReport {
            fr_id: "FR-001",
            verdict: &verdict,
        }]);
        let (summary, _) = body
            .split_once("## Findings")
            .expect("both sections render");
        assert!(summary.contains("1 unrecognized label"));
        assert!(summary.contains("1 unanswered"));
        assert!(summary.contains("3 acceptance-criteria rows"));
    }

    /// Provenance: PLAT-837 review, finding 6a. A finding's rendered summary
    /// carries the raw `noul` values, explicitly labelled uncalibrated --
    /// the PR's own stated claim, proven rather than merely asserted in
    /// prose.
    #[test]
    fn a_findings_summary_carries_its_noul_values_labelled_uncalibrated() {
        let mut f = finding(Certainty::Confident);
        f.noul = NoulSignals {
            values: vec![
                ("falsifiable".to_owned(), 0.9),
                ("threshold_present".to_owned(), 0.1),
            ],
        };
        let verdict = FrVerdict {
            classifier: "jev-1.13.0".to_owned(),
            usage_input_tokens: 0,
            usage_output_tokens: 0,
            findings: vec![f],
            sound: vec![],
            unrecognized: vec![],
            unanswered: vec![],
            coverage: None,
        };
        let body = render(&[FrReport {
            fr_id: "FR-001",
            verdict: &verdict,
        }]);
        assert!(body.contains("uncalibrated"));
        assert!(body.contains("falsifiable=0.90"));
    }
}
