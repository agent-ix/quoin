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

use crate::eval_v2_support::corpus::Row;
use crate::eval_v2_support::keys::Mode;
use crate::eval_v2_support::variant::{
    Answered, Ask, Predictions, RunOutput, Variant, request, whole_row,
};
use crate::eval_v2_support::variants::battery::{
    Battery, Check, Combiner, RuleLine, Scores, holistic_predictions, holistic_scores,
    probability_table,
};
use crate::eval_v2_support::variants::soundness::{BarD, TAU, defect_prediction, required_noul};

/// The aggregate key: `yes` when the statement has none of the defects.
pub(crate) const SOUND: &str = "fr_statement_sound";
/// The mutation kind whose rows inject one statement defect.
pub(crate) const MUTATION_KIND: &str = "fr_statement";
/// F0's wire key.
pub(crate) const WELL_FORMED: &str = "well_formed";
/// The state field carrying the unit.
pub(crate) const STATEMENT_FIELD: &str = "statement";

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
        .map(|p| holistic_predictions(SOUND, p))
        .unwrap_or_default()
}

/// The holistic baseline.
pub(crate) const F0: Variant = Variant {
    id: "F0",
    // v2 (derive only, PR #634 review): the graded answer is
    // `battery::holistic_predictions`, which flags `P(well-formed) = 0.5`
    // as the report always did; v1 graded it well-formed. Same questions.
    version: 2,
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

/// The requirement statement's battery.
pub(crate) const STATEMENT: Battery = Battery {
    sound: SOUND,
    checks: &CHECKS,
    mutation_kind: MUTATION_KIND,
    labelled_only: false,
};

/// Whether `row` is a requirement-statement mutant: it carries truth for the
/// statement keys only.
pub(crate) fn is_statement_mutant(row: &Row) -> bool {
    STATEMENT.is_mutant(row)
}

/// The check a statement mutant injected: its one by-construction `yes`.
pub(crate) fn injected(row: &Row) -> Option<&'static str> {
    STATEMENT.injected(row)
}

/// Bar D over every statement pair ([`Battery::bar_d`] on [`STATEMENT`]).
///
/// # Panics
/// As [`Battery::bar_d`].
pub(crate) fn bar_d(
    rows: &[Row],
    scores: &Scores<'_>,
    tau: f64,
    checks: &[&str],
    excluded_key: &str,
) -> BarD {
    STATEMENT.bar_d(rows, scores, tau, checks, excluded_key)
}

/// Bar D with one pair per statement source ([`Battery::bar_d_per_source`]
/// on [`STATEMENT`]).
///
/// # Panics
/// As [`Battery::bar_d`].
pub(crate) fn bar_d_per_source(
    rows: &[Row],
    scores: &Scores<'_>,
    tau: f64,
    checks: &[&str],
    excluded_key: &str,
) -> BarD {
    STATEMENT.bar_d_per_source(rows, scores, tau, checks, excluded_key)
}

/// Each row's F1 check probabilities for the variant labelled `label`.
fn f1_table<'a>(rows: &'a [Row], output: &RunOutput, label: &str) -> BTreeMap<&'a str, Vec<f64>> {
    probability_table(rows, output, label, |row, answered| {
        f1_probabilities(row, answered).map(|p| p.to_vec())
    })
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
    let all = STATEMENT.labelled(rows);
    let _ = writeln!(
        out,
        "\n## Requirement statement ({SOUND}; dev only, informational)\n\n{} labelled rows: {} \
         natural (AGENT-LABELLED, single pass), {} statement mutants (by construction). Credit \
         counts an alternative reading as half. Bar D: {MUTATION_KIND} mutant vs its source, the \
         shared pair rule (cross the rule's threshold, move at least 0.10).\n\n{}",
        all.len(),
        all.iter().filter(|row| row.mutation.is_none()).count(),
        all.iter().filter(|row| row.mutation.is_some()).count(),
        STATEMENT.rule_header(),
    );
    if let Some(f0) = ran("F0") {
        let label = f0.label();
        STATEMENT.render_rule(
            &mut out,
            rows,
            &RuleLine {
                name: format!("{label} holistic P(no) >= 0.5"),
                scores: holistic_scores(rows, output, &label, f0_probability),
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
        STATEMENT.render_rule(
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
    let wording = if f1.version == 1 {
        String::new()
    } else {
        format!(", POST HOC wording (v{})", f1.version)
    };
    STATEMENT.render_checks(&mut out, rows, &table, TAU, &format!("{label}{wording}"));
    out
}

/// One line per F1 row: every check's probability, the row's truth and its
/// unit, for reading flagged rows by hand.
pub(crate) fn dump(rows: &[Row], output: &RunOutput, variants: &[&Variant]) -> Vec<String> {
    let f0 = variants
        .iter()
        .find(|variant| variant.id == "F0")
        .map(|variant| holistic_scores(rows, output, &variant.label(), f0_probability))
        .unwrap_or_default();
    let Some(f1) = variants.iter().find(|variant| variant.id == "F1") else {
        return Vec::new();
    };
    let table = f1_table(rows, output, &f1.label());
    STATEMENT.dump(rows, &table, &f0, statement_unit)
}
