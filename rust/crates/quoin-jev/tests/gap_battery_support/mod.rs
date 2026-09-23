// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Ground truth for the OTHER six questions in PLAT-839's gap-analysis
//! battery, and the maths that grades them.
//!
//! `../live_gap_semantic.rs` graded exactly one of the seven questions the
//! lens asks — `assertion_vacuous` — against mutation truth (MP-232). The
//! other six were asked in the same live calls and never graded against
//! anything. This module builds the ground truth they need; the bars are
//! pre-registered in `../live_gap_battery.rs`'s module doc.
//!
//! # Three kinds of ground truth, named as such
//!
//! **Mechanical, targeted mutation.** `../fixtures/gap-battery-mutants.json`
//! records, per mutation: the exact source substitution, which owning test
//! was rerun, and whether that test failed. Two classes:
//!
//! - a **violating** mutant contradicts the SPECIFIC behaviour the triple's
//!   requirement states (not the coarse "stub the whole symbol to a default"
//!   of MP-232). A violating mutant the owning test caught is mechanical
//!   proof of two facts at once: the test asserts the requirement's stated
//!   behaviour — `test_asserts_intent`-truth for the unmutated triple — and
//!   the mutated body does NOT implement it, which is
//!   `code_implements_intent`-truth for the mutant row.
//! - an **additive** mutant adds behaviour no requirement in the triple
//!   states, leaving the stated behaviour intact. An additive mutant the
//!   owning test did NOT catch is the `code_exceeds_requirement` positive
//!   class.
//!
//! **Mechanical, static check on the test source.**
//! [`asserts_only_its_own_setup`] reads the test body and answers
//! `tests_only_its_own_mock` from the text, with no model in the loop. Its
//! rule is stated on the function and was fixed before the first live call.
//!
//! **Constructed rows, labelled as constructed.**
//! `../fixtures/gap-battery-mockonly.json` holds eight test bodies written
//! for this evaluation — the same convention `GAP-28`/`GAP-29` already use —
//! because the static check finds ZERO mock-only tests among the corpus's 29
//! real ones, which leaves a constant predictor scoring 100% and the question
//! unmeasurable. They are not compiled: the static check reads source text,
//! so compiling them would not change their label. Their `test_file` says
//! `CONSTRUCTED` and this module never presents them as repository tests.
//!
//! # What has no ground truth here
//!
//! `severity` gets no label. There is no mechanical fact a severity rubric
//! point corresponds to, and this module does not invent one. What it does
//! measure is *discrimination*: over the paired (original, violating-mutant)
//! rows, how often the mutant — a defect the compiler and test runner
//! confirm — is scored MORE severe than the original. A constant answer wins
//! no pairs at all, so the null is a coin flip, and that is stated as the
//! bar rather than dressed up as accuracy.

#![allow(
    dead_code,
    reason = "each test binary links this module separately, so items only one binary uses read as dead in the other"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde::Deserialize;
use typesafe_sdk_answers::{Answer, SystemOneResponse};

use crate::gap_semantic_support::{BinaryStats, GapTriple, corpus, percent};

/// The targeted-mutation log, compiled in so the fixture and this grader
/// cannot drift.
pub(crate) const MUTANTS: &str = include_str!("../fixtures/gap-battery-mutants.json");
/// The constructed mock-only rows, compiled in for the same reason.
pub(crate) const MOCKONLY: &str = include_str!("../fixtures/gap-battery-mockonly.json");
/// Targeted mutants added for `FullBatteryV1` (PLAT-979), kept apart from
/// [`MUTANTS`] so the v0 battery's recorded population does not change under
/// it on a rerun.
pub(crate) const MUTANTS_V1_ADDITIONS: &str =
    include_str!("../fixtures/gap-battery-mutants-v1-additions.json");

/// Which fact a mutation was built to establish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MutantClass {
    /// Contradicts the specific behaviour the requirement states.
    Violating,
    /// Adds behaviour no requirement in the triple states.
    Additive,
}

/// One targeted mutation, its source substitution and its test outcome.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Mutant {
    /// The triple this mutation was applied to, e.g. `GAP-17`.
    pub(crate) base_id: String,
    pub(crate) class: MutantClass,
    /// Why this substitution violates (or exceeds) the stated requirement.
    pub(crate) rationale: String,
    /// The repository file the substitution was applied to.
    pub(crate) file: String,
    /// The exact text replaced, and its replacement — the whole mutation.
    pub(crate) old: String,
    pub(crate) new: String,
    /// The one owning test that was rerun.
    pub(crate) test: String,
    /// Whether that test FAILED under the mutation.
    pub(crate) killed_by_owning_test: bool,
    /// The triple's `symbol_body` with the same substitution applied — what
    /// the lens is shown for a mutant row.
    pub(crate) mutated_symbol_body: String,
}

impl Mutant {
    /// This mutant's row id, e.g. `GAP-17/violating`.
    pub(crate) fn id(&self) -> String {
        let class = match self.class {
            MutantClass::Violating => "violating",
            MutantClass::Additive => "additive",
        };
        format!("{}/{class}", self.base_id)
    }
}

/// Parses the compiled-in mutation log.
///
/// # Panics
/// If the fixture stops matching this shape.
pub(crate) fn mutants() -> Vec<Mutant> {
    serde_json::from_str(MUTANTS).expect("the targeted-mutation log parses")
}

/// Parses the mutants added for `FullBatteryV1`.
///
/// # Panics
/// If the fixture stops matching this shape.
pub(crate) fn mutants_v1_additions() -> Vec<Mutant> {
    serde_json::from_str(MUTANTS_V1_ADDITIONS).expect("the v1 mutant additions parse")
}

/// Does this mutant contradict behaviour its triple's CITED requirement
/// states, as MP-233's P1 requires of a violating mutant?
///
/// Read off the mutant's own rationale: it must name the requirement the
/// triple is traced to. Four v0 mutants (`GAP-09`, `GAP-12`, `GAP-20`,
/// `GAP-22`) do not. Each contradicts behaviour the covered symbol documents,
/// not the FR-101 acceptance criterion the test is tagged with, and the
/// corpus's own `note` on each, written before any live call, records the tag
/// as mismatched to the test. So the owning test failing under such a mutant
/// shows that the test asserts the symbol's behaviour. It does not show that
/// the test asserts the requirement's, which is what `test_asserts_intent`
/// asks.
pub(crate) fn contradicts_cited_requirement(triples: &[GapTriple], mutant: &Mutant) -> bool {
    mutant
        .rationale
        .contains(base_of(triples, mutant).fr_id.as_str())
}

/// Parses the compiled-in constructed mock-only rows.
///
/// # Panics
/// If the fixture stops matching this shape.
pub(crate) fn constructed_mock_only() -> Vec<GapTriple> {
    serde_json::from_str(MOCKONLY).expect("the constructed mock-only rows parse")
}

/// The triple a mutant belongs to.
///
/// # Panics
/// If the mutation log names a triple the corpus does not carry.
pub(crate) fn base_of(triples: &[GapTriple], mutant: &Mutant) -> GapTriple {
    triples
        .iter()
        .find(|triple| triple.id == mutant.base_id)
        .unwrap_or_else(|| {
            panic!(
                "the mutation log names {}, absent from the corpus",
                mutant.base_id
            )
        })
        .clone()
}

/// The same triple with the mutant's body in place of the shipped one.
pub(crate) fn mutated(triples: &[GapTriple], mutant: &Mutant) -> GapTriple {
    let mut triple = base_of(triples, mutant);
    triple.id = mutant.id();
    triple.symbol_body.clone_from(&mutant.mutated_symbol_body);
    triple
}

// ---------------------------------------------------------------------------
// The static check: `tests_only_its_own_mock`
// ---------------------------------------------------------------------------

/// Does this test assert ONLY against values it configured itself?
///
/// The rule, fixed before the first live call:
///
/// 1. A test with no assertion at all answers **no** — it asserts nothing,
///    which is `assertion_vacuous`'s finding, not this one's.
/// 2. Otherwise, take every assertion's EXPECTED operand: the second
///    top-level argument of `assert_eq!`/`assert_ne!`/`assert_matches!`, and
///    the first (and only) condition of `assert!`.
/// 3. An expected operand carrying a `::`-qualified path is an INDEPENDENT
///    oracle: a constant or enum variant reached from the code under test is
///    not something the test configured.
/// 4. A literal (string, integer, `true`, `false`) in an expected operand is
///    the test's own configuration only if that same literal also appears in
///    the test's SETUP — the body with the assertion calls removed. A literal
///    that appears only in the assertion is an independent expected value.
/// 5. The answer is **yes** only when no assertion trips 3 or 4.
///
/// Returns the answer and the reason, so a reader can audit the label without
/// rerunning anything.
pub(crate) fn asserts_only_its_own_setup(body: &str) -> (bool, String) {
    let calls = assertions(body);
    if calls.is_empty() {
        return (false, "no assertions at all".to_owned());
    }
    let mut setup = body.to_owned();
    for (whole, _) in &calls {
        setup = setup.replace(whole.as_str(), " ");
    }
    let mut reasons: Vec<String> = Vec::new();
    for (_, expected) in &calls {
        if expected.contains("::") {
            reasons.push(format!(
                "`::` path in an expected operand: {}",
                expected.trim().chars().take(60).collect::<String>()
            ));
            continue;
        }
        for literal in literals(expected) {
            if !setup_carries(&setup, &literal) {
                reasons.push(format!(
                    "literal {literal} is asserted but never appears in the setup"
                ));
            }
        }
    }
    if reasons.is_empty() {
        (
            true,
            "every asserted literal round-trips a value the test supplied".to_owned(),
        )
    } else {
        reasons.truncate(3);
        (false, reasons.join("; "))
    }
}

/// Every `(whole call text, expected operand)` in `body`.
fn assertions(body: &str) -> Vec<(String, String)> {
    const HEADS: [(&str, bool); 4] = [
        ("assert_eq!", true),
        ("assert_ne!", true),
        ("assert_matches!", true),
        ("assert!", false),
    ];
    let bytes = body.as_bytes();
    let mut out = Vec::new();
    let mut at = 0usize;
    while at < bytes.len() {
        if !body.is_char_boundary(at) {
            at += 1;
            continue;
        }
        let Some((head, takes_second)) = HEADS
            .iter()
            .copied()
            .find(|(head, _)| body[at..].starts_with(head))
        else {
            at += 1;
            continue;
        };
        // `assert!` must not swallow `assert_eq!`: the longer heads are tried
        // first by the order of `HEADS`, so reaching here with `assert!` means
        // the text really is the bare form.
        let mut open = at + head.len();
        while bytes.get(open).is_some_and(u8::is_ascii_whitespace) {
            open += 1;
        }
        if bytes.get(open) != Some(&b'(') {
            at += head.len();
            continue;
        }
        let Some(close) = balanced_end(body, open) else {
            at += head.len();
            continue;
        };
        let inner = &body[open + 1..close];
        let args = split_top_level(inner);
        if let Some(expected) = args.get(usize::from(takes_second)) {
            out.push((body[at..=close].to_owned(), (*expected).to_owned()));
        }
        at = close + 1;
    }
    out
}

/// The index of the delimiter closing the one at `open`, skipping string and
/// raw-string literals.
fn balanced_end(text: &str, open: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut at = open;
    while at < bytes.len() {
        if let Some(after) = skip_string(text, at) {
            at = after;
            continue;
        }
        match bytes[at] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth -= 1;
                if depth <= 0 {
                    return Some(at);
                }
            }
            _ => {}
        }
        at += 1;
    }
    None
}

/// If a string literal starts at `at`, the index just past it.
///
/// Handles `"..."` with backslash escapes, and `r"..."` / `r#"..."#` /
/// `r##"..."##` raw strings, which the evidence-store tests use for stored
/// JSON.
fn skip_string(text: &str, at: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    if bytes.get(at) == Some(&b'r') {
        let mut hashes = 0usize;
        let mut cursor = at + 1;
        while bytes.get(cursor) == Some(&b'#') {
            hashes += 1;
            cursor += 1;
        }
        if bytes.get(cursor) == Some(&b'"') {
            let terminator = format!("\"{}", "#".repeat(hashes));
            let rest = &text[cursor + 1..];
            return rest
                .find(&terminator)
                .map(|offset| cursor + 1 + offset + terminator.len());
        }
        return None;
    }
    if bytes.get(at) != Some(&b'"') {
        return None;
    }
    let mut cursor = at + 1;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'\\' => cursor += 2,
            b'"' => return Some(cursor + 1),
            _ => cursor += 1,
        }
    }
    None
}

/// Top-level comma split of a macro argument list, string-literal aware.
fn split_top_level(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    let mut at = 0usize;
    while at < bytes.len() {
        if let Some(after) = skip_string(text, at) {
            at = after;
            continue;
        }
        match bytes[at] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b',' if depth == 0 => {
                out.push(&text[start..at]);
                start = at + 1;
            }
            _ => {}
        }
        at += 1;
    }
    if !text[start..].trim().is_empty() {
        out.push(&text[start..]);
    }
    out
}

/// Every string, integer and boolean literal in `text`.
fn literals(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut at = 0usize;
    while at < bytes.len() {
        if !text.is_char_boundary(at) {
            at += 1;
            continue;
        }
        if let Some(after) = skip_string(text, at) {
            out.push(text[at..after].to_owned());
            at = after;
            continue;
        }
        let preceded_by_word = at
            .checked_sub(1)
            .is_some_and(|before| is_word(bytes[before]));
        let byte = bytes[at];
        if byte.is_ascii_digit() && !preceded_by_word {
            let mut end = at;
            while end < bytes.len() && (bytes[end].is_ascii_digit() || bytes[end] == b'_') {
                end += 1;
            }
            out.push(text[at..end].to_owned());
            at = end;
            continue;
        }
        let word = ["true", "false"].into_iter().find(|word| {
            text[at..].starts_with(word)
                && !preceded_by_word
                && !bytes.get(at + word.len()).copied().is_some_and(is_word)
        });
        if let Some(word) = word {
            out.push(word.to_owned());
            at += word.len();
            continue;
        }
        at += 1;
    }
    out
}

const fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Does the setup carry this literal? A string literal must appear verbatim;
/// a number or boolean must appear as a whole word, so `1` in a timestamp
/// does not count as the test having configured the number one.
fn setup_carries(setup: &str, literal: &str) -> bool {
    if literal.starts_with('"') || literal.starts_with('r') {
        return setup.contains(literal);
    }
    setup.match_indices(literal).any(|(at, _)| {
        let before = setup.as_bytes().get(at.wrapping_sub(1)).copied();
        let after = setup.as_bytes().get(at + literal.len()).copied();
        !before.is_some_and(is_word) && !after.is_some_and(is_word)
    })
}

// ---------------------------------------------------------------------------
// Reading the battery's answers
// ---------------------------------------------------------------------------

/// The five `noul` keys the battery asks, in the ticket's own order.
pub(crate) const NOUL_KEYS: [&str; 5] = [
    "test_asserts_intent",
    "assertion_vacuous",
    "tests_only_its_own_mock",
    "code_implements_intent",
    "code_exceeds_requirement",
];

/// One response, reduced to the numbers this module grades.
#[derive(Debug, Clone, Default)]
pub(crate) struct BatteryAnswers {
    pub(crate) noul: BTreeMap<String, f64>,
    pub(crate) divergence_kind: Option<String>,
    pub(crate) severity: Option<f64>,
}

impl BatteryAnswers {
    /// The probability the lens gave `key`, or `None` when it answered none.
    pub(crate) fn p(&self, key: &str) -> Option<f64> {
        self.noul.get(key).copied()
    }

    /// `key`'s answer thresholded at 0.5 — the same threshold MP-232 uses.
    pub(crate) fn says(&self, key: &str) -> Option<bool> {
        self.p(key).map(|value| value >= 0.5)
    }

    /// `divergence_kind` derived from this response's OWN noul answers,
    /// rather than asked for directly.
    ///
    /// Pre-registered before the first live call, and ordered: a code defect
    /// outranks excess, and excess outranks a weak test, because a triple can
    /// exhibit more than one and the six-way label admits only the dominant
    /// one. PLAT-838 took its EARS lens from 62.5% to 93.8% with exactly this
    /// move — derive the closed label from the open questions rather than
    /// asking for the label.
    pub(crate) fn derived_divergence_kind(&self) -> Option<&'static str> {
        if !self.says("code_implements_intent")? {
            return Some("code_short_of_requirement");
        }
        if self.says("code_exceeds_requirement")? {
            return Some("code_exceeds_requirement");
        }
        if !self.says("test_asserts_intent")? || self.says("assertion_vacuous")? {
            return Some("test_weaker_than_requirement");
        }
        Some("aligned")
    }
}

/// Reduces a response to [`BatteryAnswers`].
pub(crate) fn answers(response: &SystemOneResponse) -> BatteryAnswers {
    let mut out = BatteryAnswers::default();
    for key in NOUL_KEYS {
        if let Some(Answer::Noul(answer)) = response.answer(key) {
            out.noul.insert(key.to_owned(), answer.noul);
        }
    }
    if let Some(Answer::Choice(choice)) = response.answer("divergence_kind") {
        out.divergence_kind = Some(choice.choice.clone());
    }
    if let Some(Answer::Score(score)) = response.answer("severity") {
        out.severity = Some(score.score);
    }
    out
}

// ---------------------------------------------------------------------------
// Grading
// ---------------------------------------------------------------------------

/// One graded binary row: what the lens said about `key`, against truth.
#[derive(Debug, Clone)]
pub(crate) struct BinaryRow {
    pub(crate) id: String,
    pub(crate) truth: bool,
    pub(crate) predicted_prob: Option<f64>,
    pub(crate) predicted: Option<bool>,
}

/// Confusion counts over rows the lens actually answered. Rows it left
/// unanswered are counted separately and never defaulted to a class.
pub(crate) fn tally_binary(rows: &[BinaryRow]) -> (BinaryStats, usize) {
    let mut stats = BinaryStats::default();
    let mut unanswered = 0usize;
    for row in rows {
        match row.predicted {
            None => unanswered += 1,
            Some(true) if row.truth => stats.true_positive += 1,
            Some(true) => stats.false_positive += 1,
            Some(false) if row.truth => stats.false_negative += 1,
            Some(false) => stats.true_negative += 1,
        }
    }
    (stats, unanswered)
}

/// The best constant predictor over these rows, computed from the data.
pub(crate) fn constant_binary_baseline(rows: &[BinaryRow]) -> (&'static str, f64) {
    let answered: Vec<&BinaryRow> = rows.iter().filter(|row| row.predicted.is_some()).collect();
    if answered.is_empty() {
        return ("<none>", 0.0);
    }
    let positives = answered.iter().filter(|row| row.truth).count();
    let negatives = answered.len() - positives;
    if positives >= negatives {
        ("always yes", percent(positives, answered.len()))
    } else {
        ("always no", percent(negatives, answered.len()))
    }
}

/// One graded label row, for `divergence_kind`.
#[derive(Debug, Clone)]
pub(crate) struct LabelRow {
    pub(crate) id: String,
    pub(crate) truth: &'static str,
    pub(crate) asked: Option<String>,
    pub(crate) derived: Option<&'static str>,
}

/// Agreement of `pick` with truth over the rows it answered, and the count it
/// left unanswered.
pub(crate) fn label_agreement(
    rows: &[LabelRow],
    pick: impl Fn(&LabelRow) -> Option<String>,
) -> (f64, usize, usize) {
    let mut hits = 0usize;
    let mut answered = 0usize;
    let mut unanswered = 0usize;
    for row in rows {
        match pick(row) {
            None => unanswered += 1,
            Some(label) => {
                answered += 1;
                if label == row.truth {
                    hits += 1;
                }
            }
        }
    }
    (percent(hits, answered), answered, unanswered)
}

/// The majority-label constant predictor over these rows.
pub(crate) fn constant_label_baseline(rows: &[LabelRow]) -> (&'static str, f64) {
    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for row in rows {
        *counts.entry(row.truth).or_default() += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map_or(("<none>", 0.0), |(label, count)| {
            (label, percent(count, rows.len()))
        })
}

/// One (original, violating-mutant) severity pair.
#[derive(Debug, Clone)]
pub(crate) struct SeverityPair {
    pub(crate) id: String,
    pub(crate) original: Option<f64>,
    pub(crate) mutant: Option<f64>,
}

/// Share of pairs where the mutant scored strictly MORE severe than the
/// original, over the pairs both sides answered. A constant answer wins none,
/// so the null is a coin flip at 50%, not this number's own baseline.
pub(crate) fn severity_discrimination(pairs: &[SeverityPair]) -> (f64, usize, usize) {
    let mut wins = 0usize;
    let mut ties = 0usize;
    let mut compared = 0usize;
    for pair in pairs {
        let (Some(original), Some(mutant)) = (pair.original, pair.mutant) else {
            continue;
        };
        compared += 1;
        if mutant > original {
            wins += 1;
        } else if (mutant - original).abs() < f64::EPSILON {
            ties += 1;
        }
    }
    (percent(wins, compared), compared, ties)
}

/// Renders a binary question's per-row table and rollup.
pub(crate) fn binary_report(title: &str, rows: &[BinaryRow]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "\n## {title}\n");
    let _ = writeln!(out, "| Row | Truth | P(yes) | Predicted | Correct |");
    let _ = writeln!(out, "| --- | --- | --- | --- | --- |");
    for row in rows {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} |",
            row.id,
            row.truth,
            row.predicted_prob
                .map_or_else(|| "n/a".to_owned(), |value| format!("{value:.2}")),
            row.predicted
                .map_or_else(|| "n/a".to_owned(), |value| value.to_string()),
            row.predicted.map_or_else(
                || "n/a".to_owned(),
                |value| (value == row.truth).to_string()
            ),
        );
    }
    let (stats, unanswered) = tally_binary(rows);
    let (label, baseline) = constant_binary_baseline(rows);
    let show =
        |value: Option<f64>| value.map_or_else(|| "n/a".to_owned(), |value| format!("{value:.1}%"));
    let _ = writeln!(out, "\n- rows: {} ({unanswered} unanswered)", rows.len());
    let _ = writeln!(out, "- agreement: {}", show(stats.accuracy()));
    let _ = writeln!(
        out,
        "- constant-predictor baseline: `{label}` scores {baseline:.1}%"
    );
    let _ = writeln!(out, "- yes-class recall: {}", show(stats.recall()));
    let _ = writeln!(out, "- no-class recall: {}", show(stats.no_defect_recall()));
    let _ = writeln!(
        out,
        "- confusion: TP={} FP={} TN={} FN={}",
        stats.true_positive, stats.false_positive, stats.true_negative, stats.false_negative
    );
    out
}

/// The corpus's 29 real triples, each with the static check's answer — the
/// `tests_only_its_own_mock` negative class, computed rather than asserted.
pub(crate) fn real_mock_rows() -> Vec<(GapTriple, bool, String)> {
    corpus()
        .into_iter()
        .map(|triple| {
            let (answer, reason) = asserts_only_its_own_setup(&triple.test_body);
            (triple, answer, reason)
        })
        .collect()
}
