// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Requirement-statement checks F0 and F1 (PLAT-1024 experiment 3). Dev
//! only; informational, gates nothing.
//!
//! # The unit
//!
//! A requirement's own statement: the SHALL sentence(s) of its statement
//! section, never its acceptance criteria. [`statement_unit`] extracts them;
//! a stakeholder need (`StR-`/`US-`) or a statement with no SHALL sentence has
//! no unit, is not asked, and is not labelled.
//!
//! # The variants
//!
//! | id | asks | derives |
//! | --- | --- | --- |
//! | `F0` | one holistic `noul`: is the statement a single, unambiguous, behaviour-level obligation? | `fr_statement_sound` = `P(yes) >= 0.5` |
//! | `F1` | one `noul` per named defect in [`CHECKS`] | each check at 0.5; `fr_statement_sound` by [`F1_COMBINER`] |
//!
//! # Truth
//!
//! Natural rows are one AGENT-LABELLED pass (`labels-fr-statement.json`,
//! kind `agent_single`). Mutants of kind [`MUTATION_KIND`] are dev-only and
//! carry by-construction truth: the injected check `yes` and
//! `fr_statement_sound` `no`.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Value, json};
use typesafe_sdk_questions::{Entry, NoulCriteria, Questions, noul_with};

use crate::eval_v2_support::corpus::{Row, Truth, TruthKind};
use crate::eval_v2_support::keys::{Mode, NO, YES};
use crate::eval_v2_support::variant::{
    Answered, Ask, Prediction, Predictions, RunOutput, Variant, request, whole_row,
};
use crate::eval_v2_support::variants::soundness::{
    BarD, PairOutcome, TAU, credit, defect_prediction, pair_outcome, required_noul,
};

/// The aggregate key: `yes` when the statement has none of the defects.
pub(crate) const SOUND: &str = "fr_statement_sound";
/// The mutation kind whose rows inject one statement defect.
pub(crate) const MUTATION_KIND: &str = "fr_statement";
/// F0's wire key.
pub(crate) const WELL_FORMED: &str = "well_formed";
/// The state field carrying the unit.
pub(crate) const STATEMENT_FIELD: &str = "statement";

/// One named statement defect.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Check {
    /// The corpus key, also the wire key. `yes` is the defect.
    pub(crate) key: &'static str,
    /// The question.
    pub(crate) instruction: &'static str,
    /// What `yes` (the defect) means.
    pub(crate) defect: &'static str,
    /// What `no` (clear) means.
    pub(crate) clear: &'static str,
}

impl Check {
    /// The labelling rule the corpus header states for this key.
    pub(crate) fn rule(&self) -> String {
        format!("yes: {} no: {}", self.defect, self.clear)
    }
}

/// Every instruction opens with this sentence.
macro_rules! judge {
    ($question:literal) => {
        concat!(
            "Judge the text in `statement`, read literally. It is a requirement's normative \
             statement: its SHALL sentence(s), without its acceptance criteria. ",
            $question
        )
    };
}

/// The three named defects. The `defect` and `clear` text is also the
/// labelling rule the natural rows were labelled to.
pub(crate) const CHECKS: [Check; 3] = [
    Check {
        key: "compound_obligation",
        instruction: judge!(
            "Does the statement impose two or more independent obligations, each of which could \
             be met while another is not?"
        ),
        defect: "Two or more obligations that could pass or fail separately: different actions \
                 or effects joined by 'and', several SHALL clauses that require different things, \
                 or steps of a procedure that could each be skipped.",
        clear: "One obligation. One action described together with its conditions, its inputs or \
                cases, the list of objects it applies to, the attributes of the one thing it \
                produces, its manner ('without', 'rather than') or its purpose or rationale \
                ('so that', 'because') is one obligation.",
    },
    Check {
        key: "multiple_readings",
        instruction: judge!(
            "Could a competent reader reasonably take the statement to require two different \
             behaviours?"
        ),
        defect: "A word or construction has two reasonable readings that require different \
                 observable behaviour: a pronoun or 'the X' with two possible referents, 'or' or \
                 'and/or' where it is unclear whether one, either or both is required, or a \
                 modifier such as 'only' whose scope could attach to two different parts.",
        clear: "Every reasonable reading requires the same behaviour. Vague or unmeasured wording \
                with one intended meaning, references to documents or terms defined elsewhere, \
                and long lists are not two readings.",
    },
    Check {
        key: "names_internal_symbol",
        instruction: judge!(
            "Does the statement name an identifier internal to the implementation (a function, \
             method, struct, class, trait, module or private field name) in place of the \
             observable behaviour it requires?"
        ),
        defect: "It names an implementation identifier (a function such as `parse_config`, a \
                 type such as `ConfigLoader`, a trait or a module path) as the subject or object \
                 of the obligation, so the obligation is about the implementation's inside \
                 rather than what a user or caller observes.",
        clear: "It names only observable things: commands, flags, files and paths, published \
                formats and schema record types, report fields, error identifiers, package or \
                tool names, and other requirements.",
    },
];

/// Every key F1 is graded on: [`SOUND`] and each check.
pub(crate) const GRADES: [&str; CHECKS.len() + 1] = {
    let [a, b, c] = CHECKS;
    [SOUND, a.key, b.key, c.key]
};

/// F0's holistic question.
pub(crate) const HOLISTIC: &str =
    judge!("Is this statement well-formed: a single, unambiguous, behaviour-level obligation?");

// ---------------------------------------------------------------------------
// The unit
// ---------------------------------------------------------------------------

static SHALL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bshall\b").unwrap_or_else(|error| unreachable!("a literal regex: {error}"))
});

/// A sentence end: terminal punctuation, optional closers, whitespace, then
/// the next sentence's first character (a capital, a backtick or emphasis).
static SENTENCE_END: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"[.!?][)*"]*\s+[A-Z`*]"#)
        .unwrap_or_else(|error| unreachable!("a literal regex: {error}"))
});

static LIST_ITEM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:[-*]|\d+\.)\s")
        .unwrap_or_else(|error| unreachable!("a literal regex: {error}"))
});

/// The prose blocks of a markdown section, each joined onto one line:
/// paragraphs and list items. Headings, tables, block quotes and fenced
/// blocks are dropped.
pub(crate) fn prose_blocks(text: &str) -> Vec<String> {
    let mut blocks: Vec<Vec<&str>> = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    let mut fenced = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("```") {
            fenced = !fenced;
            blocks.push(std::mem::take(&mut current));
            continue;
        }
        if fenced {
            continue;
        }
        if line.is_empty() || line.starts_with(['#', '|', '>']) {
            blocks.push(std::mem::take(&mut current));
            continue;
        }
        if LIST_ITEM.is_match(line) {
            blocks.push(std::mem::take(&mut current));
        }
        current.push(line);
    }
    blocks.push(current);
    blocks
        .into_iter()
        .filter(|block| !block.is_empty())
        .map(|block| block.join(" "))
        .collect()
}

/// A block's sentences, trimmed, in order.
pub(crate) fn sentences(block: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    for found in SENTENCE_END.find_iter(block) {
        // The match ends on the next sentence's first character, which is
        // one ASCII byte.
        let end = found.end() - 1;
        out.push(block.get(start..end).unwrap_or_default().trim());
        start = end;
    }
    out.push(block.get(start..).unwrap_or_default().trim());
    out.retain(|sentence| !sentence.is_empty());
    out
}

/// The SHALL sentences of a statement section, one per line, or `None` when
/// it has none.
pub(crate) fn shall_sentences(statement: &str) -> Option<String> {
    let kept: Vec<String> = prose_blocks(statement)
        .iter()
        .flat_map(|block| {
            sentences(block)
                .into_iter()
                .filter(|sentence| SHALL.is_match(sentence))
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect();
    (!kept.is_empty()).then(|| kept.join("\n"))
}

/// The row's unit: its requirement's SHALL sentences. `None` for a
/// stakeholder need or a statement with no SHALL sentence.
pub(crate) fn statement_unit(row: &Row) -> Option<String> {
    let id = &row.requirement.fr_id;
    if id.starts_with("StR-") || id.starts_with("US-") {
        return None;
    }
    shall_sentences(&row.requirement.statement)
}

// ---------------------------------------------------------------------------
// Asks and derivation
// ---------------------------------------------------------------------------

/// The state: the requirement's id and its unit.
pub(crate) fn statement_state(row: &Row, unit: &str) -> Value {
    json!({
        "requirement_id": row.requirement.fr_id,
        STATEMENT_FIELD: unit,
    })
}

fn noul(instruction: &str, yes: &str, no: &str) -> typesafe_sdk_questions::Question {
    noul_with(
        instruction,
        NoulCriteria {
            yes: Some(Entry::from(yes)),
            no: Some(Entry::from(no)),
        },
    )
}

/// F1 v2's `names_internal_symbol` question (POST HOC, chosen after dev run
/// 1). On dev, v1 fired on 6 of 14 statements labelled sound, and in all six
/// the only candidates were names a caller sees: package and crate names
/// (`ts-plugin-kit`, `quoin-core`, `engineering-assurance`), a report format
/// (`cli-agent-evals`) and a requirement id (FR-033). The labelling rule
/// already lists those as clear; v2 moves that list into the question. The
/// yes/no criteria are unchanged.
pub(crate) const INTERNAL_SYMBOL_V2: &str = judge!(
    "Does the statement name an identifier from inside the implementation's source (a function \
     or method name, a struct, class, enum or trait name, a module path, or a private field) as \
     what the obligation is about, in place of the observable behaviour it requires? Names a \
     user or caller sees are not such identifiers: packages, crates, binaries and tools; \
     commands and flags; files, paths and formats; published schema record types and report \
     fields; and requirement or document ids such as FR-033."
);

fn f1_questions() -> Questions {
    CHECKS
        .iter()
        .map(|check| {
            let instruction = if check.key == "names_internal_symbol" {
                INTERNAL_SYMBOL_V2
            } else {
                check.instruction
            };
            (
                check.key.to_owned(),
                noul(instruction, check.defect, check.clear),
            )
        })
        .collect()
}

fn f0_questions() -> Questions {
    std::iter::once((
        WELL_FORMED.to_owned(),
        noul(
            HOLISTIC,
            "The statement is a single, unambiguous, behaviour-level obligation.",
            "It is not.",
        ),
    ))
    .collect()
}

fn asks_with(row: &Row, questions: fn() -> Questions) -> Vec<Ask> {
    statement_unit(row)
        .map(|unit| Ask {
            unit: None,
            request: request(statement_state(row, &unit), questions()),
        })
        .into_iter()
        .collect()
}

fn f1_asks(row: &Row) -> Vec<Ask> {
    asks_with(row, f1_questions)
}

fn f0_asks(row: &Row) -> Vec<Ask> {
    asks_with(row, f0_questions)
}

/// How F1's checks combine into [`SOUND`]: the statement is flagged when at
/// least `at_least` checks are at or above `tau`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Combiner {
    /// How many checks must fire.
    pub(crate) at_least: usize,
    /// The threshold a check fires at.
    pub(crate) tau: f64,
}

/// F1's combiner, fixed before the first live call: any check at 0.5.
pub(crate) const F1_COMBINER: Combiner = Combiner {
    at_least: 1,
    tau: TAU,
};

/// The combiners every report recomputes from the same answers.
pub(crate) const COMBINERS: [(&str, Combiner); 3] = [
    ("any >= 0.5", F1_COMBINER),
    (
        ">= 2 >= 0.5",
        Combiner {
            at_least: 2,
            tau: TAU,
        },
    ),
    (
        "any >= 0.7",
        Combiner {
            at_least: 1,
            tau: 0.7,
        },
    ),
];

impl Combiner {
    /// The row's defect score: the `at_least`-th highest check probability,
    /// or 0 when there are fewer checks.
    pub(crate) fn score(&self, probabilities: &[f64]) -> f64 {
        let mut sorted = probabilities.to_vec();
        sorted.sort_by(|a, b| b.total_cmp(a));
        self.at_least
            .checked_sub(1)
            .and_then(|index| sorted.get(index))
            .copied()
            .unwrap_or(0.0)
    }

    /// Whether the statement is flagged.
    pub(crate) fn flags(&self, probabilities: &[f64]) -> bool {
        self.score(probabilities) >= self.tau
    }

    /// [`SOUND`] as a graded answer: `no` when flagged. The ordinal is
    /// `1 - score`, higher meaning more likely sound; the confidence is the
    /// score for `no` and `1 - score` for `yes`.
    pub(crate) fn sound(&self, probabilities: &[f64]) -> Prediction {
        let score = self.score(probabilities);
        let flagged = score >= self.tau;
        Prediction {
            answer: if flagged { NO } else { YES }.to_owned(),
            confidence: Some(if flagged { score } else { 1.0 - score }),
            ordinal: Some(1.0 - score),
        }
    }
}

/// F1's graded answers from one defect probability per check, in [`CHECKS`]
/// order.
pub(crate) fn derive_checks(probabilities: &[f64; CHECKS.len()]) -> Predictions {
    let mut out: Predictions = CHECKS
        .iter()
        .zip(probabilities)
        .map(|(check, p)| (check.key, defect_prediction(*p)))
        .collect();
    out.insert(SOUND, F1_COMBINER.sound(probabilities));
    out
}

/// Each check's probability on a row F1 answered, in [`CHECKS`] order.
pub(crate) fn f1_probabilities(row: &Row, answered: &[Answered]) -> Option<[f64; CHECKS.len()]> {
    if answered.is_empty() {
        return None;
    }
    let answers = whole_row(answered);
    Some(CHECKS.map(|check| required_noul(&row.id, &answers, check.key)))
}

fn f1_derive(row: &Row, answered: &[Answered]) -> Predictions {
    f1_probabilities(row, answered)
        .map(|p| derive_checks(&p))
        .unwrap_or_default()
}

/// F0's `P(well-formed)` on a row it answered.
pub(crate) fn f0_probability(row: &Row, answered: &[Answered]) -> Option<f64> {
    (!answered.is_empty()).then(|| required_noul(&row.id, &whole_row(answered), WELL_FORMED))
}

fn f0_derive(row: &Row, answered: &[Answered]) -> Predictions {
    f0_probability(row, answered)
        .map(|p| {
            let sound = p >= TAU;
            Predictions::from([(
                SOUND,
                Prediction {
                    answer: if sound { YES } else { NO }.to_owned(),
                    confidence: Some(if sound { p } else { 1.0 - p }),
                    ordinal: Some(p),
                },
            )])
        })
        .unwrap_or_default()
}

/// The holistic baseline.
pub(crate) const F0: Variant = Variant {
    id: "F0",
    version: 1,
    summary: "requirement statement: one holistic noul (single, unambiguous, behaviour-level obligation?)",
    modes: &Mode::ALL,
    references: &[],
    grades: &[SOUND],
    asks: f0_asks,
    derive: f0_derive,
};

/// The battery: one noul per named defect, combined in code.
pub(crate) const F1: Variant = Variant {
    id: "F1",
    // v2 (post hoc, after dev run 1): the `names_internal_symbol` question is
    // INTERNAL_SYMBOL_V2.
    version: 2,
    summary: "requirement statement: three named defect nouls (names_internal_symbol excludes caller-visible names); fr_statement_sound = no check at or above 0.5",
    modes: &Mode::ALL,
    references: &[],
    grades: &GRADES,
    asks: f1_asks,
    derive: f1_derive,
};

// ---------------------------------------------------------------------------
// The report (informational; every combiner past `any >= 0.5` is post hoc)
// ---------------------------------------------------------------------------

/// Each answered row's defect score under one rule; an unanswered row is absent.
type Scores<'a> = BTreeMap<&'a str, f64>;

/// Whether `row` is a requirement-statement mutant: it carries truth for the
/// statement keys only.
pub(crate) fn is_statement_mutant(row: &Row) -> bool {
    row.mutation
        .as_ref()
        .is_some_and(|mutation| mutation.kind == MUTATION_KIND)
}

/// The check a statement mutant injected: its one by-construction `yes`.
pub(crate) fn injected(row: &Row) -> Option<&'static str> {
    row.mutation
        .as_ref()
        .filter(|mutation| mutation.kind == MUTATION_KIND)?;
    CHECKS.iter().map(|check| check.key).find(|key| {
        row.truth.get(*key).is_some_and(|truth| {
            truth.kind == TruthKind::ByConstruction && truth.answer.label() == YES
        })
    })
}

fn primary(row: &Row, key: &str) -> Option<String> {
    row.truth.get(key).map(|truth| truth.answer.label())
}

/// One statement mutant paired with its source: the source id, and the
/// pair's outcome with whether it was an abstention (a failure because a side
/// went unanswered), or `None` when the source is already on the defect side
/// (dropped).
type Paired<'a> = (&'a str, Option<(PairOutcome, bool)>);

/// Every statement mutant whose injected check is in `checks`, paired with
/// its source, in row order: the pair's scores, shifted so `tau` is the
/// shared 0.5 crossing, through the shared pair rule. `excluded_key` names
/// the truth whose `yes` (for a check) or `no` (for [`SOUND`]) on the source
/// already puts it on the defect side, so the pair is dropped. An unanswered
/// side is a failure.
///
/// # Panics
/// When a mutant names no source, or one missing from `rows`, a mutant
/// itself, or in another split or mode: a broken pair link must stop the
/// run, not shrink bar D's denominator (as `soundness::pairs`, MP-243).
#[allow(
    clippy::panic,
    reason = "a broken pair link must stop the run, not shrink bar D's denominator (MP-243)"
)]
fn paired<'a>(
    rows: &'a [Row],
    scores: &Scores<'_>,
    tau: f64,
    checks: &[&str],
    excluded_key: &str,
) -> Vec<Paired<'a>> {
    let by_id: BTreeMap<&str, &Row> = rows.iter().map(|row| (row.id.as_str(), row)).collect();
    let defect_label = if excluded_key == SOUND { NO } else { YES };
    let shift = TAU - tau;
    let score = |row: &Row| scores.get(row.id.as_str()).map(|s| s + shift);
    rows.iter()
        .filter(|mutant| injected(mutant).is_some_and(|key| checks.contains(&key)))
        .map(|mutant| {
            let id = &mutant.id;
            let source_id = mutant
                .mutation
                .as_ref()
                .and_then(|m| m.source_id.as_deref())
                .unwrap_or_else(|| panic!("{id}: statement mutant has no mutation.source_id"));
            let source = by_id
                .get(source_id)
                .copied()
                .unwrap_or_else(|| panic!("{id}: source {source_id} is not among this run's rows"));
            assert!(
                source.mutation.is_none(),
                "{id}: source {source_id} is itself a mutant"
            );
            assert!(
                source.split == mutant.split && source.mode == mutant.mode,
                "{id}: source {source_id} is {}/{}, the mutant {}/{}",
                source.split.as_str(),
                source.mode.as_str(),
                mutant.split.as_str(),
                mutant.mode.as_str()
            );
            if primary(source, excluded_key).as_deref() == Some(defect_label) {
                return (source_id, None);
            }
            let outcome = score(source)
                .zip(score(mutant))
                .map_or((PairOutcome::Failure, true), |(s, m)| {
                    (pair_outcome(s, m), false)
                });
            (source_id, Some(outcome))
        })
        .collect()
}

fn tally(outcomes: impl Iterator<Item = Option<(PairOutcome, bool)>>) -> BarD {
    let mut tally = BarD::default();
    for outcome in outcomes {
        tally.pairs += 1;
        let Some((outcome, abstained)) = outcome else {
            tally.source_already_yes += 1;
            continue;
        };
        tally.abstained += usize::from(abstained);
        match outcome {
            PairOutcome::Success => tally.successes += 1,
            PairOutcome::Failure => tally.failures += 1,
            PairOutcome::Tie => tally.ties += 1,
        }
    }
    tally
}

/// Bar D over every pair ([`paired`]). Pairs sharing a source are not
/// independent, so this line is descriptive; [`bar_d_per_source`] gates.
///
/// # Panics
/// As [`paired`].
pub(crate) fn bar_d(
    rows: &[Row],
    scores: &Scores<'_>,
    tau: f64,
    checks: &[&str],
    excluded_key: &str,
) -> BarD {
    tally(
        paired(rows, scores, tau, checks, excluded_key)
            .into_iter()
            .map(|(_, outcome)| outcome),
    )
}

/// Bar D with one pair per source: each source's majority outcome over its
/// kept pairs (success when successes outnumber failures, failure when
/// failures outnumber successes, else a tie); a source whose every pair was
/// dropped counts as dropped. Sources are independent, so this line gates.
///
/// # Panics
/// As [`paired`].
pub(crate) fn bar_d_per_source(
    rows: &[Row],
    scores: &Scores<'_>,
    tau: f64,
    checks: &[&str],
    excluded_key: &str,
) -> BarD {
    let mut by_source: BTreeMap<&str, Vec<Option<(PairOutcome, bool)>>> = BTreeMap::new();
    for (source, outcome) in paired(rows, scores, tau, checks, excluded_key) {
        by_source.entry(source).or_default().push(outcome);
    }
    tally(by_source.into_values().map(|outcomes| {
        let kept: Vec<(PairOutcome, bool)> = outcomes.into_iter().flatten().collect();
        if kept.is_empty() {
            return None;
        }
        let count = |wanted: PairOutcome| kept.iter().filter(|(o, _)| *o == wanted).count();
        let (wins, losses) = (count(PairOutcome::Success), count(PairOutcome::Failure));
        let all_abstained = kept.iter().all(|(_, abstained)| *abstained);
        Some(match wins.cmp(&losses) {
            std::cmp::Ordering::Greater => (PairOutcome::Success, false),
            std::cmp::Ordering::Less => (PairOutcome::Failure, all_abstained),
            std::cmp::Ordering::Equal => (PairOutcome::Tie, false),
        })
    }))
}

fn pct(n: usize, d: usize) -> String {
    if d == 0 {
        "n/a".to_owned()
    } else {
        let (n, d) = (
            f64::from(u32::try_from(n).unwrap_or(u32::MAX)),
            f64::from(u32::try_from(d).unwrap_or(u32::MAX)),
        );
        format!("{n:.0}/{d:.0} {:.0}%", 100.0 * n / d)
    }
}

fn show_d(d: &BarD) -> String {
    format!(
        "{}-{}-{} (p = {:.4}; {}; {} dropped, source already defective{})",
        d.successes,
        d.failures,
        d.ties,
        d.p_value(),
        if d.gateable() {
            "gateable"
        } else {
            "NOT gateable"
        },
        d.source_already_yes,
        if d.abstained > 0 {
            format!("; {} abstained", d.abstained)
        } else {
            String::new()
        }
    )
}

/// One rule's row over the report's rows.
struct RuleLine<'a> {
    name: String,
    scores: Scores<'a>,
    tau: f64,
}

/// The rows the report reads: labelled with [`SOUND`].
fn labelled(rows: &[Row]) -> Vec<&Row> {
    rows.iter()
        .filter(|row| row.truth.contains_key(SOUND))
        .collect()
}

fn flagged(line: &RuleLine<'_>, row: &Row) -> Option<bool> {
    line.scores.get(row.id.as_str()).map(|s| *s >= line.tau)
}

/// `(credit, best constant credit, rows)` for [`SOUND`] over `rows`.
fn accuracy(line: &RuleLine<'_>, rows: &[&Row]) -> (f64, f64, usize) {
    let answer = |row: &Row| flagged(line, row).map(|flag| if flag { NO } else { YES });
    let truths: Vec<(&Row, &Truth)> = rows
        .iter()
        .filter_map(|row| row.truth.get(SOUND).map(|truth| (*row, truth)))
        .collect();
    let ours = truths
        .iter()
        .map(|(row, truth)| credit(truth, answer(row)))
        .sum();
    let constant = |label: &str| -> f64 {
        truths
            .iter()
            .map(|(_, truth)| credit(truth, Some(label)))
            .sum()
    };
    (ours, constant(YES).max(constant(NO)), truths.len())
}

fn render_rule(out: &mut String, rows: &[Row], line: &RuleLine<'_>) {
    let all = labelled(rows);
    let natural: Vec<&Row> = all
        .iter()
        .copied()
        .filter(|row| row.mutation.is_none())
        .collect();
    let sound: Vec<&Row> = natural
        .iter()
        .copied()
        .filter(|row| primary(row, SOUND).as_deref() == Some(YES))
        .collect();
    let defective: Vec<&Row> = natural
        .iter()
        .copied()
        .filter(|row| primary(row, SOUND).as_deref() == Some(NO))
        .collect();
    let cleared = sound
        .iter()
        .filter(|row| flagged(line, row) == Some(false))
        .count();
    let caught = |rows: &[&Row]| {
        rows.iter()
            .filter(|row| flagged(line, row) == Some(true))
            .count()
    };
    let mut per_kind = String::new();
    for check in &CHECKS {
        let mutants: Vec<&Row> = all
            .iter()
            .copied()
            .filter(|row| injected(row) == Some(check.key))
            .collect();
        let _ = write!(per_kind, " {} | ", pct(caught(&mutants), mutants.len()));
    }
    let (ours_n, constant_n, n_n) = accuracy(line, &natural);
    let (ours_a, constant_a, n_a) = accuracy(line, &all);
    let keys: Vec<&str> = CHECKS.iter().map(|check| check.key).collect();
    let d = bar_d(rows, &line.scores, line.tau, &keys, SOUND);
    let per_source = bar_d_per_source(rows, &line.scores, line.tau, &keys, SOUND);
    let _ = writeln!(
        out,
        "| {} | {} | {} |{per_kind} {ours_n:.1} vs {constant_n:.1} of {n_n} | {ours_a:.1} vs \
         {constant_a:.1} of {n_a} | {} | {} |",
        line.name,
        pct(cleared, sound.len()),
        pct(caught(&defective), defective.len()),
        show_d(&per_source),
        show_d(&d),
    );
}

const RULE_HEADER: &str = "| Rule | Sound-row recall (natural) | Defect recall (natural) | \
    compound_obligation mutants | multiple_readings mutants | names_internal_symbol mutants | \
    Credit vs best constant (natural) | Credit vs best constant (natural + mutants) | Bar D \
    aggregate, one pair per source (W-L-T; GATES) | Bar D aggregate, every pair (W-L-T; \
    descriptive, pairs share sources) |\n| --- | --- | --- | --- | --- | --- | --- | --- | --- | \
    --- |";

/// Each row's check probabilities for F1 labelled `label`.
fn f1_table<'a>(rows: &'a [Row], output: &RunOutput, label: &str) -> BTreeMap<&'a str, Vec<f64>> {
    let by_id: BTreeMap<&str, &Row> = rows.iter().map(|row| (row.id.as_str(), row)).collect();
    output
        .results
        .iter()
        .filter(|result| result.variant == label)
        .filter_map(|result| {
            let row = by_id.get(result.row_id.as_str())?;
            f1_probabilities(row, &result.answered).map(|p| (row.id.as_str(), p.to_vec()))
        })
        .collect()
}

fn f0_scores<'a>(rows: &'a [Row], output: &RunOutput, label: &str) -> Scores<'a> {
    let by_id: BTreeMap<&str, &Row> = rows.iter().map(|row| (row.id.as_str(), row)).collect();
    output
        .results
        .iter()
        .filter(|result| result.variant == label)
        .filter_map(|result| {
            let row = by_id.get(result.row_id.as_str())?;
            f0_probability(row, &result.answered).map(|p| (row.id.as_str(), 1.0 - p))
        })
        .collect()
}

fn render_checks(out: &mut String, rows: &[Row], table: &BTreeMap<&str, Vec<f64>>) {
    let all = labelled(rows);
    let natural = |label: &str| -> Vec<&Row> {
        all.iter()
            .copied()
            .filter(|row| row.mutation.is_none() && primary(row, SOUND).as_deref() == Some(label))
            .collect()
    };
    let (sound, defective) = (natural(YES), natural(NO));
    let _ = writeln!(
        out,
        "\n| Check | Fires on sound (natural) | Fires on defective (natural) | Natural labelled \
         yes on it | Fires on its own mutants | Fires on other mutants | Bar D (its own \
         mutants, W-L-T) |\n| --- | --- | --- | --- | --- | --- | --- |"
    );
    for (index, check) in CHECKS.iter().enumerate() {
        let fires = |row: &Row| {
            table
                .get(row.id.as_str())
                .and_then(|p| p.get(index))
                .is_some_and(|p| *p >= TAU)
        };
        let count = |rows: &[&Row]| rows.iter().filter(|row| fires(row)).count();
        let own: Vec<&Row> = all
            .iter()
            .copied()
            .filter(|row| injected(row) == Some(check.key))
            .collect();
        let other: Vec<&Row> = all
            .iter()
            .copied()
            .filter(|row| injected(row).is_some_and(|key| key != check.key))
            .collect();
        let labelled_yes = all
            .iter()
            .filter(|row| row.mutation.is_none() && primary(row, check.key).as_deref() == Some(YES))
            .count();
        let scores: Scores<'_> = table
            .iter()
            .filter_map(|(id, p)| p.get(index).map(|p| (*id, *p)))
            .collect();
        let d = bar_d(rows, &scores, TAU, &[check.key], check.key);
        let _ = writeln!(
            out,
            "| {} | {} | {} | {labelled_yes} | {} | {} | {} |",
            check.key,
            pct(count(&sound), sound.len()),
            pct(count(&defective), defective.len()),
            pct(count(&own), own.len()),
            pct(count(&other), other.len()),
            show_d(&d),
        );
    }
}

/// The report for every F variant in `variants`: F0 and F1's pre-registered
/// rule, then F1's per-check table and its combiners recomputed from the same
/// answers (post hoc). Empty when no F variant ran.
pub(crate) fn render(rows: &[Row], output: &RunOutput, variants: &[&Variant]) -> String {
    let mut out = String::new();
    let ran = |id: &str| variants.iter().find(|variant| variant.id == id);
    if ran("F0").is_none() && ran("F1").is_none() {
        return out;
    }
    let all = labelled(rows);
    let _ = writeln!(
        out,
        "\n## Requirement statement ({SOUND}; dev only, informational)\n\n{} labelled rows: {} \
         natural (AGENT-LABELLED, single pass), {} statement mutants (by construction). Credit \
         counts an alternative reading as half. Bar D: {MUTATION_KIND} mutant vs its source, the \
         shared pair rule (cross the rule's threshold, move at least 0.10).\n\n{RULE_HEADER}",
        all.len(),
        all.iter().filter(|row| row.mutation.is_none()).count(),
        all.iter().filter(|row| row.mutation.is_some()).count(),
    );
    if let Some(f0) = ran("F0") {
        let label = f0.label();
        render_rule(
            &mut out,
            rows,
            &RuleLine {
                name: format!("{label} holistic P(no) >= 0.5"),
                scores: f0_scores(rows, output, &label),
                tau: TAU,
            },
        );
    }
    let Some(f1) = ran("F1") else {
        return out;
    };
    let label = f1.label();
    let table = f1_table(rows, output, &label);
    for (name, combiner) in COMBINERS {
        let post_hoc = match (combiner == F1_COMBINER, f1.version) {
            (true, 1) => " (pre-registered)".to_owned(),
            (true, version) => {
                format!(" (pre-registered combiner; POST HOC wording, v{version})")
            }
            (false, _) => " (POST HOC)".to_owned(),
        };
        render_rule(
            &mut out,
            rows,
            &RuleLine {
                name: format!("{label} {name}{post_hoc}"),
                scores: table
                    .iter()
                    .map(|(id, p)| (*id, combiner.score(p)))
                    .collect(),
                tau: combiner.tau,
            },
        );
    }
    render_checks(&mut out, rows, &table);
    out
}

/// One line per F1 row: every check's probability, the row's truth and its
/// unit, for reading flagged rows by hand.
pub(crate) fn dump(rows: &[Row], output: &RunOutput, variants: &[&Variant]) -> Vec<String> {
    let f0 = variants
        .iter()
        .find(|variant| variant.id == "F0")
        .map(|variant| f0_scores(rows, output, &variant.label()))
        .unwrap_or_default();
    let Some(f1) = variants.iter().find(|variant| variant.id == "F1") else {
        return Vec::new();
    };
    let table = f1_table(rows, output, &f1.label());
    labelled(rows)
        .into_iter()
        .map(|row| {
            let truth: BTreeMap<&str, Value> = row
                .truth
                .iter()
                .map(|(key, truth)| {
                    (
                        key.as_str(),
                        json!({
                            "answer": truth.answer.label(),
                            "alternatives": truth.alternatives.iter().map(crate::eval_v2_support::corpus::TruthAnswer::label).collect::<Vec<_>>(),
                            "rationale": truth.rationale,
                        }),
                    )
                })
                .collect();
            let p: BTreeMap<&str, f64> = table
                .get(row.id.as_str())
                .map(|p| {
                    CHECKS
                        .iter()
                        .zip(p)
                        .map(|(check, p)| (check.key, *p))
                        .collect()
                })
                .unwrap_or_default();
            json!({
                "id": row.id,
                "mode": row.mode.as_str(),
                "fr_id": row.requirement.fr_id,
                "natural": row.mutation.is_none(),
                "mutation": row.mutation.as_ref().map(|m| json!({"id": m.id, "source_id": m.source_id, "description": m.description})),
                "injected": injected(row),
                "sound": primary(row, SOUND),
                "f1": p,
                "f0_p_defect": f0.get(row.id.as_str()),
                "unit": statement_unit(row),
                "truth": truth,
            })
            .to_string()
        })
        .collect()
}
