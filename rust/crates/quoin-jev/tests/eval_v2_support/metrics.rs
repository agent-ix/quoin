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

use super::corpus::{Excluded, KindGroup, Row, TruthAnswer, TruthKind};
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

/// The most a variant may leave unanswered and still pass an ordering bar,
/// in percent of its rows (MP-240 Bar C).
pub(crate) const MAX_ABSTENTION_PERCENT: f64 = 10.0;

/// A variant's ordering against a baseline's on the rows BOTH answered
/// (PR #620 review). [`concordance`] drops each run's unanswered rows
/// separately, so a variant that abstains on hard rows is compared over an
/// easier set; this compares the two over one set, and counts what each
/// left out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct PairedConcordance {
    /// The variant's pairs, over the shared rows.
    pub(crate) variant: Concordance,
    /// The baseline's pairs, over the same rows.
    pub(crate) baseline: Concordance,
    /// Rows both answered with an ordinal.
    pub(crate) shared: usize,
    /// The variant's rows.
    pub(crate) variant_rows: usize,
    /// The variant's rows with no ordinal.
    pub(crate) variant_abstained: usize,
    /// The baseline's rows with no ordinal.
    pub(crate) baseline_abstained: usize,
}

impl PairedConcordance {
    /// The variant's abstention, as a percentage of its rows.
    pub(crate) fn variant_abstention(&self) -> Option<f64> {
        (self.variant_rows > 0).then(|| percent(self.variant_abstained, self.variant_rows))
    }

    /// Bar C: at most [`MAX_ABSTENTION_PERCENT`] of the variant's rows
    /// unanswered, and a paired index strictly above the baseline's. `None`
    /// when there is no pair to compare.
    pub(crate) fn beats_baseline(&self) -> Option<bool> {
        let (variant, baseline) = (self.variant.index()?, self.baseline.index()?);
        let abstention = self.variant_abstention()?;
        Some(abstention <= MAX_ABSTENTION_PERCENT && variant > baseline)
    }
}

/// [`PairedConcordance`] of `variant` against `baseline` on `spec`, matching
/// rows by id. `None` for a non-ordinal key.
pub(crate) fn paired_concordance(
    variant: &[Scored],
    baseline: &[Scored],
    spec: &KeySpec,
) -> Option<PairedConcordance> {
    if !spec.ordinal {
        return None;
    }
    let answered = |row: &Scored| row.prediction.as_ref().and_then(|p| p.ordinal).is_some();
    let baseline_answered: BTreeMap<&str, &Scored> = baseline
        .iter()
        .filter(|row| answered(row))
        .map(|row| (row.row_id.as_str(), row))
        .collect();
    let (mut ours, mut theirs) = (Vec::new(), Vec::new());
    for row in variant.iter().filter(|row| answered(row)) {
        if let Some(other) = baseline_answered.get(row.row_id.as_str()) {
            ours.push(row.clone());
            theirs.push((*other).clone());
        }
    }
    Some(PairedConcordance {
        variant: concordance(&ours, spec)?,
        baseline: concordance(&theirs, spec)?,
        shared: ours.len(),
        variant_rows: variant.len(),
        variant_abstained: variant.iter().filter(|row| !answered(row)).count(),
        baseline_abstained: baseline.iter().filter(|row| !answered(row)).count(),
    })
}

/// For an ordinal key, the baseline variant id its ordering bar compares
/// against (MP-240 Bar C).
pub(crate) const ORDINAL_BASELINES: &[(&str, &str)] = &[("severity", "S0")];

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
    /// Rows with no stated confidence, which ECE and the curve leave out.
    pub(crate) without_confidence: usize,
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
        without_confidence: graded.iter().filter(|row| row.confidence.is_none()).count(),
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
        "| {name} | {} | {} | `{}` {:.1}% | {} | {} | {} | {} ({} w/o confidence) | {ordering} |",
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
        summary.without_confidence,
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

// ---------------------------------------------------------------------------
// Paired contrast (PLAT-1029 bar D; the rule PLAT-1028..1031 share)
// ---------------------------------------------------------------------------

/// Where a mutant row names its unmutated source row. PLAT-1025 adds it to
/// the corpus schema; until then callers pass a pairing function.
pub(crate) const SOURCE_ID_FIELD: &str = "mutation.source_id";

/// The one-sided sign test's significance level.
pub(crate) const SIGN_TEST_ALPHA: f64 = 0.05;

/// The fewest non-tie pairs a paired contrast is gateable at.
pub(crate) const MIN_DECIDED_PAIRS: usize = 10;

/// Which way a mutation should move a variant's answer on its key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    /// The mutation injects a defect that raises the answer: a probability
    /// of `yes`, or a severity level.
    Up,
    /// The mutation should lower the answer.
    Down,
}

/// What a paired contrast measures: `variant`'s answer (the prediction's
/// ordinal) on `key`, over mutants whose kind is in `kinds`.
///
/// `tau` and `delta` are in the ordinal's own units. For a probability key
/// the ordinal is `P(yes)`: `tau = 0.5`, `delta = 0.10`. For `severity` it is
/// the raw score in rubric levels (`none` 0 .. `high` 3): `tau = 2.0` makes
/// "the mutant is at least `medium`" the crossing, and `delta = 0.5` is half
/// a level.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ContrastSpec<'a> {
    /// The variant label, e.g. `E1@v1`.
    pub(crate) variant: &'a str,
    /// The modes the variant runs on ([`Variant::modes`]). Only a mutant row
    /// in one of these is paired, and its source must be in one too.
    pub(crate) modes: &'a [Mode],
    /// The question key.
    pub(crate) key: &'a str,
    /// The mutation kinds paired.
    pub(crate) kinds: &'a [&'a str],
    /// The expected direction.
    pub(crate) direction: Direction,
    /// The pair must cross this in the expected direction: source `< tau`
    /// and mutant `>= tau` for [`Direction::Up`], the reverse for
    /// [`Direction::Down`].
    pub(crate) tau: f64,
    /// The smallest move that is not a tie.
    pub(crate) delta: f64,
    /// The truth labels on the defect side: a pair whose source's primary
    /// truth on `key` is one of these is excluded, since the mutation
    /// injected nothing new there. `["yes"]` for an Up probability key,
    /// `["medium", "high"]` for severity.
    pub(crate) defect_side: &'a [&'a str],
}

/// How one pair came out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PairVerdict {
    /// The pair crossed tau in the expected direction and moved at least
    /// delta.
    Success,
    /// It moved at least delta the other way, or either row went unanswered
    /// (an abstention never helps a variant).
    Failure,
    /// Anything else. Excluded from the test.
    Tie,
}

/// One mutant/source pair.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PairOutcome {
    /// The mutant row.
    pub(crate) mutant: String,
    /// Its unmutated source row.
    pub(crate) source: String,
    /// Mutant ordinal minus source ordinal, in the ordinal's units, signed so
    /// positive is the expected direction; `None` when either side has none.
    pub(crate) shift: Option<f64>,
    /// The verdict.
    pub(crate) verdict: PairVerdict,
}

/// A paired contrast over every eligible mutant/source pair.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PairedContrast {
    /// Every eligible pair, in row order.
    pub(crate) pairs: Vec<PairOutcome>,
    /// Mutants left out because their source's truth is already on the
    /// defect side ([`ContrastSpec::defect_side`]).
    pub(crate) excluded_source_on_defect_side: usize,
}

impl PairedContrast {
    fn count(&self, verdict: PairVerdict) -> usize {
        self.pairs.iter().filter(|p| p.verdict == verdict).count()
    }

    /// Pairs that succeeded.
    pub(crate) fn successes(&self) -> usize {
        self.count(PairVerdict::Success)
    }

    /// Pairs that failed.
    pub(crate) fn failures(&self) -> usize {
        self.count(PairVerdict::Failure)
    }

    /// Ties, excluded from the test.
    pub(crate) fn ties(&self) -> usize {
        self.count(PairVerdict::Tie)
    }

    /// Failures that are abstentions (no probability on one side), reported
    /// apart.
    pub(crate) fn unanswered(&self) -> usize {
        self.pairs.iter().filter(|p| p.shift.is_none()).count()
    }

    /// The non-tie pairs the sign test runs on.
    pub(crate) fn decided(&self) -> usize {
        self.successes() + self.failures()
    }

    /// The one-sided sign test's p-value: `P(X >= successes)` for
    /// `X ~ Binomial(decided, 1/2)`. `None` with no decided pair.
    pub(crate) fn p_value(&self) -> Option<f64> {
        sign_test_p(self.successes(), self.decided())
    }

    /// Whether there are enough decided pairs to gate on.
    pub(crate) fn gateable(&self) -> bool {
        self.decided() >= MIN_DECIDED_PAIRS
    }

    /// Gateable, and the sign test rejects "no effect" at
    /// [`SIGN_TEST_ALPHA`].
    pub(crate) fn passes(&self) -> bool {
        self.gateable() && self.p_value().is_some_and(|p| p <= SIGN_TEST_ALPHA)
    }
}

/// `P(X >= successes)` for `X ~ Binomial(n, 1/2)`, summed over the pmf in
/// log space, so a large `n` does not underflow `0.5^n` to zero; `None` when
/// `n` is zero or does not fit in `u32`.
pub(crate) fn sign_test_p(successes: usize, n: usize) -> Option<f64> {
    let n = u32::try_from(n).ok().filter(|n| *n > 0)?;
    let Ok(successes) = u32::try_from(successes) else {
        return Some(0.0);
    };
    if successes > n {
        return Some(0.0);
    }
    // ln pmf(k), walked down from k = n, where pmf(n) = 2^-n; the terms are
    // summed with a running log-sum-exp.
    let mut log_pmf = -f64::from(n) * std::f64::consts::LN_2;
    let (mut peak, mut scaled) = (f64::NEG_INFINITY, 0.0f64);
    for k in (successes..=n).rev() {
        if log_pmf > peak {
            scaled = scaled * (peak - log_pmf).exp() + 1.0;
            peak = log_pmf;
        } else {
            scaled += (log_pmf - peak).exp();
        }
        // pmf(k - 1) = pmf(k) * k / (n - k + 1).
        log_pmf += f64::from(k).ln() - (f64::from(n - k) + 1.0).ln();
    }
    Some((peak + scaled.ln()).exp().min(1.0))
}

/// How many artifacts a mode carries, then the mode itself: RTC is richest,
/// then RC, then RT.
fn richness(mode: Mode) -> (usize, Mode) {
    (
        usize::from(mode.has_test()) + usize::from(mode.has_code()),
        mode,
    )
}

/// Every eligible pair for `spec`, and its verdict.
///
/// One pair per mutation id: among the mutation's rows in the variant's
/// modes (`spec.modes`), the richest is used: RTC, then RC, then RT. A row
/// in a mode the variant does not run on is never paired, so a variant is
/// never scored on a row it could not have answered. Its source is the row
/// `source_of` names (the [`SOURCE_ID_FIELD`] reading). A pair whose
/// source's primary truth on `spec.key` is already on the defect side is
/// left out and counted.
///
/// A pair succeeds when it crosses `tau` in the expected direction and moved
/// at least `delta` that way; it fails when it moved at least `delta` the
/// other way, or when either row has no answer (an abstention never helps a
/// variant). Anything else is a tie, excluded from the sign test.
///
/// # Errors
/// When a listed mutant names no source, or a source that is not among
/// `rows`, is itself a mutant, sits in another split, or is in a mode the
/// variant does not run on: a pairing the corpus does not support is
/// refused, never skipped. Also when either row's ordinal is not a finite
/// number, which would otherwise read as a silent tie.
pub(crate) fn paired_contrast(
    rows: &[Row],
    output: &RunOutput,
    spec: &ContrastSpec<'_>,
    source_of: &dyn Fn(&Row) -> Option<String>,
) -> Result<PairedContrast, String> {
    let ordinals: BTreeMap<&str, Option<f64>> = output
        .results
        .iter()
        .filter(|result| result.variant == spec.variant)
        .map(|result| {
            (
                result.row_id.as_str(),
                result.predictions.get(spec.key).and_then(|p| p.ordinal),
            )
        })
        .collect();
    let by_id: BTreeMap<&str, &Row> = rows.iter().map(|row| (row.id.as_str(), row)).collect();
    let mut chosen: BTreeMap<&str, &Row> = BTreeMap::new();
    for row in rows {
        let Some(mutation) = row
            .mutation
            .as_ref()
            .filter(|mutation| spec.kinds.contains(&mutation.kind.as_str()))
            .filter(|_| spec.modes.contains(&row.mode))
        else {
            continue;
        };
        let slot = chosen.entry(mutation.id.as_str()).or_insert(row);
        if richness(row.mode) > richness(slot.mode) {
            *slot = row;
        }
    }
    let chosen_ids: std::collections::BTreeSet<&str> =
        chosen.values().map(|row| row.id.as_str()).collect();
    let mut contrast = PairedContrast {
        pairs: Vec::new(),
        excluded_source_on_defect_side: 0,
    };
    // Row order, not mutation-id order, so a report reads like the corpus.
    for row in rows
        .iter()
        .filter(|row| chosen_ids.contains(row.id.as_str()))
    {
        let kind = row.mutation.as_ref().map_or("", |m| m.kind.as_str());
        let source_id = source_of(row)
            .ok_or_else(|| format!("{}: {kind} mutant names no {SOURCE_ID_FIELD}", row.id))?;
        let source = by_id.get(source_id.as_str()).ok_or_else(|| {
            format!(
                "{}: source {source_id} is not among the rows loaded",
                row.id
            )
        })?;
        if source.mutation.is_some() {
            return Err(format!("{}: source {source_id} is itself a mutant", row.id));
        }
        if source.split != row.split {
            return Err(format!(
                "{}: source {source_id} is in split {}, the mutant in {}",
                row.id,
                source.split.as_str(),
                row.split.as_str()
            ));
        }
        if !spec.modes.contains(&source.mode) {
            return Err(format!(
                "{}: source {source_id} is in mode {}, which {} does not run on",
                row.id,
                source.mode.as_str(),
                spec.variant
            ));
        }
        if source
            .truth
            .get(spec.key)
            .is_some_and(|truth| spec.defect_side.contains(&truth.answer.label().as_str()))
        {
            contrast.excluded_source_on_defect_side += 1;
            continue;
        }
        let ordinal_of = |id: &str| -> Result<Option<f64>, String> {
            match ordinals.get(id).copied().flatten() {
                Some(value) if !value.is_finite() => Err(format!(
                    "{id}: {} ordinal on {} is {value}, not a finite number",
                    spec.variant, spec.key
                )),
                value => Ok(value),
            }
        };
        let mutant_p = ordinal_of(&row.id)?;
        let source_p = ordinal_of(&source_id)?;
        let (shift, verdict) = match mutant_p.zip(source_p) {
            None => (None, PairVerdict::Failure),
            Some((mutant, source)) => {
                let (shift, verdict) = pair_verdict(spec, mutant, source);
                (Some(shift), verdict)
            }
        };
        contrast.pairs.push(PairOutcome {
            mutant: row.id.clone(),
            source: source_id,
            shift,
            verdict,
        });
    }
    Ok(contrast)
}

/// One answered pair's shift (signed so positive is `spec.direction`) and
/// verdict.
fn pair_verdict(spec: &ContrastSpec<'_>, mutant: f64, source: f64) -> (f64, PairVerdict) {
    let tau = spec.tau;
    let (shift, crossed) = match spec.direction {
        Direction::Up => (mutant - source, source < tau && mutant >= tau),
        Direction::Down => (source - mutant, source >= tau && mutant < tau),
    };
    // 1e-9 absorbs float error: 0.6 - 0.5 is 0.0999... in f64.
    let delta = spec.delta - 1e-9;
    let verdict = if shift >= delta && crossed {
        PairVerdict::Success
    } else if -shift >= delta {
        PairVerdict::Failure
    } else {
        PairVerdict::Tie
    };
    (shift, verdict)
}

/// The report for a whole run: every variant, every key it grades that some
/// row carries truth for, in [`KEYS`] order.
/// `excluded` is every row a load left out; they are listed first, so a
/// report can never be read without its exclusions.
pub(crate) fn render_run(
    rows: &[Row],
    excluded: &[Excluded],
    output: &RunOutput,
    variants: &[&Variant],
) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "## Jev corpus-v2 run: {} requests sent, {} reused; models {:?}",
        output.requests_sent, output.requests_reused, output.models
    );
    let _ = writeln!(
        out,
        "\n{} row(s) run; {} row(s) EXCLUDED at load:",
        rows.len(),
        excluded.len()
    );
    for row in excluded {
        let _ = writeln!(out, "- EXCLUDED {}: {}", row.id, row.reason);
    }
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
            let ours = scored(rows, output, &label, spec.key);
            let _ = write!(out, "{}", render(&label, spec.key, &ours));
            let baseline = ORDINAL_BASELINES
                .iter()
                .filter(|(key, id)| *key == spec.key && *id != variant.id)
                .find_map(|(_, id)| variants.iter().find(|other| other.id == *id));
            if let Some(baseline) = baseline {
                let theirs = scored(rows, output, &baseline.label(), spec.key);
                let _ = write!(
                    out,
                    "{}",
                    render_paired(&label, &baseline.label(), &ours, &theirs, spec)
                );
            }
        }
    }
    out
}

/// The paired ordering line: both indices over the rows both answered, and
/// each side's abstentions.
fn render_paired(
    label: &str,
    baseline: &str,
    ours: &[Scored],
    theirs: &[Scored],
    spec: &KeySpec,
) -> String {
    let Some(paired) = paired_concordance(ours, theirs, spec) else {
        return String::new();
    };
    let verdict = match paired.beats_baseline() {
        Some(true) => "above",
        Some(false) => "NOT above (or abstains on more than 10%)",
        None => "no shared pair",
    };
    format!(
        "\nPaired ordering vs `{baseline}` over {} row(s) both answered: {label} {} vs {} \
         ({verdict}); unanswered: {label} {}/{} ({}), {baseline} {}/{}.\n",
        paired.shared,
        show_ratio(paired.variant.index()),
        show_ratio(paired.baseline.index()),
        paired.variant_abstained,
        paired.variant_rows,
        show_percent(paired.variant_abstention()),
        paired.baseline_abstained,
        theirs.len(),
    )
}
