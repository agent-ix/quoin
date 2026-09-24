// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `code_exceeds_requirement` variants E1, E2 and E4, and the E3 report
//! (PLAT-1029, parent PLAT-1024; pre-registered in MP-241).
//!
//! E0 (`variant::E0`) asks the question about a whole code body. In PLAT-1014
//! (MP-234) it tied the constant predictor, with every probability between
//! 0.50 and 0.73: a coin flip. The variants here ask about one code unit at a
//! time ([`crate::eval_v2_support::units`]) and decide the row in code.
//!
//! | id | asks, per non-trivial unit | row answer |
//! | --- | --- | --- |
//! | `E1` | a 4-level `task_relation` score plus a `unit_kind` choice cross-check | `yes` when a unit has a strict majority of BOTH answers' mass on "not needed" |
//! | `E4` | the same request as E1 (no extra call) | `yes` on `task_relation` alone |
//! | `E2` | a choice: which requirement clause the unit serves, or `none` | `yes` when a unit's `P(none) >= TAU` |
//! | `E0-RC` | E0's one `code_exceeds_requirement` question, whole body, RC only | `yes` at `p >= 0.5`: the as-asked RC baseline |
//!
//! E3 is not a registered variant. It is [`render_diagnostics`]'s
//! confidence-gated curve over E0's, E1's, E2's and E4's stored answers: rows
//! below a confidence floor abstain, and count in coverage but not in
//! accuracy. Registering it would send the same requests as its base, and the
//! shared grader counts an abstention as a miss, which is not E3's measure.
//!
//! Bar D ([`bar_d`]) is a paired contrast: each `additive_code` mutant against
//! its unmutated source row (`mutation.source_id`). A pair succeeds when the
//! variant's probability crosses [`PAIRED_TAU`] upward and rises by at least
//! [`PAIRED_DELTA`]. It fails when the probability falls by at least delta,
//! or when either row went unanswered. Anything else is a tie. The bar is a one-sided sign test over the
//! non-tie pairs, computed with the shared `metrics::paired_contrast`.
//!
//! # Where the shapes come from
//!
//! - E1 follows jev-code's `check-task` workflow: each changed hunk gets a
//!   four-level `task_relation` score with concrete levels, an independent
//!   `change_kind` choice as a cross-check, a conflict between the two parks
//!   the hunk for a human, decisions read distribution mass rather than the
//!   argmax, deterministic pre-filters run before any model call, and a
//!   circuit breaker fires when most hunks look unrelated. jev-code publishes
//!   no accuracy figure, so this is design, not evidence.
//! - E2 follows William Lyon's notebook 11 ("selection, not generation"): code
//!   enumerates the candidates, the model picks from a closed set that
//!   includes `none`, and the answer's own probability is the filter.
//! - E4 is the ablation Lyon's result predicts: there, a second gate question
//!   cost recall and bought almost no precision. E1's cross-check is such a
//!   gate. E4 reads E1's answers without it, so the comparison costs nothing.
//!
//! # Thresholds, fixed before any E-variant answer exists
//!
//! - [`MAJORITY`] = 0.5, strict. "Most of its mass" means more than half.
//!   It is the only threshold that needs no data to justify it.
//! - [`TAU`] = 0.5, inclusive, as PLAT-1029 states E2's rule (`>= tau`).
//!   [`TAU_SWEEP`] reports the rest of the range on dev only.
//! - The circuit breaker trips when a strict majority of the row's
//!   considered units put a strict majority of their mass on level 0 ("no
//!   visible connection"). Level 1 ("same area, not needed") is the
//!   exceedance signal. Level 0 on most of a body says the requirement is not
//!   about this code: a trace problem. The row then abstains (no answer) and
//!   is reported as trace-suspect, and never as exceedance.
//! - A parked unit (its two answers disagree on "not needed") is unresolved
//!   evidence. A `no` that ignores it cannot be confident, so the row's `no`
//!   confidence is capped at [`PARKED_NO_CONFIDENCE_CAP`] = 0.5, below every
//!   floor the E3 curve draws above 0.5.
//!
//! # Trivial units
//!
//! The pre-filter, applied before any call: a unit is trivial when, with
//! comments and literals blanked ([`crate::eval_v2_support::units::mask`]), it is empty, or
//! when it is a `match` arm or `if`/`else` block ([`UnitKind::Branch`]) whose
//! body is a pass-through: at most one of `return`/`break`/`continue`, at most
//! one of `Ok`/`Err`/`Some`, and a bare path, `()` or a blanked literal, with
//! no call, macro, operator or assignment. `_ => Ok(())` and `None => None`
//! are trivial; `0 => Err(Error::empty())` is not. A whole function is never
//! trivial: even a one-line accessor is behaviour.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::sync::LazyLock;

use regex::Regex;
use typesafe_sdk_questions::{Questions, choice, questions, score};

use crate::eval_v2_support::corpus::{KindGroup, Row};
use crate::eval_v2_support::keys::{Mode, NO, YES, spec};
use crate::eval_v2_support::metrics::{
    self, ContrastSpec, Direction, PairedContrast, Scored, graded, paired_contrast, scored,
    slice_name, summarize,
};
use crate::eval_v2_support::units::{Unit, UnitKind, mask};
use crate::eval_v2_support::variant::{
    Answered, Artifact, Ask, E0, Prediction, Predictions, RawAnswer, RunOutput, Variant,
    code_units, noul_prediction, per_unit_asks, request, state,
};
use crate::gap_semantic_support::{Variant as BatteryShape, question_set as battery_questions};

/// The key every variant here is graded on.
pub(crate) const KEY: &str = "code_exceeds_requirement";

/// "Most of its mass": strictly more than this.
pub(crate) const MAJORITY: f64 = 0.5;

/// E2's rule: `yes` when a unit's `P(none)` is at least this.
pub(crate) const TAU: f64 = 0.5;

/// The `tau` values E2's sweep reports on dev.
pub(crate) const TAU_SWEEP: [f64; 7] = [0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9];

/// The highest confidence a `no` may carry when a unit was parked.
pub(crate) const PARKED_NO_CONFIDENCE_CAP: f64 = 0.5;

/// E3's target coverages, in percent.
pub(crate) const COVERAGE_TARGETS: [usize; 6] = [100, 90, 80, 70, 60, 50];

/// MP-241 bar C: accuracy at this coverage, in percent.
pub(crate) const BAR_C_COVERAGE: usize = 60;

/// The `task_relation` score's key.
pub(crate) const RELATION_KEY: &str = "task_relation";

/// The `unit_kind` cross-check's key.
pub(crate) const KIND_KEY: &str = "unit_kind";

/// E2's selection choice key.
pub(crate) const CLAUSE_KEY: &str = "serves_clause";

/// E2's "serves no clause" label.
pub(crate) const NONE_LABEL: &str = "none";

/// The state field E2 lists the clauses in.
pub(crate) const CLAUSES_FIELD: &str = "requirement_clauses";

/// The `task_relation` levels, lowest first, adapted from jev-code's
/// `task_relation`. Levels 0 and 1 are "not needed".
pub(crate) const RELATION_LEVELS: [&str; 4] = [
    "No visible connection: nothing in this unit relates to what the requirement is about.",
    "Same area, not needed: this unit works on the requirement's subject but makes a \
     decision, output, refusal or side effect the requirement neither states nor implies.",
    "Supports indirectly: this unit is work the stated behaviour needs, such as input \
     parsing, validation, a size limit, error handling, conversion or keeping state \
     consistent, without being the stated behaviour itself.",
    "Directly implements: this unit carries out behaviour the requirement states.",
];

/// The `task_relation` levels that mean "not needed".
const RELATION_LOW: [u32; 2] = [0, 1];

/// The `task_relation` level that means "no visible connection".
const RELATION_UNRELATED: [u32; 1] = [0];

/// The `unit_kind` labels, adapted from jev-code's `change_kind` to a unit
/// that already exists rather than a change.
pub(crate) const KIND_LABELS: [(&str, &str); 4] = [
    (
        "states_behaviour",
        "Carries out behaviour the requirement states.",
    ),
    (
        "supports_requirement",
        "Work the stated behaviour needs: input parsing, validation, a size limit, error \
         handling or its context, conversion, or keeping state consistent.",
    ),
    (
        "adds_behaviour",
        "Adds a decision, output, refusal or side effect the requirement neither states nor \
         implies.",
    ),
    (
        "unrelated",
        "Concerns a different subject from the requirement.",
    ),
];

/// The `unit_kind` labels that mean "not needed", matching levels 0 and 1.
const KIND_LOW: [&str; 2] = ["adds_behaviour", "unrelated"];

/// E2 asks about at most this many clauses; the overflow joins the last.
pub(crate) const MAX_CLAUSES: usize = 12;

/// A sentence piece shorter than this many words joins the clause before it.
const MIN_CLAUSE_WORDS: usize = 3;

/// A `.` after one of these is not a sentence end.
const ABBREVIATIONS: [&str; 5] = ["e.g", "i.e", "etc", "vs", "cf"];

const RC_AND_RTC: &[Mode] = &[Mode::ReqCode, Mode::ReqTestCode];
const CODE_ONLY: &[Artifact] = &[Artifact::Code];

// ---------------------------------------------------------------------------
// The pre-filter
// ---------------------------------------------------------------------------

static PASS_THROUGH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?:(?:return|break|continue)(?:\s+|$))?(?:(?:Ok|Err|Some)\s*)?(?:\(\s*(?:\(\s*\)|[\w:.&]*)\s*\)|[\w:.&]*)$",
    )
    .unwrap_or_else(|error| unreachable!("a literal regex compiles: {error}"))
});

/// A branch unit's body in masked text: after the arm's `=>`, or inside the
/// block's outer braces.
fn branch_body<'a>(unit: &Unit, masked: &'a str) -> &'a str {
    if unit.label.starts_with("match arm") {
        return masked
            .find("=>")
            .and_then(|arrow| masked.get(arrow + 2..))
            .unwrap_or(masked);
    }
    match (masked.find('{'), masked.rfind('}')) {
        (Some(open), Some(close)) if open < close => masked.get(open + 1..close).unwrap_or(masked),
        _ => masked,
    }
}

/// Whitespace collapsed, trailing `,`/`;` and wrapping braces peeled off.
fn normalize(body: &str) -> String {
    let mut text: String = body.split_whitespace().collect::<Vec<_>>().join(" ");
    loop {
        let trimmed = text.trim_end_matches([',', ';']).trim().to_owned();
        let inner = trimmed
            .strip_prefix('{')
            .and_then(|rest| rest.strip_suffix('}'))
            .map(|inner| inner.trim().to_owned());
        match inner {
            Some(inner) => text = inner,
            None if trimmed == text => return text,
            None => text = trimmed,
        }
    }
}

/// Whether `unit` is skipped before any call. See the module doc.
pub(crate) fn is_trivial(unit: &Unit) -> bool {
    let masked = String::from_utf8_lossy(&mask(&unit.text)).into_owned();
    if masked.trim().is_empty() {
        return true;
    }
    unit.kind == UnitKind::Branch && PASS_THROUGH.is_match(&normalize(branch_body(unit, &masked)))
}

/// The row's per-unit asks, minus the trivial units. Each ask keeps the unit
/// index into [`code_units`].
fn nontrivial_asks(row: &Row, questions: fn(&Unit) -> Questions) -> Vec<Ask> {
    let units = code_units(row);
    per_unit_asks(row, questions)
        .into_iter()
        .filter(|ask| {
            ask.unit
                .and_then(|index| units.get(index))
                .is_some_and(|unit| !is_trivial(unit))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Reading mass
// ---------------------------------------------------------------------------

/// How far a distribution's total may sit from 1 before it is refused.
const MASS_TOLERANCE: f64 = 0.05;

/// The mass a score distribution over levels `0..level_count` puts on
/// `levels`.
///
/// # Errors
/// When the distribution is empty, a key is not one of the levels, or the
/// total is not 1 within [`MASS_TOLERANCE`]. A missing or mis-keyed
/// distribution fails loudly; it never reads as zero or as "unanswered".
pub(crate) fn level_mass(
    probabilities: &BTreeMap<String, f64>,
    levels: &[u32],
    level_count: u32,
) -> Result<f64, String> {
    if probabilities.is_empty() {
        return Err("the score came back with no distribution".to_owned());
    }
    let mut mass = 0.0;
    let mut total = 0.0;
    for (key, p) in probabilities {
        let level = key
            .trim()
            .parse::<f64>()
            .ok()
            .and_then(|value| (0..level_count).find(|l| (f64::from(*l) - value).abs() < 1e-9))
            .ok_or_else(|| format!("score key {key:?} is not a level 0..{level_count}"))?;
        total += p;
        if levels.contains(&level) {
            mass += p;
        }
    }
    check_total(total)?;
    Ok(mass)
}

/// The mass a choice distribution puts on `labels`.
///
/// # Errors
/// When the distribution is empty, lacks a label of `space`, carries a key
/// outside it, or does not total 1 within [`MASS_TOLERANCE`].
pub(crate) fn label_mass(
    probabilities: &BTreeMap<String, f64>,
    labels: &[&str],
    space: &[&str],
) -> Result<f64, String> {
    if probabilities.is_empty() {
        return Err("the choice came back with no distribution".to_owned());
    }
    if let Some(stray) = probabilities
        .keys()
        .find(|key| !space.contains(&key.as_str()))
    {
        return Err(format!("choice key {stray:?} is not one of {space:?}"));
    }
    if let Some(missing) = space
        .iter()
        .find(|label| !probabilities.contains_key(**label))
    {
        return Err(format!("the choice gave no probability for {missing:?}"));
    }
    check_total(probabilities.values().sum())?;
    Ok(labels
        .iter()
        .filter_map(|label| probabilities.get(*label))
        .sum())
}

fn check_total(total: f64) -> Result<(), String> {
    if (total - 1.0).abs() > MASS_TOLERANCE {
        return Err(format!("the distribution totals {total}, not 1"));
    }
    Ok(())
}

fn score_distribution<'a>(
    answered: &'a Answered,
    key: &str,
) -> Result<&'a BTreeMap<String, f64>, String> {
    match answered.answers.get(key) {
        Some(RawAnswer::Score { probabilities, .. }) => Ok(probabilities),
        Some(other) => Err(format!("`{key}` came back as {other:?}, not a score")),
        None => Err(format!("`{key}` is missing from the answers")),
    }
}

fn choice_distribution<'a>(
    answered: &'a Answered,
    key: &str,
) -> Result<&'a BTreeMap<String, f64>, String> {
    match answered.answers.get(key) {
        Some(RawAnswer::Choice { probabilities, .. }) => Ok(probabilities),
        Some(other) => Err(format!("`{key}` came back as {other:?}, not a choice")),
        None => Err(format!("`{key}` is missing from the answers")),
    }
}

// ---------------------------------------------------------------------------
// E1 and E4: per-unit task relation
// ---------------------------------------------------------------------------

/// E1's per-unit questions. E4 sends the same request.
pub(crate) fn relation_questions(_unit: &Unit) -> Questions {
    questions([
        (
            RELATION_KEY,
            score(
                "How does this unit of code (`code_unit`, shown in `symbol_body`) relate to the \
                 requirement (`fr_statement`, and `ac_text` when present)? Judge against the \
                 requirement shown and nothing else.",
                RELATION_LEVELS,
            ),
        ),
        (
            KIND_KEY,
            choice(
                "What kind of work does this unit of code (`code_unit`, shown in \
                 `symbol_body`) do, relative to the requirement shown?",
                KIND_LABELS,
            ),
        ),
    ])
}

fn relation_asks(row: &Row) -> Vec<Ask> {
    nontrivial_asks(row, relation_questions)
}

/// One non-trivial unit's reading.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct UnitReading {
    /// The unit's index in [`code_units`].
    pub(crate) unit: usize,
    /// `task_relation` mass on levels 0 and 1 ("not needed").
    pub(crate) relation_low: f64,
    /// `task_relation` mass on level 0 alone ("no visible connection").
    pub(crate) unrelated: f64,
    /// `unit_kind` mass on its two "not needed" labels; `None` when the
    /// variant does not cross-check (E4).
    pub(crate) kind_low: Option<f64>,
}

impl UnitReading {
    /// The two answers disagree on "not needed".
    pub(crate) fn parked(&self) -> bool {
        self.kind_low
            .is_some_and(|kind| (kind > MAJORITY) != (self.relation_low > MAJORITY))
    }

    /// How strongly the unit says "not needed": the weaker of the two
    /// answers when cross-checked, so both must agree.
    pub(crate) fn strength(&self) -> f64 {
        self.kind_low
            .map_or(self.relation_low, |kind| kind.min(self.relation_low))
    }

    /// A strict majority says "not needed", on both answers when
    /// cross-checked.
    pub(crate) fn exceeds(&self) -> bool {
        self.strength() > MAJORITY
    }

    /// A strict majority on "no visible connection".
    pub(crate) fn looks_unrelated(&self) -> bool {
        self.unrelated > MAJORITY
    }
}

/// What E1 or E4 concluded about a row.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RelationOutcome {
    /// A graded answer.
    Decided(Prediction),
    /// The circuit breaker tripped: most units look unrelated, so the row is
    /// reported as a trace problem and not answered.
    TraceSuspect {
        /// Units that look unrelated.
        unrelated: usize,
        /// Units considered.
        considered: usize,
    },
}

/// E1's or E4's full reading of a row.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RelationAssessment {
    /// Units skipped as trivial before any call.
    pub(crate) trivial: usize,
    /// One reading per considered unit.
    pub(crate) readings: Vec<UnitReading>,
    /// The row's outcome.
    pub(crate) outcome: RelationOutcome,
}

impl RelationAssessment {
    /// Units whose two answers conflict.
    pub(crate) fn parked(&self) -> usize {
        self.readings.iter().filter(|r| r.parked()).count()
    }

    /// Units that exceed.
    pub(crate) fn exceeding(&self) -> usize {
        self.readings.iter().filter(|r| r.exceeds()).count()
    }
}

/// One unit's reading, or why its answers cannot be read.
fn read_unit(answer: &Answered, unit: usize, cross_check: bool) -> Result<UnitReading, String> {
    let relation = score_distribution(answer, RELATION_KEY)?;
    let level_count = u32::try_from(RELATION_LEVELS.len()).unwrap_or(u32::MAX);
    let kind_space: Vec<&str> = KIND_LABELS.iter().map(|(label, _)| *label).collect();
    let kind_low = if cross_check {
        Some(label_mass(
            choice_distribution(answer, KIND_KEY)?,
            &KIND_LOW,
            &kind_space,
        )?)
    } else {
        None
    };
    Ok(UnitReading {
        unit,
        relation_low: level_mass(relation, &RELATION_LOW, level_count)?,
        unrelated: level_mass(relation, &RELATION_UNRELATED, level_count)?,
        kind_low,
    })
}

/// Reads a row's per-unit answers. `cross_check` is E1 (`true`) or E4.
///
/// # Errors
/// When any unit's answers cannot be read, naming the row and the unit.
pub(crate) fn assess_relation(
    row: &Row,
    answered: &[Answered],
    cross_check: bool,
) -> Result<RelationAssessment, String> {
    let trivial = code_units(row).iter().filter(|u| is_trivial(u)).count();
    let readings = answered
        .iter()
        .filter_map(|answer| answer.unit.map(|unit| (answer, unit)))
        .map(|(answer, unit)| {
            read_unit(answer, unit, cross_check)
                .map_err(|error| format!("{} unit {unit}: {error}", row.id))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let outcome = relation_outcome(&readings);
    Ok(RelationAssessment {
        trivial,
        readings,
        outcome,
    })
}

fn relation_outcome(readings: &[UnitReading]) -> RelationOutcome {
    let considered = readings.len();
    let unrelated = readings.iter().filter(|r| r.looks_unrelated()).count();
    if considered > 0 && unrelated * 2 > considered {
        return RelationOutcome::TraceSuspect {
            unrelated,
            considered,
        };
    }
    let ordinal = readings
        .iter()
        .map(UnitReading::strength)
        .reduce(f64::max)
        .unwrap_or(0.0);
    let yes = ordinal > MAJORITY;
    let confidence = if yes {
        ordinal
    } else if readings.iter().any(UnitReading::parked) {
        (1.0 - ordinal).min(PARKED_NO_CONFIDENCE_CAP)
    } else {
        1.0 - ordinal
    };
    RelationOutcome::Decided(Prediction {
        answer: if yes { YES } else { NO }.to_owned(),
        confidence: Some(confidence),
        ordinal: Some(ordinal),
    })
}

/// `derive` has no error channel, and an unreadable answer must never become
/// a quiet "unanswered": the run stops, naming the row and the unit. With a
/// cassette, re-running after a fix costs nothing.
#[allow(
    clippy::panic,
    reason = "an unreadable answer stops the measurement; it is never scored"
)]
fn loud<T>(result: Result<T, String>) -> T {
    result.unwrap_or_else(|error| panic!("PLAT-1029: unreadable answer: {error}"))
}

fn prediction_of(outcome: RelationOutcome) -> Predictions {
    match outcome {
        RelationOutcome::Decided(prediction) => Predictions::from([(KEY, prediction)]),
        RelationOutcome::TraceSuspect { .. } => Predictions::new(),
    }
}

fn e1_derive(row: &Row, answered: &[Answered]) -> Predictions {
    prediction_of(loud(assess_relation(row, answered, true)).outcome)
}

fn e4_derive(row: &Row, answered: &[Answered]) -> Predictions {
    prediction_of(loud(assess_relation(row, answered, false)).outcome)
}

/// E1: per-unit task relation with the `unit_kind` cross-check.
pub(crate) const E1: Variant = Variant {
    id: "E1",
    version: 1,
    summary: "per-unit task_relation score + unit_kind cross-check; yes if a unit is \
              'not needed' on both; parks conflicts; trace breaker",
    modes: RC_AND_RTC,
    references: CODE_ONLY,
    grades: &[KEY],
    asks: relation_asks,
    derive: e1_derive,
};

/// E4: E1's request, read on `task_relation` alone (no second gate).
pub(crate) const E4: Variant = Variant {
    id: "E4",
    version: 1,
    summary: "E1's answers without the unit_kind gate (Lyon nb 11: a second gate costs recall)",
    modes: RC_AND_RTC,
    references: CODE_ONLY,
    grades: &[KEY],
    asks: relation_asks,
    derive: e4_derive,
};

// ---------------------------------------------------------------------------
// E2: per-unit selection
// ---------------------------------------------------------------------------

/// `line` without a leading list marker (`- `, `* `, `+ `, `1. `, `1) `), or
/// `None` when it has none.
fn strip_list_marker(line: &str) -> Option<&str> {
    for bullet in ["- ", "* ", "+ "] {
        if let Some(rest) = line.strip_prefix(bullet) {
            return Some(rest.trim_start());
        }
    }
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    let rest = line.get(digits..).filter(|_| digits > 0)?;
    rest.strip_prefix(". ")
        .or_else(|| rest.strip_prefix(") "))
        .map(str::trim_start)
}

/// A source's list items: a marked line starts a new item, a blank line
/// ends one, and any other line continues the current one (wrapped prose).
fn list_items(source: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut current = String::new();
    for line in source.lines() {
        let trimmed = line.trim();
        let marked = strip_list_marker(trimmed);
        if (trimmed.is_empty() || marked.is_some()) && !current.is_empty() {
            items.push(std::mem::take(&mut current));
        }
        let text = marked.unwrap_or(trimmed);
        if !text.is_empty() {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(text);
        }
    }
    if !current.is_empty() {
        items.push(current);
    }
    items
}

fn ends_with_abbreviation(text: &str) -> bool {
    text.split_whitespace().next_back().is_some_and(|word| {
        let word = word.trim_start_matches(['(', '[']).to_lowercase();
        ABBREVIATIONS.contains(&word.as_str())
    })
}

/// An item's sentences: split after `;`, and after `.`, `!` or `?` when
/// followed by whitespace or the end and not closing an abbreviation.
fn sentences(item: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut chars = item.char_indices().peekable();
    while let Some((at, ch)) = chars.next() {
        let next_is_space = chars.peek().is_none_or(|(_, next)| next.is_whitespace());
        let boundary = match ch {
            ';' => true,
            '.' | '!' | '?' => {
                next_is_space && !ends_with_abbreviation(item.get(start..at).unwrap_or_default())
            }
            _ => false,
        };
        if boundary {
            let end = at + ch.len_utf8();
            out.push(item.get(start..end).unwrap_or_default().to_owned());
            start = end;
        }
    }
    out.push(item.get(start..).unwrap_or_default().to_owned());
    out.into_iter()
        .map(|piece| piece.trim().trim_end_matches(';').trim().to_owned())
        .filter(|piece| piece.chars().any(char::is_alphanumeric))
        .collect()
}

/// What two clauses must share to be the same clause.
fn clause_key(clause: &str) -> String {
    clause
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_end_matches(['.', ';', '!', '?'])
        .to_lowercase()
}

/// Splits a requirement into clauses for E2. Deterministic, one rule:
///
/// 1. The statement, then the acceptance criterion when there is one.
/// 2. Each into list items: a line opening with `- `, `* `, `+ `, `1. ` or
///    `1) ` starts an item, a blank line ends one, any other line continues
///    the current one.
/// 3. Each item into sentences: after `;`, and after `.`, `!` or `?` followed
///    by whitespace or the end, unless the word before is `e.g`, `i.e`,
///    `etc`, `vs` or `cf`. `4.5` does not split; `and` never splits.
/// 4. A piece with no letter or digit is dropped. A piece of fewer than three
///    words joins the clause before it.
/// 5. A clause equal to an earlier one (case, spacing and trailing
///    punctuation aside) is dropped: an AC often restates its statement.
/// 6. At most [`MAX_CLAUSES`]; the overflow joins the last.
pub(crate) fn requirement_clauses(statement: &str, criterion: Option<&str>) -> Vec<String> {
    let mut clauses: Vec<String> = Vec::new();
    for source in std::iter::once(statement).chain(criterion) {
        for item in list_items(source) {
            for piece in sentences(&item) {
                match clauses.last_mut() {
                    Some(last) if piece.split_whitespace().count() < MIN_CLAUSE_WORDS => {
                        last.push(' ');
                        last.push_str(&piece);
                    }
                    _ => clauses.push(piece),
                }
            }
        }
    }
    let mut seen = BTreeSet::new();
    clauses.retain(|clause| seen.insert(clause_key(clause)));
    if clauses.len() > MAX_CLAUSES {
        let overflow = clauses.split_off(MAX_CLAUSES - 1).join(" ");
        clauses.push(overflow);
    }
    clauses
}

/// The clause labels: `C1` .. `Cn`.
pub(crate) fn clause_label(index: usize) -> String {
    format!("C{}", index + 1)
}

/// E2's question for `count` clauses.
pub(crate) fn selection_questions(count: usize) -> Questions {
    let mut options: Vec<(String, String)> = (0..count)
        .map(|index| {
            let label = clause_label(index);
            let text = format!("Serves clause {label} of `{CLAUSES_FIELD}`.");
            (label, text)
        })
        .collect();
    options.push((
        NONE_LABEL.to_owned(),
        "Serves none of the clauses: it makes a decision, output, refusal or side effect no \
         clause states or needs."
            .to_owned(),
    ));
    questions([(
        CLAUSE_KEY,
        choice(
            format!(
                "Which clause of the requirement (listed in `{CLAUSES_FIELD}`) does this unit of \
                 code (`code_unit`, shown in `symbol_body`) serve, directly or as work that \
                 clause needs such as input parsing, validation, a size limit or error \
                 handling? Choose `{NONE_LABEL}` only when it serves no clause."
            ),
            options,
        ),
    )])
}

fn selection_asks(row: &Row) -> Vec<Ask> {
    let requirement = &row.requirement;
    let clauses = requirement_clauses(&requirement.statement, requirement.ac_text.as_deref());
    if clauses.is_empty() {
        return Vec::new();
    }
    let listed: serde_json::Map<String, serde_json::Value> = clauses
        .iter()
        .enumerate()
        .map(|(index, clause)| (clause_label(index), clause.clone().into()))
        .collect();
    let questions = selection_questions(clauses.len());
    nontrivial_asks(row, |_| Questions::new())
        .into_iter()
        .map(|mut ask| {
            if let serde_json::Value::Object(fields) = &mut ask.request.state.0 {
                fields.insert(
                    CLAUSES_FIELD.to_owned(),
                    serde_json::Value::Object(listed.clone()),
                );
            }
            ask.request.questions.clone_from(&questions);
            ask
        })
        .collect()
}

/// E2's reading of a row: `Ok(None)` only when the requirement yields no
/// clause, so nothing was asked.
///
/// # Errors
/// When a unit's choice cannot be read, naming the row and the unit.
pub(crate) fn assess_selection(
    row: &Row,
    answered: &[Answered],
) -> Result<Option<Prediction>, String> {
    let requirement = &row.requirement;
    let clauses = requirement_clauses(&requirement.statement, requirement.ac_text.as_deref());
    if clauses.is_empty() {
        return Ok(None);
    }
    let mut space: Vec<String> = (0..clauses.len()).map(clause_label).collect();
    space.push(NONE_LABEL.to_owned());
    let space: Vec<&str> = space.iter().map(String::as_str).collect();
    let mut highest = 0.0f64;
    for (answer, unit) in answered
        .iter()
        .filter_map(|answer| answer.unit.map(|unit| (answer, unit)))
    {
        let none = choice_distribution(answer, CLAUSE_KEY)
            .and_then(|p| label_mass(p, &[NONE_LABEL], &space))
            .map_err(|error| format!("{} unit {unit}: {error}", row.id))?;
        highest = highest.max(none);
    }
    let yes = highest >= TAU;
    Ok(Some(Prediction {
        answer: if yes { YES } else { NO }.to_owned(),
        confidence: Some(if yes { highest } else { 1.0 - highest }),
        ordinal: Some(highest),
    }))
}

fn e2_derive(row: &Row, answered: &[Answered]) -> Predictions {
    loud(assess_selection(row, answered))
        .map(|prediction| Predictions::from([(KEY, prediction)]))
        .unwrap_or_default()
}

/// E2: per-unit selection, "which clause does this unit serve, or none".
pub(crate) const E2: Variant = Variant {
    id: "E2",
    version: 1,
    summary: "per-unit choice of the requirement clause served, or none; yes if a unit's \
              P(none) >= 0.5",
    modes: RC_AND_RTC,
    references: CODE_ONLY,
    grades: &[KEY],
    asks: selection_asks,
    derive: e2_derive,
};

// ---------------------------------------------------------------------------
// E0-RC: the as-asked baseline on requirement-plus-code rows
// ---------------------------------------------------------------------------

/// The key PLAT-1030's trace check answers.
pub(crate) const TRACE_KEY: &str = "trace_correct";

/// E0-RC's one question: `FullBatteryV1`'s `code_exceeds_requirement`
/// question, taken from the battery itself so it cannot drift.
///
/// The one-line diff from E0: E0 sends all seven battery questions, and
/// E0-RC sends only this one. The six it drops are the ones that reference
/// the test (`test_asserts_intent`, `assertion_vacuous`,
/// `tests_only_its_own_mock`) or the divergence between test and code
/// (`divergence_kind`, `severity`), plus `code_implements_intent`, which is
/// not this key. The question text is byte-identical: "Does the code
/// implement behaviour no requirement states?"
pub(crate) fn e0_rc_questions() -> Questions {
    battery_questions(BatteryShape::FullBatteryV1)
        .get(KEY)
        .map(|question| questions([(KEY, question.clone())]))
        .unwrap_or_default()
}

fn e0_rc_asks(row: &Row) -> Vec<Ask> {
    vec![Ask {
        unit: None,
        request: request(state(row), e0_rc_questions()),
    }]
}

fn e0_rc_derive(_row: &Row, answered: &[Answered]) -> Predictions {
    answered
        .iter()
        .find(|answer| answer.unit.is_none())
        .and_then(|answer| noul_prediction(&answer.answers, KEY))
        .map(|prediction| Predictions::from([(KEY, prediction)]))
        .unwrap_or_default()
}

/// E0-RC: E0's question alone, on RC rows. MP-241's RC comparator.
pub(crate) const E0_RC: Variant = Variant {
    id: "E0-RC",
    version: 1,
    summary: "FullBatteryV1's code_exceeds_requirement question alone (no test questions), RC",
    modes: &[Mode::ReqCode],
    references: CODE_ONLY,
    grades: &[KEY],
    asks: e0_rc_asks,
    derive: e0_rc_derive,
};

// ---------------------------------------------------------------------------
// Reports: E1/E4 diagnostics, E2's tau sweep, E3's gated curve
// ---------------------------------------------------------------------------

/// E1's or E4's counts over a run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RelationCounts {
    /// Rows the variant ran on.
    pub(crate) rows: usize,
    /// Rows answered.
    pub(crate) decided: usize,
    /// Rows the breaker reported as trace-suspect.
    pub(crate) trace_suspect: usize,
    /// Trace-suspect rows by their `code_exceeds_requirement` truth
    /// (`yes`/`no`/`unlabelled`), each tagged with its truth group.
    pub(crate) trace_suspect_truth: BTreeMap<String, usize>,
    /// Units skipped as trivial.
    pub(crate) trivial_units: usize,
    /// Units asked about and read.
    pub(crate) considered_units: usize,
    /// Units parked.
    pub(crate) parked_units: usize,
    /// Units that exceed.
    pub(crate) exceeding_units: usize,
}

/// Tallies E1's (`cross_check`) or E4's assessments over `output`.
pub(crate) fn relation_counts(
    rows: &[Row],
    output: &RunOutput,
    variant: &Variant,
    cross_check: bool,
) -> RelationCounts {
    let label = variant.label();
    let by_id: BTreeMap<&str, &Row> = rows.iter().map(|row| (row.id.as_str(), row)).collect();
    let mut counts = RelationCounts::default();
    for result in output.results.iter().filter(|r| r.variant == label) {
        let Some(row) = by_id.get(result.row_id.as_str()) else {
            continue;
        };
        let assessment = loud(assess_relation(row, &result.answered, cross_check));
        counts.rows += 1;
        counts.trivial_units += assessment.trivial;
        counts.considered_units += assessment.readings.len();
        counts.parked_units += assessment.parked();
        counts.exceeding_units += assessment.exceeding();
        match assessment.outcome {
            RelationOutcome::Decided(_) => counts.decided += 1,
            RelationOutcome::TraceSuspect { .. } => {
                counts.trace_suspect += 1;
                let truth = row.truth.get(KEY).map_or_else(
                    || "unlabelled".to_owned(),
                    |truth| format!("{} ({})", truth.answer.label(), truth.kind.group().label()),
                );
                *counts.trace_suspect_truth.entry(truth).or_default() += 1;
            }
        }
    }
    counts
}

/// One `tau` of a threshold sweep.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SweepPoint {
    /// The threshold: `yes` when the row's ordinal is at least this.
    pub(crate) tau: f64,
    /// Agreement, as a percentage.
    pub(crate) agreement: Option<f64>,
    /// The constant predictor, as a percentage.
    pub(crate) baseline: f64,
    /// `yes`-class recall, as a percentage.
    pub(crate) yes_recall: Option<f64>,
    /// `no`-class recall, as a percentage.
    pub(crate) no_recall: Option<f64>,
}

/// Re-thresholds each row's stored ordinal at every `tau`, with no new call.
/// A row without an ordinal stays unanswered at every `tau`.
pub(crate) fn tau_sweep(rows: &[Scored], taus: &[f64]) -> Vec<SweepPoint> {
    let Some(key) = spec(KEY) else {
        return Vec::new();
    };
    taus.iter()
        .map(|tau| {
            let rethresholded: Vec<Scored> = rows
                .iter()
                .map(|row| {
                    let prediction = row.prediction.as_ref().and_then(|p| p.ordinal).map(|o| {
                        let yes = o >= *tau;
                        Prediction {
                            answer: if yes { YES } else { NO }.to_owned(),
                            confidence: Some(if yes { o } else { 1.0 - o }),
                            ordinal: Some(o),
                        }
                    });
                    Scored {
                        prediction,
                        ..row.clone()
                    }
                })
                .collect();
            let summary = summarize(&rethresholded, key);
            SweepPoint {
                tau: *tau,
                agreement: summary.agreement,
                baseline: summary.baseline,
                yes_recall: summary.defect_recall,
                no_recall: summary.no_defect_recall,
            }
        })
        .collect()
}

/// One point of E3's gated curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GatePoint {
    /// The target coverage, in percent.
    pub(crate) target: usize,
    /// The confidence floor that reaches it: the confidence of the row at
    /// rank `ceil(target% * total)`, highest first. `None` when fewer rows
    /// than that carry a confidence.
    pub(crate) floor: Option<f64>,
    /// Rows at or above the floor (ties kept, so coverage may exceed the
    /// target).
    pub(crate) kept: usize,
    /// Of those, rows agreeing with a recorded reading.
    pub(crate) correct: usize,
    /// Every row, answered or not.
    pub(crate) total: usize,
}

impl GatePoint {
    /// `kept / total`, as a percentage.
    pub(crate) fn coverage(&self) -> Option<f64> {
        (self.total > 0).then(|| crate::eval_v2_support::grading::percent(self.kept, self.total))
    }

    /// `correct / kept`, as a percentage: abstentions are not in it.
    pub(crate) fn accuracy(&self) -> Option<f64> {
        (self.kept > 0).then(|| crate::eval_v2_support::grading::percent(self.correct, self.kept))
    }
}

/// E3: at each target coverage, the confidence floor that reaches it and the
/// accuracy of the rows it keeps. An unanswered row counts in `total` and is
/// never kept.
pub(crate) fn gated_curve(rows: &[Scored], targets: &[usize]) -> Vec<GatePoint> {
    let Some(key) = spec(KEY) else {
        return Vec::new();
    };
    let graded = graded(rows, key);
    let mut confidences: Vec<f64> = graded.iter().filter_map(|row| row.confidence).collect();
    confidences.sort_by(|left, right| right.total_cmp(left));
    let total = graded.len();
    targets
        .iter()
        .map(|target| {
            let rank = (target * total).div_ceil(100);
            let floor = rank
                .checked_sub(1)
                .and_then(|index| confidences.get(index))
                .copied();
            let (kept, correct) = floor.map_or((0, 0), |floor| {
                graded
                    .iter()
                    .filter(|row| row.confidence.is_some_and(|c| c >= floor))
                    .fold((0, 0), |(kept, correct), row| {
                        (kept + 1, correct + usize::from(row.verdict.agrees()))
                    })
            });
            GatePoint {
                target: *target,
                floor,
                kept,
                correct,
                total,
            }
        })
        .collect()
}

fn show(value: Option<f64>) -> String {
    value.map_or_else(|| "n/a".to_owned(), |value| format!("{value:.1}%"))
}

/// The slices every E-variant table is read on: all rows, each mode, and
/// the known-truth and agent-labelled groups apart.
fn slices(rows: &[Scored]) -> Vec<(String, Vec<Scored>)> {
    let pick = |keep: &dyn Fn(&Scored) -> bool| -> Vec<Scored> {
        rows.iter().filter(|row| keep(row)).cloned().collect()
    };
    let mut out = vec![(slice_name("all", rows), rows.to_vec())];
    for mode in RC_AND_RTC {
        let slice = pick(&|row| row.mode == *mode);
        out.push((
            slice_name(&format!("mode {}", mode.as_str()), &slice),
            slice,
        ));
    }
    let known = pick(&|row| row.kind.group() != KindGroup::Agent);
    out.push((slice_name("known truth", &known), known));
    let agent = pick(&|row| row.kind.group() == KindGroup::Agent);
    out.push((KindGroup::Agent.label().to_owned(), agent));
    out.into_iter()
        .filter(|(_, slice)| !slice.is_empty())
        .collect()
}

fn render_relation_counts(out: &mut String, label: &str, counts: &RelationCounts) {
    let _ = writeln!(
        out,
        "\n#### {label}: units and the circuit breaker\n\n\
         | Rows | Answered | Trace-suspect (breaker) | Units trivial | Units asked | \
         Units parked | Units exceeding |\n| --- | --- | --- | --- | --- | --- | --- |\n\
         | {} | {} | {} | {} | {} | {} | {} |",
        counts.rows,
        counts.decided,
        counts.trace_suspect,
        counts.trivial_units,
        counts.considered_units,
        counts.parked_units,
        counts.exceeding_units,
    );
    if !counts.trace_suspect_truth.is_empty() {
        let truth: Vec<String> = counts
            .trace_suspect_truth
            .iter()
            .map(|(truth, count)| format!("{truth}: {count}"))
            .collect();
        let _ = writeln!(
            out,
            "\nTrace-suspect rows by their `{KEY}` truth: {}. They are unanswered in the \
             graded tables.",
            truth.join(", ")
        );
    }
}

fn render_sweep(out: &mut String, label: &str, rows: &[Scored]) {
    let _ = writeln!(
        out,
        "\n#### {label}: tau sweep (dev only; `yes` when P(none) >= tau)\n"
    );
    for (name, slice) in slices(rows) {
        let _ = writeln!(
            out,
            "{name}:\n\n| tau | Agreement | Constant | yes recall | no recall |\n\
             | --- | --- | --- | --- | --- |"
        );
        for point in tau_sweep(&slice, &TAU_SWEEP) {
            let _ = writeln!(
                out,
                "| {:.1} | {} | {:.1}% | {} | {} |",
                point.tau,
                show(point.agreement),
                point.baseline,
                show(point.yes_recall),
                show(point.no_recall),
            );
        }
        let _ = writeln!(out);
    }
}

/// The as-asked baseline for `mode`: E0 on RTC, E0-RC on RC.
fn baseline_for(mode: Mode) -> Variant {
    if mode == Mode::ReqCode { E0_RC } else { E0 }
}

/// Bar C's comparator on `slice` (one mode): the as-asked baseline's
/// agreement on the rows of the slice it answered. `None` when it did not
/// run on them.
fn bar_c_comparator(baseline: &[Scored], slice: &[Scored], mode: Mode) -> Option<(String, f64)> {
    let key = spec(KEY)?;
    let ids: BTreeSet<&str> = slice.iter().map(|row| row.row_id.as_str()).collect();
    let same: Vec<Scored> = answered_only(baseline)
        .into_iter()
        .filter(|row| ids.contains(row.row_id.as_str()))
        .collect();
    let agreement = summarize(&same, key).agreement?;
    Some((
        format!("{} overall (rows it answered)", baseline_for(mode).label()),
        agreement,
    ))
}

fn render_gate(out: &mut String, label: &str, rows: &[Scored], baseline: &[Scored]) {
    let _ = writeln!(
        out,
        "\n#### E3 over {label}: confidence-gated coverage / accuracy\n"
    );
    for (name, slice) in slices(rows) {
        let _ = writeln!(
            out,
            "{name}:\n\n| Target coverage | Floor | Kept | Coverage | Accuracy |\n\
             | --- | --- | --- | --- | --- |"
        );
        for point in gated_curve(&slice, &COVERAGE_TARGETS) {
            let _ = writeln!(
                out,
                "| {}% | {} | {}/{} | {} | {} |",
                point.target,
                point
                    .floor
                    .map_or_else(|| "unreachable".to_owned(), |f| format!("{f:.3}")),
                point.kept,
                point.total,
                show(point.coverage()),
                show(point.accuracy()),
            );
        }
        let _ = writeln!(out);
    }
    for mode in RC_AND_RTC {
        let slice: Vec<Scored> = rows
            .iter()
            .filter(|row| row.mode == *mode)
            .cloned()
            .collect();
        if slice.is_empty() {
            continue;
        }
        let Some(point) = gated_curve(&slice, &[BAR_C_COVERAGE]).into_iter().next() else {
            continue;
        };
        let Some((comparator, value)) = bar_c_comparator(baseline, &slice, *mode) else {
            let _ = writeln!(
                out,
                "Bar C, mode {}: no comparator in this run.",
                mode.as_str()
            );
            continue;
        };
        let holds = point.accuracy().is_some_and(|accuracy| accuracy > value);
        let _ = writeln!(
            out,
            "Bar C, {}: accuracy at >= {BAR_C_COVERAGE}% coverage {} vs {comparator} {value:.1}% \
             -> {}",
            slice_name(&format!("mode {}", mode.as_str()), &slice),
            show(point.accuracy()),
            if holds { "HOLDS" } else { "fails" }
        );
    }
}

/// MP-241's abstention ceiling, in percent: a variant that leaves more of
/// its population unanswered than this is not selectable.
pub(crate) const ABSTENTION_CEILING: usize = 20;

/// The rows a variant answered.
pub(crate) fn answered_only(rows: &[Scored]) -> Vec<Scored> {
    rows.iter()
        .filter(|row| row.prediction.is_some())
        .cloned()
        .collect()
}

/// Whether `abstained` of `total` is above [`ABSTENTION_CEILING`].
pub(crate) fn above_ceiling(abstained: usize, total: usize) -> bool {
    abstained * 100 > ABSTENTION_CEILING * total
}

/// MP-241's bars A and B are read on the rows the variant answered, with the
/// constant predictor on those same rows; its abstentions are counted beside
/// them and checked against the ceiling.
fn render_answered(out: &mut String, label: &str, rows: &[Scored]) {
    let Some(key) = spec(KEY) else {
        return;
    };
    let _ = writeln!(
        out,
        "\n#### {label}: on the rows it answered (bars A and B)\n\n\
         | Slice | Rows | Abstained | Answered | Agreement | Constant (same rows) | Margin | \
         yes recall | no recall |\n| --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    );
    for (name, slice) in slices(rows) {
        let answered = answered_only(&slice);
        let abstained = slice.len() - answered.len();
        let summary = summarize(&answered, key);
        let _ = writeln!(
            out,
            "| {name} | {} | {abstained}{} | {} | {} | `{}` {:.1}% | {} | {} | {} |",
            slice.len(),
            if above_ceiling(abstained, slice.len()) {
                " (ABOVE CEILING)"
            } else {
                ""
            },
            answered.len(),
            show(summary.agreement),
            summary.baseline_label,
            summary.baseline,
            summary
                .margin()
                .map_or_else(|| "n/a".to_owned(), |m| format!("{m:+.1}pp")),
            show(summary.defect_recall),
            show(summary.no_defect_recall),
        );
    }
}

/// A comparison of `label` against the as-asked baseline (E0 on RTC, E0-RC
/// on RC), per mode, over the rows both answered.
fn render_paired(out: &mut String, label: &str, rows: &[Scored], baseline: &[Scored]) {
    let Some(key) = spec(KEY) else {
        return;
    };
    let theirs: BTreeSet<&str> = baseline
        .iter()
        .filter(|row| row.prediction.is_some())
        .map(|row| row.row_id.as_str())
        .collect();
    for mode in RC_AND_RTC {
        let ours: Vec<Scored> = answered_only(rows)
            .into_iter()
            .filter(|row| row.mode == *mode && theirs.contains(row.row_id.as_str()))
            .collect();
        if ours.is_empty() {
            continue;
        }
        let ids: BTreeSet<&str> = ours.iter().map(|row| row.row_id.as_str()).collect();
        let base: Vec<Scored> = baseline
            .iter()
            .filter(|row| ids.contains(row.row_id.as_str()))
            .cloned()
            .collect();
        let (mine, theirs_summary) = (summarize(&ours, key), summarize(&base, key));
        let _ = writeln!(
            out,
            "\n{label} vs {} on the {} {} row(s) both answered{}: {} vs {}.",
            baseline_for(*mode).label(),
            ours.len(),
            mode.as_str(),
            if ours.iter().any(|row| row.kind.group() == KindGroup::Agent) {
                " (incl. AGENT-LABELLED)"
            } else {
                ""
            },
            show(mine.agreement),
            show(theirs_summary.agreement),
        );
    }
}

/// MP-241 bar D's delta: a move in the variant's probability smaller than
/// this, either way, is a tie and is excluded.
pub(crate) const PAIRED_DELTA: f64 = 0.10;

/// MP-241 bar D's tau: a success must cross it, from a source below it to a
/// mutant at or above it, so the answer flips to `yes`.
pub(crate) const PAIRED_TAU: f64 = 0.5;

/// The mutation kinds bar D pairs: each adds behaviour the requirement does
/// not state, so `code_exceeds_requirement` should rise.
pub(crate) const PAIRED_KINDS: [&str; 1] = ["additive_code"];

/// A mutant's source row id, read from [`metrics::SOURCE_ID_FIELD`].
///
/// This is a stub until PLAT-1025 adds `source_id` to the corpus schema's
/// `mutation`, when it becomes `row.mutation.as_ref()?.source_id.clone()`.
/// Until then a run holding an `additive_code` mutant stops at bar D with
/// "names no `mutation.source_id`". It stops loudly, and never reports zero
/// pairs.
pub(crate) fn source_id(_row: &Row) -> Option<String> {
    None
}

/// Bar D for one variant.
///
/// # Errors
/// As [`paired_contrast`]: a pairing the corpus does not support.
pub(crate) fn bar_d(
    rows: &[Row],
    output: &RunOutput,
    variant: &Variant,
    source_of: &dyn Fn(&Row) -> Option<String>,
) -> Result<PairedContrast, String> {
    let label = variant.label();
    paired_contrast(
        rows,
        output,
        &ContrastSpec {
            variant: &label,
            modes: variant.modes,
            key: KEY,
            kinds: &PAIRED_KINDS,
            direction: Direction::Up,
            tau: PAIRED_TAU,
            delta: PAIRED_DELTA,
            defect_side: &[YES],
        },
        source_of,
    )
}

/// A pairing the corpus does not support stops the report: bar D is never
/// read over the pairs that happened to resolve.
#[allow(
    clippy::panic,
    reason = "an unsupported pairing stops the measurement; it is never skipped"
)]
fn unpairable<T>(result: Result<T, String>) -> T {
    result.unwrap_or_else(|error| panic!("PLAT-1029 bar D: {error}"))
}

/// Bar D's line for one variant.
pub(crate) fn render_bar_d(out: &mut String, label: &str, contrast: &PairedContrast) {
    let verdict = if !contrast.gateable() {
        format!(
            "not gateable (< {} decided pairs): no claim",
            metrics::MIN_DECIDED_PAIRS
        )
    } else if contrast.passes() {
        "HOLDS".to_owned()
    } else {
        "fails".to_owned()
    };
    let _ = writeln!(
        out,
        "\nBar D, {label}: {} succeeded, {} failed ({} of them abstentions), {} tied, \
         {} excluded (source already `yes`); one-sided sign test p = {} -> {verdict}",
        contrast.successes(),
        contrast.failures(),
        contrast.unanswered(),
        contrast.ties(),
        contrast.excluded_source_on_defect_side,
        contrast
            .p_value()
            .map_or_else(|| "n/a".to_owned(), |p| format!("{p:.4}")),
    );
}

/// Breaker rows against every other variant's `trace_correct` answer on the
/// same row (PLAT-1030's trace check), when one ran.
fn render_breaker_crosstab(out: &mut String, rows: &[Row], output: &RunOutput, variant: &Variant) {
    let label = variant.label();
    let by_id: BTreeMap<&str, &Row> = rows.iter().map(|row| (row.id.as_str(), row)).collect();
    let cross_check = variant.id == E1.id;
    let breaker: BTreeSet<&str> = output
        .results
        .iter()
        .filter(|result| result.variant == label)
        .filter_map(|result| {
            let row = by_id.get(result.row_id.as_str())?;
            let assessment = loud(assess_relation(row, &result.answered, cross_check));
            matches!(assessment.outcome, RelationOutcome::TraceSuspect { .. })
                .then_some(result.row_id.as_str())
        })
        .collect();
    let mut table: BTreeMap<(String, bool, String), usize> = BTreeMap::new();
    for result in &output.results {
        let Some(prediction) = result.predictions.get(TRACE_KEY) else {
            continue;
        };
        if !by_id.contains_key(result.row_id.as_str())
            || !output
                .results
                .iter()
                .any(|mine| mine.variant == label && mine.row_id == result.row_id)
        {
            continue;
        }
        let cell = (
            result.variant.clone(),
            breaker.contains(result.row_id.as_str()),
            prediction.answer.clone(),
        );
        *table.entry(cell).or_default() += 1;
    }
    if table.is_empty() {
        return;
    }
    let _ = writeln!(
        out,
        "\n{label} breaker vs `{TRACE_KEY}` (rows both ran on):\n\n\
         | Trace variant | Breaker tripped | `{TRACE_KEY}` answer | Rows |\n| --- | --- | --- | --- |"
    );
    for ((trace, tripped, answer), count) in table {
        let _ = writeln!(
            out,
            "| {trace} | {} | {answer} | {count} |",
            if tripped { "yes" } else { "no" }
        );
    }
}

/// PLAT-1029's report beside the shared one: E1's and E4's unit counts and
/// breaker, E2's tau sweep, and E3's gated curve over every E-variant in the
/// run. Empty when the run holds none of them.
pub(crate) fn render_diagnostics(rows: &[Row], output: &RunOutput) -> String {
    let ran = |variant: &Variant| {
        let label = variant.label();
        output.results.iter().any(|result| result.variant == label)
    };
    let ran_any = [E0, E0_RC, E1, E2, E4].iter().any(ran);
    if !ran_any {
        return String::new();
    }
    let mut out = String::new();
    let _ = writeln!(
        out,
        "\n## PLAT-1029 diagnostics (`{KEY}`, MP-241)\n\nRows labelled AGENT-LABELLED were \
         graded against agent-written truth."
    );
    for (variant, cross_check) in [(E1, true), (E4, false)] {
        if ran(&variant) {
            let counts = relation_counts(rows, output, &variant, cross_check);
            render_relation_counts(&mut out, &variant.label(), &counts);
            render_breaker_crosstab(&mut out, rows, output, &variant);
        }
    }
    if ran(&E2) {
        render_sweep(
            &mut out,
            &E2.label(),
            &scored(rows, output, &E2.label(), KEY),
        );
    }
    // E0 runs on RTC and E0-RC on RC, so their rows never overlap: together
    // they are the as-asked baseline in both modes.
    let mut baseline = scored(rows, output, &E0.label(), KEY);
    baseline.extend(scored(rows, output, &E0_RC.label(), KEY));
    for variant in [E0, E0_RC, E1, E2, E4] {
        if ran(&variant) {
            let label = variant.label();
            let graded = scored(rows, output, &label, KEY);
            render_answered(&mut out, &label, &graded);
            if variant.id != E0.id && variant.id != E0_RC.id {
                render_paired(&mut out, &label, &graded, &baseline);
            }
            let contrast = unpairable(bar_d(rows, output, &variant, &source_id));
            render_bar_d(&mut out, &label, &contrast);
            render_gate(&mut out, &label, &graded, &baseline);
        }
    }
    out
}
