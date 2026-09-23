// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Grading a run and rendering the report (PLAT-1027).
//!
//! Agreement, the best constant predictor, defect / no-defect recall,
//! per-class recall and ECE come from `../support/grading.rs`, the maths
//! PLAT-917, PLAT-838 and PLAT-1014 already share: this module only builds
//! its [`Graded`] rows. Two measures are new here:
//!
//! - **Ordering quality** for an ordinal key (severity): over every pair of
//!   rows whose true levels differ, whether the predicted ordinal puts them in
//!   the same order. Reported as the concordance index `(C + T/2) / pairs`
//!   (0.5 = no better than a constant or a coin), with Goodman-Kruskal gamma
//!   `(C - D) / (C + D)` and the raw counts beside it. Severity orders the
//!   findings in a report, so a variant can be useful for ordering while
//!   missing the exact bucket, and this is how that shows.
//! - **The coverage/accuracy curve**: at each confidence threshold, the share
//!   of rows the variant answers with at least that confidence, and its
//!   accuracy on them.
//!
//! Every table is broken down by mode and by truth kind. A slice containing
//! agent-labelled truth says so in its name; the agent slice is named
//! `AGENT-LABELLED`.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use super::corpus::{KindGroup, Row, TruthAnswer, TruthKind};
use super::grading::{
    Graded, Tier, Verdict, class_stats, defect_recall, expected_calibration_error,
    no_defect_recall, percent, tally, trivial_baseline,
};
use super::keys::{KEYS, KeySpec, Mode, spec};
use super::variant::{Prediction, RunOutput, Variant};

/// One (row, key) pair a variant is graded on.
#[derive(Debug, Clone)]
pub(crate) struct Scored {
    /// The row.
    pub(crate) row_id: String,
    /// Its mode.
    pub(crate) mode: Mode,
    /// How its truth was established.
    pub(crate) kind: TruthKind,
    /// The primary truth label.
    pub(crate) expected: String,
    /// Other defensible labels.
    pub(crate) alternatives: Vec<String>,
    /// What the variant said, `None` when it gave no answer for this key.
    pub(crate) prediction: Option<Prediction>,
}

/// Every graded (row, `key`) pair for one variant in `output`: each row that
/// carries truth for `key` and that the variant ran on. A row it ran on but
/// left without an answer is kept, as unanswered.
pub(crate) fn scored(rows: &[Row], output: &RunOutput, variant: &str, key: &str) -> Vec<Scored> {
    let results: BTreeMap<&str, _> = output
        .results
        .iter()
        .filter(|result| result.variant == variant)
        .map(|result| (result.row_id.as_str(), &result.predictions))
        .collect();
    rows.iter()
        .filter_map(|row| {
            let truth = row.truth.get(key)?;
            let predictions = results.get(row.id.as_str())?;
            Some(Scored {
                row_id: row.id.clone(),
                mode: row.mode,
                kind: truth.kind,
                expected: truth.answer.label(),
                alternatives: truth.alternatives.iter().map(TruthAnswer::label).collect(),
                prediction: predictions.get(key).cloned(),
            })
        })
        .collect()
}

/// `rows` as the shared grader's rows, against `spec`'s answer space.
pub(crate) fn graded(rows: &[Scored], spec: &KeySpec) -> Vec<Graded> {
    rows.iter()
        .map(|row| {
            let actual = row.prediction.as_ref().map(|p| p.answer.clone());
            let verdict = match actual.as_deref() {
                None => Verdict::Unanswered,
                Some(label) if !spec.space.contains(&label) => Verdict::Unrecognized,
                Some(label) if label == row.expected => Verdict::Primary,
                Some(label) if row.alternatives.iter().any(|alt| alt == label) => {
                    Verdict::Contested
                }
                Some(_) => Verdict::Wrong,
            };
            let mut readings = vec![row.expected.clone()];
            readings.extend(row.alternatives.iter().cloned());
            let shown = actual.unwrap_or_else(|| "<unanswered>".to_owned());
            Graded {
                fixture_id: row.row_id.clone(),
                tier: if row.alternatives.is_empty() {
                    Tier::Clean
                } else {
                    Tier::Ambiguous
                },
                expected: row.expected.clone(),
                contested: readings,
                actual: shown.clone(),
                actual_class: shown,
                verdict,
                confidence: row.prediction.as_ref().and_then(|p| p.confidence),
            }
        })
        .collect()
}

/// Pairwise ordering counts for an ordinal key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Concordance {
    /// Pairs the prediction orders as the truth does.
    pub(crate) concordant: usize,
    /// Pairs it orders the other way.
    pub(crate) discordant: usize,
    /// Pairs it scores equal although the truth differs.
    pub(crate) tied: usize,
    /// Rows left out for having no ordinal (unanswered).
    pub(crate) excluded: usize,
}

impl Concordance {
    /// Every compared pair.
    pub(crate) const fn pairs(&self) -> usize {
        self.concordant + self.discordant + self.tied
    }

    /// `(C + T/2) / pairs`, or `None` with no pair to compare.
    pub(crate) fn index(&self) -> Option<f64> {
        let pairs = self.pairs();
        (pairs > 0)
            .then(|| (percent(self.concordant, pairs) + percent(self.tied, pairs) / 2.0) / 100.0)
    }

    /// Goodman-Kruskal gamma, `(C - D) / (C + D)`, or `None` when every pair
    /// is tied.
    pub(crate) fn gamma(&self) -> Option<f64> {
        let decided = self.concordant + self.discordant;
        (decided > 0).then(|| {
            (percent(self.concordant, decided) - percent(self.discordant, decided)) / 100.0
        })
    }
}

/// Ordering quality of the predicted ordinal against the true level (by
/// position in `spec.space`, lowest first). `None` for a non-ordinal key.
pub(crate) fn concordance(rows: &[Scored], spec: &KeySpec) -> Option<Concordance> {
    if !spec.ordinal {
        return None;
    }
    let mut counts = Concordance::default();
    let mut points: Vec<(usize, f64)> = Vec::new();
    for row in rows {
        let level = spec.space.iter().position(|label| *label == row.expected);
        match (level, row.prediction.as_ref().and_then(|p| p.ordinal)) {
            (Some(level), Some(ordinal)) => points.push((level, ordinal)),
            _ => counts.excluded += 1,
        }
    }
    for (index, (level_a, score_a)) in points.iter().enumerate() {
        for (level_b, score_b) in points.iter().skip(index + 1) {
            if level_a == level_b {
                continue;
            }
            let truth_up = level_b > level_a;
            if (score_b - score_a).abs() < 1e-12 {
                counts.tied += 1;
            } else if (score_b > score_a) == truth_up {
                counts.concordant += 1;
            } else {
                counts.discordant += 1;
            }
        }
    }
    Some(counts)
}

/// One point of the coverage/accuracy curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CurvePoint {
    /// Rows answered with at least this confidence are kept.
    pub(crate) threshold: f64,
    /// Rows kept.
    pub(crate) answered: usize,
    /// Of those, rows agreeing with a recorded reading.
    pub(crate) correct: usize,
    /// Every row in the slice, confident or not, answered or not.
    pub(crate) total: usize,
}

impl CurvePoint {
    /// `answered / total`, as a percentage.
    pub(crate) fn coverage(&self) -> Option<f64> {
        (self.total > 0).then(|| percent(self.answered, self.total))
    }

    /// `correct / answered`, as a percentage.
    pub(crate) fn accuracy(&self) -> Option<f64> {
        (self.answered > 0).then(|| percent(self.correct, self.answered))
    }
}

/// The thresholds every report's curve is drawn at.
pub(crate) const THRESHOLDS: [f64; 6] = [0.5, 0.6, 0.7, 0.8, 0.9, 0.95];

/// The coverage/accuracy curve at `thresholds`. A row with no confidence is
/// never kept, at any threshold.
pub(crate) fn coverage_curve(graded: &[Graded], thresholds: &[f64]) -> Vec<CurvePoint> {
    thresholds
        .iter()
        .map(|threshold| {
            let kept = graded
                .iter()
                .filter(|row| row.confidence.is_some_and(|c| c >= *threshold));
            let (answered, correct) = kept.fold((0, 0), |(answered, correct), row| {
                (answered + 1, correct + usize::from(row.verdict.agrees()))
            });
            CurvePoint {
                threshold: *threshold,
                answered,
                correct,
                total: graded.len(),
            }
        })
        .collect()
}

/// The headline numbers for one slice.
#[derive(Debug, Clone)]
pub(crate) struct Summary {
    /// Rows in the slice.
    pub(crate) rows: usize,
    /// Agreement, primary plus contested, over every row, as a percentage.
    pub(crate) agreement: Option<f64>,
    /// The best constant predictor's label.
    pub(crate) baseline_label: String,
    /// Its agreement, as a percentage.
    pub(crate) baseline: f64,
    /// Of defect rows, the share flagged as some defect.
    pub(crate) defect_recall: Option<f64>,
    /// Of no-defect rows, the share cleared.
    pub(crate) no_defect_recall: Option<f64>,
    /// Expected calibration error over the stated confidences.
    pub(crate) ece: Option<f64>,
    /// Ordering quality, for an ordinal key.
    pub(crate) concordance: Option<Concordance>,
}

impl Summary {
    /// Agreement minus the constant predictor, in percentage points.
    pub(crate) fn margin(&self) -> Option<f64> {
        self.agreement.map(|agreement| agreement - self.baseline)
    }
}

/// The headline numbers for `rows` under `spec`.
pub(crate) fn summarize(rows: &[Scored], spec: &KeySpec) -> Summary {
    let graded = graded(rows, spec);
    let (baseline_label, baseline) = trivial_baseline(&graded);
    Summary {
        rows: rows.len(),
        agreement: tally(&graded).agreement(),
        baseline_label,
        baseline,
        defect_recall: defect_recall(&graded, spec.no_defect),
        no_defect_recall: no_defect_recall(&graded, spec.no_defect),
        ece: expected_calibration_error(&graded),
        concordance: concordance(rows, spec),
    }
}

/// A slice's name, saying how much of it is agent-labelled.
pub(crate) fn slice_name(base: &str, rows: &[Scored]) -> String {
    let agent = rows
        .iter()
        .filter(|row| row.kind.group() == KindGroup::Agent)
        .count();
    if agent == 0 || base == KindGroup::Agent.label() {
        base.to_owned()
    } else if agent == rows.len() {
        format!("{base} [AGENT-LABELLED]")
    } else {
        format!("{base} (incl. {agent} AGENT-LABELLED)")
    }
}

fn show_percent(value: Option<f64>) -> String {
    value.map_or_else(|| "n/a".to_owned(), |value| format!("{value:.1}%"))
}

fn show_ratio(value: Option<f64>) -> String {
    value.map_or_else(|| "n/a".to_owned(), |value| format!("{value:.3}"))
}

fn summary_row(name: &str, summary: &Summary) -> String {
    let ordering = summary.concordance.map_or_else(
        || "-".to_owned(),
        |c| {
            format!(
                "{} (gamma {}; C={} D={} T={}, {} excluded)",
                show_ratio(c.index()),
                show_ratio(c.gamma()),
                c.concordant,
                c.discordant,
                c.tied,
                c.excluded
            )
        },
    );
    format!(
        "| {name} | {} | {} | `{}` {:.1}% | {} | {} | {} | {} | {ordering} |",
        summary.rows,
        show_percent(summary.agreement),
        summary.baseline_label,
        summary.baseline,
        summary
            .margin()
            .map_or_else(|| "n/a".to_owned(), |m| format!("{m:+.1}pp")),
        show_percent(summary.defect_recall),
        show_percent(summary.no_defect_recall),
        show_ratio(summary.ece),
    )
}

/// The full report for one variant on one key: every slice, per-class
/// recall and the coverage/accuracy curve over all rows.
pub(crate) fn render(variant: &str, key: &str, rows: &[Scored]) -> String {
    let Some(spec) = spec(key) else {
        return format!("\n### {variant} — {key}: unknown key\n");
    };
    let mut out = String::new();
    let _ = writeln!(out, "\n### {variant} — `{key}`\n");
    if rows.is_empty() {
        let _ = writeln!(out, "No row carries truth for this key in this run.");
        return out;
    }
    let _ = writeln!(
        out,
        "| Slice | Rows | Agreement | Constant predictor | Margin | Defect recall | \
         No-defect recall | ECE | Ordering (concordance) |"
    );
    let _ = writeln!(
        out,
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    );
    let _ = writeln!(
        out,
        "{}",
        summary_row(&slice_name("all", rows), &summarize(rows, spec))
    );
    for mode in Mode::ALL {
        let slice: Vec<Scored> = rows
            .iter()
            .filter(|row| row.mode == mode)
            .cloned()
            .collect();
        if !slice.is_empty() {
            let name = slice_name(&format!("mode {}", mode.as_str()), &slice);
            let _ = writeln!(out, "{}", summary_row(&name, &summarize(&slice, spec)));
        }
    }
    for group in KindGroup::ALL {
        let slice: Vec<Scored> = rows
            .iter()
            .filter(|row| row.kind.group() == group)
            .cloned()
            .collect();
        if !slice.is_empty() {
            let _ = writeln!(
                out,
                "{}",
                summary_row(&slice_name(group.label(), &slice), &summarize(&slice, spec))
            );
        }
    }

    let all = graded(rows, spec);
    let _ = writeln!(
        out,
        "\nPer class, {} (primary reading only):\n",
        slice_name("all rows", rows)
    );
    let _ = writeln!(out, "| Label | Expected | Predicted | Recall | Precision |");
    let _ = writeln!(out, "| --- | --- | --- | --- | --- |");
    for (label, stats) in class_stats(&all) {
        let _ = writeln!(
            out,
            "| {label} | {} | {} | {} | {} |",
            stats.expected,
            stats.predicted,
            show_percent(stats.recall()),
            show_percent(stats.precision()),
        );
    }
    let _ = writeln!(
        out,
        "\nCoverage / accuracy by confidence, {}:\n",
        slice_name("all rows", rows)
    );
    let _ = writeln!(out, "| Confidence ≥ | Answered | Coverage | Accuracy |");
    let _ = writeln!(out, "| --- | --- | --- | --- |");
    for point in coverage_curve(&all, &THRESHOLDS) {
        let _ = writeln!(
            out,
            "| {:.2} | {}/{} | {} | {} |",
            point.threshold,
            point.answered,
            point.total,
            show_percent(point.coverage()),
            show_percent(point.accuracy()),
        );
    }
    out
}

/// The report for a whole run: every variant, every key it grades that some
/// row carries truth for, in [`KEYS`] order.
pub(crate) fn render_run(rows: &[Row], output: &RunOutput, variants: &[&Variant]) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "## Jev corpus-v2 run: {} requests sent, {} reused; models {:?}",
        output.requests_sent, output.requests_reused, output.models
    );
    let _ = writeln!(
        out,
        "\nRows labelled AGENT-LABELLED were graded against agent-written truth, \
         not mechanical or by-construction ground truth."
    );
    for variant in variants {
        let label = variant.label();
        let _ = writeln!(out, "\n## {label}: {}", variant.summary);
        for spec in KEYS
            .iter()
            .filter(|spec| variant.grades.contains(&spec.key))
        {
            let _ = write!(
                out,
                "{}",
                render(&label, spec.key, &scored(rows, output, &label, spec.key))
            );
        }
    }
    out
}
