// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Lens-agnostic grading maths, factored out of the PLAT-917
//! criterion-strength grader so the PLAT-838 EARS grader (`support::ears`)
//! reuses it rather than re-deriving the same arithmetic (AP-202: "reuse,
//! don't rebuild").
//!
//! Every function here operates on [`Graded`] rows alone -- a fixture id, a
//! tier, an expected label plus every contested alternative, what the lens
//! actually returned, and an optional confidence. Nothing in this module
//! knows what `weakness_kind` or `ears_pattern_actual` mean; a caller builds
//! [`Graded`] rows from its own fixture type (see `support::mod`'s
//! `grade_weakness`/`grade_coverage` and `support::ears::grade`) and everything
//! below -- tally, per-class precision/recall, calibration, the constant-
//! predictor baseline, defect recall, and the M1 disagreement rate -- is
//! shared.

#![allow(
    dead_code,
    reason = "each test binary links this module separately, so items only one binary uses read as dead in the other"
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde::Deserialize;

/// How sure the corpus is of its own label.
///
/// `ambiguous` is not a defect in the fixture: a corpus needs cases "the lens
/// is expected to get *wrong-ish*", because a corpus of only clean positives
/// and negatives proves nothing (PLAT-837's own M2 requirement).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Tier {
    /// The label is not seriously contested.
    Clean,
    /// The lens is expected to find this hard.
    Ambiguous,
}

/// How a returned label compared to what the corpus recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// Matched the primary reading.
    Primary,
    /// Matched a second reader's/labeller's recorded alternative, not the primary.
    Contested,
    /// Matched no recorded reading.
    Wrong,
    /// The lens returned a label outside the question set's `answer_space`.
    Unrecognized,
    /// The lens got no answer for this row at all.
    Unanswered,
}

impl Verdict {
    /// Whether this counts as agreement with the corpus.
    ///
    /// `Contested` counts: the corpus recorded that reading as defensible, so
    /// scoring it wrong would penalise the lens for a disagreement the
    /// labellers themselves had.
    pub(crate) const fn agrees(self) -> bool {
        matches!(self, Self::Primary | Self::Contested)
    }
}

/// One fixture's graded result, independent of what the label space means.
#[derive(Debug, Clone)]
pub(crate) struct Graded {
    /// The corpus id.
    pub(crate) fixture_id: String,
    /// How sure the corpus was.
    pub(crate) tier: Tier,
    /// The primary recorded label.
    pub(crate) expected: String,
    /// Every reading the corpus recorded, including [`Self::expected`].
    pub(crate) contested: Vec<String>,
    /// What the lens returned, for display. May differ from
    /// [`Self::actual_class`] when the underlying answer is continuous (a
    /// coverage score) and this is the raw, unrounded value.
    pub(crate) actual: String,
    /// What the lens returned, as the class every count is keyed off.
    pub(crate) actual_class: String,
    /// How the two compared.
    pub(crate) verdict: Verdict,
    /// The lens's reported confidence, where it carried one.
    pub(crate) confidence: Option<f64>,
}

/// Counts over a graded set, split the ways a report needs them.
#[derive(Debug, Clone, Default)]
pub(crate) struct Tally {
    /// Matched the primary reading.
    pub(crate) primary: usize,
    /// Matched a second reader's alternative.
    pub(crate) contested: usize,
    /// Matched no recorded reading.
    pub(crate) wrong: usize,
    /// Label outside the declared answer space.
    pub(crate) unrecognized: usize,
    /// No answer at all.
    pub(crate) unanswered: usize,
}

impl Tally {
    /// Every graded row counted.
    pub(crate) const fn total(&self) -> usize {
        self.primary + self.contested + self.wrong + self.unrecognized + self.unanswered
    }

    /// Rows where the lens agreed with some recorded reading.
    pub(crate) const fn agreed(&self) -> usize {
        self.primary + self.contested
    }

    /// Agreement as a percentage of every row, or `None` over an empty set.
    ///
    /// Unrecognized and unanswered rows are in the denominator deliberately:
    /// a lens that answers nothing has not scored 100%.
    pub(crate) fn agreement(&self) -> Option<f64> {
        let total = self.total();
        (total > 0).then(|| percent(self.agreed(), total))
    }
}

/// Tallies a graded set.
pub(crate) fn tally(graded: &[Graded]) -> Tally {
    let mut tally = Tally::default();
    for row in graded {
        match row.verdict {
            Verdict::Primary => tally.primary += 1,
            Verdict::Contested => tally.contested += 1,
            Verdict::Wrong => tally.wrong += 1,
            Verdict::Unrecognized => tally.unrecognized += 1,
            Verdict::Unanswered => tally.unanswered += 1,
        }
    }
    tally
}

/// Per-class precision and recall over a graded set, keyed by label.
///
/// M2 requires both, and the false-positive count separately: "a lens that
/// flags everything has perfect recall and is worthless". `predicted` and
/// `expected` are counted against the primary reading only -- a per-class
/// confusion matrix has no way to express "either of two labels was
/// acceptable", so contested rows are reported alongside rather than folded
/// in here.
#[derive(Debug, Clone, Default)]
pub(crate) struct ClassStats {
    /// Times the lens returned this label.
    pub(crate) predicted: usize,
    /// Times the corpus recorded this label as primary.
    pub(crate) expected: usize,
    /// Times both agreed.
    pub(crate) correct: usize,
}

impl ClassStats {
    /// Of the rows the lens gave this label, the share the corpus agreed with.
    pub(crate) fn precision(&self) -> Option<f64> {
        (self.predicted > 0).then(|| percent(self.correct, self.predicted))
    }

    /// Of the rows the corpus gave this label, the share the lens found.
    pub(crate) fn recall(&self) -> Option<f64> {
        (self.expected > 0).then(|| percent(self.correct, self.expected))
    }

    /// Rows the lens gave this label and the corpus did not.
    pub(crate) const fn false_positives(&self) -> usize {
        // `saturating_sub`, not `-`: the invariant that `correct <=
        // predicted` holds only while both are keyed off `actual_class`, and
        // a subtraction that panics in a measurement harness destroys the
        // whole run's numbers over one skewed row.
        self.predicted.saturating_sub(self.correct)
    }
}

/// Builds the per-class confusion counts.
pub(crate) fn class_stats(graded: &[Graded]) -> BTreeMap<String, ClassStats> {
    let mut stats: BTreeMap<String, ClassStats> = BTreeMap::new();
    for row in graded {
        stats.entry(row.actual_class.clone()).or_default().predicted += 1;
        stats.entry(row.expected.clone()).or_default().expected += 1;
        if row.verdict == Verdict::Primary {
            stats.entry(row.expected.clone()).or_default().correct += 1;
        }
    }
    stats
}

/// One confidence decile's accuracy, for the M3 calibration curve.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Bucket {
    /// The decile's lower bound, e.g. `0.9` for 0.9-1.0.
    pub(crate) floor: f64,
    /// Rows whose reported confidence fell in this decile.
    pub(crate) count: usize,
    /// Of those, how many agreed with a recorded reading.
    pub(crate) correct: usize,
    /// The sum of those rows' stated confidences, for the bucket mean.
    pub(crate) confidence_sum: f64,
}

impl Bucket {
    /// Observed accuracy in this decile.
    pub(crate) fn accuracy(&self) -> Option<f64> {
        (self.count > 0).then(|| percent(self.correct, self.count))
    }

    /// The mean stated confidence of the rows in this decile.
    pub(crate) fn mean_confidence(&self) -> Option<f64> {
        (self.count > 0)
            .then(|| self.confidence_sum / f64::from(u32::try_from(self.count).unwrap_or(u32::MAX)))
    }
}

/// Buckets graded rows by reported confidence decile.
///
/// Rows carrying no confidence are excluded and their count returned
/// separately -- silently dropping them would make the calibration curve look
/// denser than the evidence behind it.
pub(crate) fn calibration(graded: &[Graded]) -> (Vec<Bucket>, usize) {
    let mut buckets: Vec<Bucket> = (0..10)
        .map(|decile| Bucket {
            floor: f64::from(decile) / 10.0,
            count: 0,
            correct: 0,
            confidence_sum: 0.0,
        })
        .collect();
    let mut without = 0usize;

    for row in graded {
        let Some(confidence) = row.confidence else {
            without += 1;
            continue;
        };
        // `confidence` is a probability, so `scaled` clamps to `[0.0, 9.0]`
        // before the cast, which is why the cast below cannot truncate or
        // wrap despite the workspace's wire-boundary cast lints being `warn`.
        let scaled = (confidence * 10.0).clamp(0.0, 9.0);
        #[allow(
            clippy::cast_sign_loss,
            clippy::cast_possible_truncation,
            reason = "scaled is clamped to [0.0, 9.0] immediately above"
        )]
        let index = scaled as usize;
        buckets[index].count += 1;
        buckets[index].confidence_sum += confidence;
        if row.verdict.agrees() {
            buckets[index].correct += 1;
        }
    }
    (buckets, without)
}

/// Expected calibration error: the count-weighted mean gap between a
/// decile's mean stated confidence and its observed accuracy, as MP-226's
/// `jev.ece-v1` defines it.
///
/// Before PLAT-1027 this used each decile's midpoint (`floor + 0.05`) in
/// place of the mean. The two agree only when a decile's confidences average
/// to its midpoint, so ECEs published before that change (MP-229) were
/// computed under the midpoint rule.
///
/// Returns `None` when no row carried a confidence at all -- an ECE over
/// nothing is not zero.
pub(crate) fn expected_calibration_error(graded: &[Graded]) -> Option<f64> {
    let (buckets, _) = calibration(graded);
    let total: usize = buckets.iter().map(|bucket| bucket.count).sum();
    if total == 0 {
        return None;
    }
    let error: f64 = buckets
        .iter()
        .filter(|bucket| bucket.count > 0)
        .map(|bucket| {
            let stated = bucket.mean_confidence().unwrap_or(bucket.floor + 0.05);
            let observed = f64::from(u32::try_from(bucket.correct).unwrap_or(u32::MAX))
                / f64::from(u32::try_from(bucket.count).unwrap_or(u32::MAX));
            let weight = f64::from(u32::try_from(bucket.count).unwrap_or(u32::MAX))
                / f64::from(u32::try_from(total).unwrap_or(u32::MAX));
            weight * (stated - observed).abs()
        })
        .sum();
    Some(error)
}

/// The best score a constant predictor can get on this corpus, and the label
/// that gets it.
///
/// # Why this exists, and why raw agreement is not the gate
///
/// MEASURED on the PLAT-917 criterion-strength corpus: answering `sound` to
/// every one of the eleven `weakness_kind` fixtures agreed with a recorded
/// reading 9 times out of 11 -- 81.8% -- because `sound` was one of the two
/// readings on six of the seven contested rows. So a lens that never flags
/// anything scored 81.8% agreement there and had found nothing. That is the
/// mirror image of the trap M2 already names ("a lens that flags everything
/// has perfect recall and is worthless"), and it means an agreement
/// percentage alone cannot be the correctness gate on any corpus with a
/// dominant label. The gate is: beat this baseline, and have non-zero recall
/// on the classes that are not the no-defect answer -- actually find the
/// defects the corpus records.
///
/// Computed from the corpus rather than written down as a constant, so
/// changing the fixture set moves the bar automatically instead of leaving a
/// stale number that silently stops being a bar at all.
pub(crate) fn trivial_baseline(graded: &[Graded]) -> (String, f64) {
    // One constant per family, never one across both. The first version of
    // this function picked a single label over all fifteen criterion-strength
    // rows, which scored the constant predictor at 9/15 (60%) -- `sound`
    // everywhere, earning nothing on the coverage rows. But a constant
    // predictor is free to answer `sound` on a criterion and a fixed level on
    // an FR, and doing so scores higher. The single-label form understated the
    // bar, in the lens's favour. A corpus with no coverage rows (the EARS
    // corpus) has one family, and this reduces to the single-label form.
    let mut labels = Vec::new();
    let mut hits = 0;
    for family in [false, true] {
        let rows: Vec<&Graded> = graded
            .iter()
            .filter(|row| is_coverage(row) == family)
            .collect();
        // Every label in the family's answer space: a label recorded only as
        // a contested alternate is a constant a corpus-blind predictor could
        // answer too (MP-222's `n_{f,i}` ranges over the answer space).
        let mut candidates: Vec<&String> = rows
            .iter()
            .flat_map(|row| std::iter::once(&row.expected).chain(row.contested.iter()))
            .collect();
        candidates.sort();
        candidates.dedup();
        let best = candidates
            .into_iter()
            .map(|label| {
                let count = rows
                    .iter()
                    .filter(|row| row.expected == *label || row.contested.contains(label))
                    .count();
                (label.clone(), count)
            })
            .max_by_key(|(_, count)| *count);
        if let Some((label, count)) = best {
            labels.push(if family {
                format!("level {label}")
            } else {
                label
            });
            hits += count;
        }
    }
    if labels.is_empty() {
        return (String::from("<none>"), 0.0);
    }
    (labels.join(" + "), percent(hits, graded.len()))
}

/// Whether a row grades an FR-level coverage fixture rather than a criterion.
///
/// Coverage rows record a 0-3 rubric level; label rows a label. Keyed off
/// the recorded label's shape because `Graded` carries no family field, and
/// every coverage label in the criterion-strength corpus is an integer while
/// no label in either corpus is.
pub(crate) fn is_coverage(row: &Graded) -> bool {
    row.expected.parse::<u8>().is_ok()
}

/// Recall over every class that is not `no_defect_label`: of the rows the
/// corpus marked as carrying some defect, the share the lens also flagged as
/// some defect.
///
/// Deliberately coarse -- it does not require the lens to pick the *same*
/// defect, only to not wave the row through. Calling a defective row by the
/// wrong defect name is a mislabel; calling it the no-defect answer is the
/// failure the lens exists to prevent, and the gate has to separate those two.
///
/// Coverage rows are excluded. They never return the no-defect label, so the
/// first version of this function counted all four criterion-strength
/// coverage rows as "found" and inflated recall.
pub(crate) fn defect_recall(graded: &[Graded], no_defect_label: &str) -> Option<f64> {
    let defects: Vec<&Graded> = graded
        .iter()
        .filter(|row| !is_coverage(row))
        .filter(|row| {
            row.expected != no_defect_label && !row.contested.iter().any(|c| c == no_defect_label)
        })
        .collect();
    if defects.is_empty() {
        return None;
    }
    // An unanswered or unrecognized row found nothing. Before PLAT-1027 its
    // placeholder class (`<unanswered>`) differed from the no-defect label and
    // so counted as a defect found, inflating recall by every row the lens
    // failed to answer.
    let found = defects
        .iter()
        .filter(|row| !matches!(row.verdict, Verdict::Unanswered | Verdict::Unrecognized))
        .filter(|row| row.actual_class != no_defect_label)
        .count();
    Some(percent(found, defects.len()))
}

/// Recall of the no-defect answer itself (MP-224): of the rows whose primary
/// recorded reading IS `no_defect_label`, the share the lens also answered
/// `no_defect_label`.
///
/// This is the other half of the flag-everything failure `defect_recall`
/// alone cannot see: a lens can score perfect defect recall by returning a
/// defect label unconditionally, and this metric is what catches that. Named
/// separately from `defect_recall` rather than derived from it, because their
/// populations are disjoint (this excludes nothing for a contested no-defect
/// reading the way `defect_recall` does -- MP-224 counts every primary
/// no-defect row).
pub(crate) fn no_defect_recall(graded: &[Graded], no_defect_label: &str) -> Option<f64> {
    let sound: Vec<&Graded> = graded
        .iter()
        .filter(|row| row.expected == no_defect_label)
        .collect();
    if sound.is_empty() {
        return None;
    }
    let found = sound
        .iter()
        .filter(|row| row.actual_class == no_defect_label)
        .count();
    Some(percent(found, sound.len()))
}

/// The share of fixtures whose verdict changed between two runs (M1).
///
/// A row present in one run and missing from the other counts as a change,
/// per the tickets' own definition; the denominator is every fixture id
/// either run graded.
pub(crate) fn disagreement(left: &[Graded], right: &[Graded]) -> Option<f64> {
    let index = |rows: &[Graded]| {
        rows.iter()
            .map(|row| (row.fixture_id.clone(), row.actual_class.clone()))
            .collect::<BTreeMap<_, _>>()
    };
    let (left, right) = (index(left), index(right));
    let mut ids: Vec<&String> = left.keys().chain(right.keys()).collect();
    ids.sort_unstable();
    ids.dedup();
    if ids.is_empty() {
        return None;
    }
    let changed = ids
        .iter()
        .filter(|id| left.get(**id) != right.get(**id))
        .count();
    Some(percent(changed, ids.len()))
}

/// `part` as a percentage of `whole`, to one decimal place's worth of
/// precision.
pub(crate) fn percent(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        return 0.0;
    }
    let part = f64::from(u32::try_from(part).unwrap_or(u32::MAX));
    let whole = f64::from(u32::try_from(whole).unwrap_or(u32::MAX));
    part / whole * 100.0
}

/// Renders the per-fixture table and the M2 rollup.
///
/// Every number says what it counts, per this repo's standing reporting rule;
/// a rate computed over an empty set prints `n/a` rather than `0.0%`.
#[allow(
    clippy::too_many_lines,
    reason = "one linear report section per M2 metric (agreement, baseline, defect/no-defect \
              recall, per-class table, calibration) reads more clearly as one function than \
              split into single-call helpers nothing else reuses"
)]
pub(crate) fn report(title: &str, graded: &[Graded], no_defect_label: &str) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "\n## {title}\n");
    let _ = writeln!(
        out,
        "| Fixture | Tier | Expected | Returned | Verdict | Confidence |"
    );
    let _ = writeln!(out, "| --- | --- | --- | --- | --- | --- |");
    for row in graded {
        let confidence = row
            .confidence
            .map_or_else(|| "-".to_owned(), |value| format!("{value:.2}"));
        let _ = writeln!(
            out,
            "| {} | {:?} | {} | {} | {:?} | {confidence} |",
            row.fixture_id, row.tier, row.expected, row.actual, row.verdict,
        );
    }

    let all = tally(graded);
    let clean: Vec<Graded> = graded
        .iter()
        .filter(|row| row.tier == Tier::Clean)
        .cloned()
        .collect();
    let ambiguous: Vec<Graded> = graded
        .iter()
        .filter(|row| row.tier == Tier::Ambiguous)
        .cloned()
        .collect();

    let _ = writeln!(
        out,
        "\n**Agreement** (primary + contested, over every graded row):\n"
    );
    for (label, set) in [
        ("all", graded),
        ("clean", &clean),
        ("ambiguous", &ambiguous),
    ] {
        let tally = tally(set);
        let rate = tally
            .agreement()
            .map_or_else(|| "n/a".to_owned(), |value| format!("{value:.1}%"));
        let _ = writeln!(
            out,
            "- {label}: {rate} ({}/{} — primary {}, contested {}, wrong {}, unrecognized {}, unanswered {})",
            tally.agreed(),
            tally.total(),
            tally.primary,
            tally.contested,
            tally.wrong,
            tally.unrecognized,
            tally.unanswered,
        );
    }

    let (baseline_label, baseline_rate) = trivial_baseline(graded);
    let _ = writeln!(
        out,
        "\n**Constant-predictor baseline (MP-222 population):** answering `{baseline_label}` \
         to everything scores {baseline_rate:.1}%.",
    );
    // A `no_defect_label` that names no real label in this graded set (the
    // caller passing a sentinel like `"<none>"` for a report where the
    // defect/clean reduction does not apply, e.g. a raw multi-class pattern
    // read) would otherwise print a "100%"/"0%" line that counts nothing
    // meaningful -- skip both lines in that case rather than print a number
    // with no referent.
    let label_is_real = graded
        .iter()
        .any(|row| row.expected == no_defect_label || row.actual_class == no_defect_label);
    if label_is_real {
        if let Some(recall) = defect_recall(graded, no_defect_label) {
            let _ = writeln!(out, "\n**Defect recall (MP-223):** {recall:.1}%");
        }
        if let Some(recall) = no_defect_recall(graded, no_defect_label) {
            let _ = writeln!(out, "\n**No-defect recall (MP-224):** {recall:.1}%");
        }
    }

    let _ = writeln!(
        out,
        "\n**Per class** (against the primary reading only; contested rows are \
         counted in the agreement block above, not here):\n"
    );
    let _ = writeln!(
        out,
        "| Label | Predicted | Expected | Correct | Precision | Recall | False positives |"
    );
    let _ = writeln!(out, "| --- | --- | --- | --- | --- | --- | --- |");
    for (label, stats) in class_stats(graded) {
        let rate = |value: Option<f64>| {
            value.map_or_else(|| "n/a".to_owned(), |value| format!("{value:.1}%"))
        };
        let _ = writeln!(
            out,
            "| {label} | {} | {} | {} | {} | {} | {} |",
            stats.predicted,
            stats.expected,
            stats.correct,
            rate(stats.precision()),
            rate(stats.recall()),
            stats.false_positives(),
        );
    }

    let (buckets, without) = calibration(graded);
    let _ = writeln!(out, "\n**Calibration** (M3):\n");
    for bucket in buckets.iter().filter(|bucket| bucket.count > 0) {
        let _ = writeln!(
            out,
            "- confidence {:.1}-{:.1}: {} row(s), {:.1}% agreed",
            bucket.floor,
            bucket.floor + 0.1,
            bucket.count,
            bucket.accuracy().unwrap_or(0.0),
        );
    }
    let _ = writeln!(
        out,
        "- {without} row(s) carried no confidence and are excluded from the curve",
    );
    let ece = expected_calibration_error(graded).map_or_else(
        || "n/a (no row carried a confidence)".to_owned(),
        |value| format!("{value:.4}"),
    );
    let _ = writeln!(out, "- expected calibration error: {ece}");
    let _ = writeln!(out, "\n(total rows graded: {})", all.total());
    out
}
