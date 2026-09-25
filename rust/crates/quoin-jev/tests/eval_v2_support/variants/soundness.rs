// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Criterion-soundness checklist variants K1 and K2, and MP-243's bars
//! (PLAT-1031, parent PLAT-1024). Pre-registered in
//! `spec/assurance/MP-243-jev-criterion-soundness-checklist.md`.
//!
//! # Why a checklist
//!
//! PLAT-917 (MP-228) asked the criterion-strength lens one holistic question,
//! "classify this criterion's weakness, if any". As shipped (`v0`) it flagged
//! every criterion: of 5 criteria labelled sound it cleared 0 (46.7%, -33.3
//! pp). No later wording beat the constant predictor; the last retry, `v5`,
//! scored -6.7 to 0 pp (`live_criterion_strength.rs`). The EARS lens, which
//! asks named, narrow pattern-defect questions, passed with +18.6 pp (MP-229).
//! These variants replace the holistic question with one yes/no `noul` per
//! named defect and derive `criterion_sound` in code.
//!
//! # The variants
//!
//! | id | asks | derives |
//! | --- | --- | --- |
//! | `K1` | one `noul` per defect in [`CHECKS`]; each states the exact condition, with the boundary cases in its yes/no criteria | each defect key at [`TAU`]; `criterion_sound = yes` iff no defect is at or above [`TAU`] |
//! | `K2` | `K1`, with 4 synthetic worked examples per check (2 per outcome) appended to each instruction | as `K1` |
//!
//! Both run in mode `R` only. The state is the criterion (the AC, or the
//! requirement's statement when the row has no AC) plus the requirement's
//! statement as context, the same context C0 and the corpus labelling brief
//! use ([`criterion_state`]).
//!
//! # τ
//!
//! [`TAU`] = 0.5. A defect key is `yes` at `p >= TAU`. `criterion_sound` is
//! `no` exactly when some defect key is `yes`, so the per-check and aggregate
//! answers never disagree. τ is fixed before the first live call.
//!
//! Every asked check must come back as a probability ([`required_noul`]). A
//! missing, mis-typed or out-of-range answer aborts the run naming the row
//! and the key; it never becomes an abstention.
//!
//! # The bars
//!
//! [`bars`] computes MP-243's bars A-D for one variant: A-C on the natural
//! rows only (a mutant's inherited labels never count), with fractional
//! contested credit ([`credit`]); D on mutant/source pairs linked by
//! `mutation.source_id` ([`pairs`]), with a one-sided sign test
//! ([`sign_test_p`]). [`render_bars`] prints them for every K variant in a
//! run, plus the jointly-answered comparison against C0.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde_json::{Value, json};
use typesafe_sdk_questions::{Entry, NoulCriteria, Question, Questions, noul_with};

use crate::eval_v2_support::corpus::{Row, Truth, TruthAnswer, TruthKind};
use crate::eval_v2_support::criterion_defects::CRITERION_DEFECTS;
use crate::eval_v2_support::keys::{Mode, NO, YES};
use crate::eval_v2_support::variant::{
    Answered, Ask, Prediction, Predictions, RawAnswer, RawAnswers, RunOutput, Variant, criterion,
    request, whole_row,
};

/// The threshold at which a defect `noul` counts as `yes`.
pub(crate) const TAU: f64 = 0.5;
/// Bar D: the least rise (or fall) in the injected check's probability, from
/// source to mutant, that is not a tie.
pub(crate) const DELTA: f64 = 0.10;
/// Bar D: the one-sided sign test's level.
pub(crate) const ALPHA: f64 = 0.05;
/// Bar D: the fewest non-tie pairs for D to be gateable.
pub(crate) const MIN_PAIRS: usize = 10;
/// The share of a slice's labelled rows a variant may leave unanswered
/// before a comparison on that slice is not interpretable.
pub(crate) const ABSTENTION_CEILING: f64 = 0.05;

/// The aggregate key.
pub(crate) const SOUND: &str = "criterion_sound";

/// One synthetic worked example: a criterion and why it falls on its side.
/// Written for this module; none is taken from a corpus row.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Example {
    /// The example criterion.
    pub(crate) criterion: &'static str,
    /// Why it is on this side of the check.
    pub(crate) why: &'static str,
}

/// One named defect check.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Check {
    /// The corpus key, also the wire key. `yes` is the defect.
    pub(crate) key: &'static str,
    /// The question.
    pub(crate) instruction: &'static str,
    /// What `yes` (the defect) means, with its boundary cases.
    pub(crate) defect: &'static str,
    /// What `no` (clear) means, with its boundary cases.
    pub(crate) clear: &'static str,
    /// `K2`'s examples on the `yes` side.
    pub(crate) defect_examples: &'static [Example],
    /// `K2`'s examples on the `no` side.
    pub(crate) clear_examples: &'static [Example],
}

/// Every check's instruction opens with this sentence.
macro_rules! judge {
    ($question:literal) => {
        concat!(
            "Judge the text in `criterion`, read literally. `requirement_statement` is the \
             requirement the criterion belongs to, given only as context for what the \
             criterion's words refer to. ",
            $question
        )
    };
}

/// The five named defects, in [`crate::eval_v2_support::keys::KEYS`] order.
/// Each check's `defect` and `clear` text is the canonical definition the
/// corpus labels to (MP-243). PLAT-1025's shared definitions module holds
/// the same text word for word; once it is on `main`, this table reads its
/// text from there.
pub(crate) const CHECKS: [Check; 5] = [
    Check {
        key: "vague_term",
        instruction: judge!(
            "Does the criterion rely on a vague or subjective word or phrase whose meaning \
             decides whether it passes or fails?"
        ),
        defect: CRITERION_DEFECTS[0].defect,
        clear: CRITERION_DEFECTS[0].clear,
        defect_examples: &[
            Example {
                criterion: "The dashboard loads in a reasonable time on typical hardware.",
                why: "'reasonable' and 'typical' decide pass or fail and have no fixed meaning.",
            },
            Example {
                criterion: "Malformed invoices are handled gracefully.",
                why: "'gracefully' names no observable outcome; reviewers would disagree.",
            },
        ],
        clear_examples: &[
            Example {
                criterion: "An invoice with a negative total is rejected with error INV_NEGATIVE.",
                why: "Every deciding word has one reading: the input, the verb and the \
                      identifier.",
            },
            Example {
                criterion: "Each exported invoice is a valid UBL 2.1 document.",
                why: "'valid' is defined by the named standard.",
            },
        ],
    },
    Check {
        key: "no_measurable_threshold",
        instruction: judge!(
            "Does the criterion state a quantity, limit, rate, duration, size or quality level \
             without the number and unit, or the exact named value, needed to decide pass or \
             fail?"
        ),
        defect: CRITERION_DEFECTS[1].defect,
        clear: CRITERION_DEFECTS[1].clear,
        defect_examples: &[
            Example {
                criterion: "Search results appear quickly for large catalogues.",
                why: "Both a duration and a size are implied and neither has a number.",
            },
            Example {
                criterion: "Most uploads complete without a retry.",
                why: "'Most' is a proportion with no stated percentage.",
            },
        ],
        clear_examples: &[
            Example {
                criterion: "A thumbnail request for a 20 MB image returns within 800 ms at the \
                            95th percentile.",
                why: "The size, the duration and the percentile are all stated.",
            },
            Example {
                criterion: "A password reset link is single-use: a second visit shows the \
                            expired-link page.",
                why: "No quantity or level is asserted, so no threshold is needed.",
            },
        ],
    },
    Check {
        key: "untestable",
        instruction: judge!(
            "Is the criterion unfalsifiable, meaning that whatever the system is observed to do, \
             the criterion could still be claimed to hold?"
        ),
        defect: CRITERION_DEFECTS[2].defect,
        clear: CRITERION_DEFECTS[2].clear,
        defect_examples: &[
            Example {
                criterion: "The scheduler is built with future scalability in mind.",
                why: "A design intention: no observation could show it false.",
            },
            Example {
                criterion: "The sync client aims to resolve conflicts within one second.",
                why: "'aims to' governs the outcome, so a slow resolution does not falsify it.",
            },
        ],
        clear_examples: &[
            Example {
                criterion: "When two devices edit the same note offline, the later save wins \
                            and the earlier text is kept in the note's history.",
                why: "Both outcomes can be observed after a scripted conflict.",
            },
            Example {
                criterion: "Error messages are shown in a user-friendly way.",
                why: "Vague, but an error message is observable and a clearly hostile one \
                      would count against it, so it is not unfalsifiable.",
            },
        ],
    },
    Check {
        key: "compound",
        instruction: judge!(
            "Does the criterion bundle two or more independently verifiable outcomes or \
             conditions, so that one could hold while another fails?"
        ),
        defect: CRITERION_DEFECTS[3].defect,
        clear: CRITERION_DEFECTS[3].clear,
        defect_examples: &[
            Example {
                criterion: "On checkout the stock count is decremented and a confirmation \
                            email is sent.",
                why: "Two different effects; either could fail while the other holds.",
            },
            Example {
                criterion: "The export is written as CSV or JSON.",
                why: "Two behaviours joined by 'or' with no statement of which is required.",
            },
        ],
        clear_examples: &[
            Example {
                criterion: "An expired session token is refused with status 401 and error \
                            SESSION_EXPIRED.",
                why: "One response to one event, described by its status and its identifier.",
            },
            Example {
                criterion: "Empty, whitespace-only and over-long usernames are each rejected at \
                            sign-up.",
                why: "A list of inputs sharing one outcome.",
            },
        ],
    },
    Check {
        key: "missing_trigger",
        instruction: judge!(
            "Does the criterion describe a response or behaviour that only makes sense after \
             some event, in some state or under some condition, while naming none?"
        ),
        defect: CRITERION_DEFECTS[4].defect,
        clear: CRITERION_DEFECTS[4].clear,
        defect_examples: &[
            Example {
                criterion: "A warning banner is displayed.",
                why: "A banner only makes sense in response to something, and nothing is named.",
            },
            Example {
                criterion: "The draft is then discarded.",
                why: "'then' refers to an event the criterion does not state.",
            },
        ],
        clear_examples: &[
            Example {
                criterion: "A login attempt with a wrong password increments the failed-attempt \
                            counter.",
                why: "The input it applies to is the trigger.",
            },
            Example {
                criterion: "Every audit record carries a UTC timestamp.",
                why: "Holds at all times; no trigger is needed.",
            },
        ],
    },
];

/// The keys every K variant is graded on: `criterion_sound` and each check,
/// read off [`CHECKS`] so a new check cannot be asked and left ungraded.
pub(crate) const GRADES: [&str; CHECKS.len() + 1] = {
    let [a, b, c, d, e] = CHECKS;
    [SOUND, a.key, b.key, c.key, d.key, e.key]
};

/// One check's instruction; `with_examples` appends `K2`'s worked examples.
pub(crate) fn check_instruction(check: &Check, with_examples: bool) -> String {
    let mut text = check.instruction.to_owned();
    if with_examples {
        text.push_str("\n\nWorked examples:");
        for (answer, examples) in [(YES, check.defect_examples), (NO, check.clear_examples)] {
            for example in examples {
                let _ = write!(
                    text,
                    "\n- \"{}\" -> {answer}: {}",
                    example.criterion, example.why
                );
            }
        }
    }
    text
}

/// One check as a `noul`; `with_examples` is `K2`'s form. The criteria are
/// the same in both.
pub(crate) fn check_question(check: &Check, with_examples: bool) -> Question {
    noul_with(
        check_instruction(check, with_examples),
        NoulCriteria {
            yes: Some(Entry::from(check.defect)),
            no: Some(Entry::from(check.clear)),
        },
    )
}

/// The state: the criterion under judgement and the requirement's statement
/// as context, as C0 sends it.
pub(crate) fn criterion_state(row: &Row) -> Value {
    let criterion = criterion(row);
    json!({
        "criterion_id": criterion.id,
        "criterion": criterion.text,
        "requirement_id": row.requirement.fr_id,
        "requirement_statement": row.requirement.statement,
    })
}

fn checklist(with_examples: bool) -> Questions {
    CHECKS
        .iter()
        .map(|check| (check.key.to_owned(), check_question(check, with_examples)))
        .collect()
}

/// K1 v2's `compound` question (PLAT-1024 experiment 2). v1 asked about
/// "outcomes or conditions", which the canonical clear text contradicts
/// ("several conditions on the trigger do not make it compound"). On dev, v1's
/// `compound` fired on 6 of 15 sound criteria, 3 of them one response read as
/// several ("names it and refuses acceptance"; a gate described by its legs;
/// one prohibition over a list). v2 asks the canonical definition's question:
/// it drops "or conditions" and names the definition's own one-outcome cases.
pub(crate) const COMPOUND_V2: &str = judge!(
    "Does the criterion require two or more different behaviours, effects or outcomes, each of \
     which could be verified, and fail, on its own? Count required outcomes, not clauses: parts \
     that together describe one observed response to one event (a refusal and the message or \
     identifier it gives, a status with its error identifier, one value or one artifact described by \
     several of its own attributes) are one outcome; conditions on the trigger are not outcomes; \
     a list of inputs that must each get the same single outcome is one outcome."
);

fn k1_checklist() -> Questions {
    CHECKS
        .iter()
        .map(|check| {
            let question = if check.key == "compound" {
                noul_with(
                    COMPOUND_V2,
                    NoulCriteria {
                        yes: Some(Entry::from(check.defect)),
                        no: Some(Entry::from(check.clear)),
                    },
                )
            } else {
                check_question(check, false)
            };
            (check.key.to_owned(), question)
        })
        .collect()
}

fn k1_asks(row: &Row) -> Vec<Ask> {
    vec![Ask {
        unit: None,
        request: request(criterion_state(row), k1_checklist()),
    }]
}

fn k2_asks(row: &Row) -> Vec<Ask> {
    vec![Ask {
        unit: None,
        request: request(criterion_state(row), checklist(true)),
    }]
}

// ---------------------------------------------------------------------------
// Derivation
// ---------------------------------------------------------------------------

/// The probability the service gave for a `noul` this variant asked on
/// `row`.
///
/// # Panics
/// When `key` is missing, is not a `noul`, or is not a finite probability in
/// `[0, 1]`. Every key read here was asked, so any of those is a broken
/// response or a mis-keyed question: the run stops naming the row and the
/// key, rather than letting the row read as an abstention.
#[allow(
    clippy::panic,
    reason = "a missing or mis-keyed answer to an asked question must abort the run, not become an abstention (MP-243)"
)]
pub(crate) fn required_noul(row: &str, answers: &RawAnswers, key: &str) -> f64 {
    match answers.get(key) {
        Some(RawAnswer::Noul(p)) if p.is_finite() && (0.0..=1.0).contains(p) => *p,
        Some(RawAnswer::Noul(p)) => panic!("{row} / {key}: noul {p} is not a probability"),
        Some(other) => panic!("{row} / {key}: asked as a noul, answered as {other:?}"),
        None => panic!(
            "{row} / {key}: asked but not answered; answered keys: {:?}",
            answers.keys().collect::<Vec<_>>()
        ),
    }
}

/// A defect `noul` probability as a graded answer: `yes` at `p >= TAU`, the
/// confidence of the chosen label, and `p` as the ordinal.
pub(crate) fn defect_prediction(p: f64) -> Prediction {
    let yes = p >= TAU;
    Prediction {
        answer: if yes { YES } else { NO }.to_owned(),
        confidence: Some(if yes { p } else { 1.0 - p }),
        ordinal: Some(p),
    }
}

/// Graded answers from one defect probability per check, in [`CHECKS`]
/// order.
///
/// Each check is graded by [`defect_prediction`]. `criterion_sound` is `no`
/// when any check is at or above [`TAU`], with the highest probability as
/// its confidence, and `yes` otherwise, with confidence `1 - max`. Its
/// ordinal is `1 - max`, higher meaning more likely sound.
pub(crate) fn derive_checklist(probabilities: &[f64; CHECKS.len()]) -> Predictions {
    let mut out: Predictions = CHECKS
        .iter()
        .zip(probabilities)
        .map(|(check, p)| (check.key, defect_prediction(*p)))
        .collect();
    let highest = probabilities.iter().copied().fold(0.0, f64::max);
    let flagged = highest >= TAU;
    out.insert(
        SOUND,
        Prediction {
            answer: if flagged { NO } else { YES }.to_owned(),
            confidence: Some(if flagged { highest } else { 1.0 - highest }),
            ordinal: Some(1.0 - highest),
        },
    );
    out
}

fn k_derive(row: &Row, answered: &[Answered]) -> Predictions {
    let answers = whole_row(answered);
    derive_checklist(&CHECKS.map(|check| required_noul(&row.id, &answers, check.key)))
}

const R_ONLY: &[Mode] = &[Mode::Req];

/// The checklist: one `noul` per named defect, `criterion_sound` derived.
pub(crate) const K1: Variant = Variant {
    id: "K1",
    // v2 (PLAT-1024 experiment 2): the `compound` question is COMPOUND_V2.
    version: 2,
    summary: "criterion soundness as five named defect nouls (compound asked per the canonical definition); criterion_sound = no defect at or above 0.5",
    modes: R_ONLY,
    references: &[],
    grades: &GRADES,
    asks: k1_asks,
    derive: k_derive,
};

/// `K1` with synthetic worked examples appended to each check's instruction.
pub(crate) const K2: Variant = Variant {
    id: "K2",
    version: 1,
    summary: "K1 plus four synthetic worked examples per check (two per outcome) in the instruction",
    modes: R_ONLY,
    references: &[],
    grades: &GRADES,
    asks: k2_asks,
    derive: k_derive,
};

/// Every K variant, for [`render_bars`].
pub(crate) const K_VARIANTS: [&Variant; 2] = [&K1, &K2];

// ---------------------------------------------------------------------------
// Bars A-C: natural rows, fractional contested credit
// ---------------------------------------------------------------------------

fn count(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

/// A row that counts for bars A-C: mode `R` and not a mutant, so no
/// inherited label ever contributes.
pub(crate) fn is_natural(row: &Row) -> bool {
    row.mode == Mode::Req && row.mutation.is_none()
}

/// Credit for `answer` against `truth`. The readings are the primary label
/// and every alternative, deduplicated. An answer among `k` readings earns
/// `1/k`; any other answer, or none, earns 0. An uncontested row is worth 1
/// to a correct answer; a `yes`/`no` contested row is worth 0.5 to either
/// answer. The constant predictor is credited by the same rule.
pub(crate) fn credit(truth: &Truth, answer: Option<&str>) -> f64 {
    let mut readings: Vec<String> = std::iter::once(&truth.answer)
        .chain(&truth.alternatives)
        .map(TruthAnswer::label)
        .collect();
    readings.sort();
    readings.dedup();
    match answer {
        Some(answer) if readings.iter().any(|reading| reading == answer) => {
            1.0 / count(readings.len())
        }
        _ => 0.0,
    }
}

/// One key's standing against its best constant predictor on one slice.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct KeyMargin {
    /// Rows labelled with the key.
    pub(crate) rows: usize,
    /// Whether the rows' primary labels hold both `yes` and `no`.
    pub(crate) both_classes: bool,
    /// The variant's total credit.
    pub(crate) credit: f64,
    /// The best constant's total credit.
    pub(crate) constant: f64,
    /// The best constant's label; `yes` on a tie between the two.
    pub(crate) constant_label: &'static str,
    /// Labelled rows the variant gave no answer for.
    pub(crate) abstentions: usize,
}

impl KeyMargin {
    /// Strictly more credit than the constant. An exact tie, including one
    /// made of half credits, is not a win.
    pub(crate) fn beats_constant(&self) -> bool {
        self.credit > self.constant
    }

    /// The margin in percentage points, over every labelled row.
    pub(crate) fn margin_pp(&self) -> Option<f64> {
        (self.rows > 0).then(|| 100.0 * (self.credit - self.constant) / count(self.rows))
    }
}

/// `key`'s margin over `rows` for the predictions in `by_row`.
pub(crate) fn key_margin(
    rows: &[&Row],
    by_row: &BTreeMap<&str, &Predictions>,
    key: &str,
) -> KeyMargin {
    let labelled: Vec<(&Row, &Truth)> = rows
        .iter()
        .filter_map(|row| row.truth.get(key).map(|truth| (*row, truth)))
        .collect();
    let answer = |row: &Row| {
        by_row
            .get(row.id.as_str())
            .and_then(|predictions| predictions.get(key))
            .map(|prediction| prediction.answer.clone())
    };
    let constant_credit = |label: &str| -> f64 {
        labelled
            .iter()
            .map(|(_, truth)| credit(truth, Some(label)))
            .sum()
    };
    let (yes, no) = (constant_credit(YES), constant_credit(NO));
    let primaries: Vec<String> = labelled.iter().map(|(_, t)| t.answer.label()).collect();
    KeyMargin {
        rows: labelled.len(),
        both_classes: primaries.iter().any(|l| l == YES) && primaries.iter().any(|l| l == NO),
        credit: labelled
            .iter()
            .map(|(row, truth)| credit(truth, answer(row).as_deref()))
            .sum(),
        constant: yes.max(no),
        constant_label: if yes >= no { YES } else { NO },
        abstentions: labelled
            .iter()
            .filter(|(row, _)| answer(row).is_none())
            .count(),
    }
}

/// Bar C: which checks qualify (both classes on natural rows) and which of
/// those beat their constant.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BarC {
    /// Checks whose natural slice holds both classes.
    pub(crate) qualifying: Vec<&'static str>,
    /// Qualifying checks that beat their constant.
    pub(crate) beating: Vec<&'static str>,
}

impl BarC {
    /// Gateable with at least two qualifying checks.
    pub(crate) fn gateable(&self) -> bool {
        self.qualifying.len() >= 2
    }

    /// A strict majority of the qualifying checks beat their constant.
    pub(crate) fn passes(&self) -> bool {
        self.gateable() && 2 * self.beating.len() > self.qualifying.len()
    }
}

// ---------------------------------------------------------------------------
// Bar D: mutant/source pairs
// ---------------------------------------------------------------------------

/// One mutant and the natural row it was made from.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Pair<'a> {
    /// The mutated row.
    pub(crate) mutant: &'a Row,
    /// Its source row.
    pub(crate) source: &'a Row,
    /// The check the mutation injected.
    pub(crate) key: &'static str,
}

/// The one check a mode-`R` mutant's by-construction truth says was
/// injected, or `None` when it is not a checklist mutant.
///
/// # Panics
/// When more than one check is labelled injected: a mutation injects
/// exactly one defect.
#[allow(
    clippy::panic,
    reason = "a malformed corpus row must stop the run, not be quietly paired (MP-243)"
)]
pub(crate) fn injected_check(row: &Row) -> Option<&'static str> {
    let injected: Vec<&'static str> = CHECKS
        .iter()
        .map(|check| check.key)
        .filter(|key| {
            row.truth.get(*key).is_some_and(|truth| {
                truth.kind == TruthKind::ByConstruction && truth.answer.label() == YES
            })
        })
        .collect();
    match injected.as_slice() {
        [] => None,
        [key] => Some(*key),
        many => panic!("{}: injects {many:?}; a mutation injects one", row.id),
    }
}

/// Every checklist pair among `rows`, linked by `mutation.source_id`.
///
/// Only mode-`R` mutants are paired: K variants run in mode `R` only, so a
/// checklist mutant in any other mode has no K answer and is not a pair.
/// One pair per mutation id, as the shared paired rule has it. The shared
/// rule prefers the RTC row of a mutation; with every candidate in mode `R`
/// the first row in corpus order is used.
///
/// # Panics
/// When a checklist mutant has no `source_id`, or its source is missing from
/// `rows`, is itself a mutant, is not mode `R`, or is in another split.
#[allow(
    clippy::panic,
    reason = "a broken pair link must stop the run, not shrink bar D's denominator (MP-243)"
)]
pub(crate) fn pairs(rows: &[Row]) -> Vec<Pair<'_>> {
    let by_id: BTreeMap<&str, &Row> = rows.iter().map(|row| (row.id.as_str(), row)).collect();
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    rows.iter()
        .filter(|row| row.mode == Mode::Req)
        .filter_map(|mutant| {
            let mutation = mutant.mutation.as_ref()?;
            let key = injected_check(mutant)?;
            if !seen.insert(mutation.id.as_str()) {
                return None;
            }
            let id = &mutant.id;
            let source_id = mutation
                .source_id
                .as_deref()
                .unwrap_or_else(|| panic!("{id}: {key} mutant has no mutation.source_id"));
            let source = by_id
                .get(source_id)
                .copied()
                .unwrap_or_else(|| panic!("{id}: source {source_id} is not among this run's rows"));
            assert!(
                source.mutation.is_none(),
                "{id}: source {source_id} is itself a mutant, not a natural row"
            );
            assert!(
                source.mode == Mode::Req,
                "{id}: source {source_id} is mode {}, not R",
                source.mode.as_str()
            );
            assert!(
                source.split == mutant.split,
                "{id}: source {source_id} is in {}, the mutant in {}",
                source.split.as_str(),
                mutant.split.as_str()
            );
            Some(Pair {
                mutant,
                source,
                key,
            })
        })
        .collect()
}

/// How one pair came out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PairOutcome {
    /// The answer crossed τ (source below, mutant at or above) and rose by
    /// at least δ.
    Success,
    /// The mutant fell by at least δ.
    Failure,
    /// Anything else; excluded from the sign test.
    Tie,
}

/// One pair's outcome from the injected check's probability on the source
/// and on the mutant. A success must cross τ: the source below it and the
/// mutant at or above it, so a source already on the defect side by answer
/// can never succeed. The comparison allows 1e-9 for the float subtraction,
/// so a rise of exactly δ counts.
pub(crate) fn pair_outcome(source: f64, mutant: f64) -> PairOutcome {
    const SLACK: f64 = 1e-9;
    let rise = mutant - source;
    if source < TAU && mutant >= TAU && rise >= DELTA - SLACK {
        PairOutcome::Success
    } else if -rise >= DELTA - SLACK {
        PairOutcome::Failure
    } else {
        PairOutcome::Tie
    }
}

/// `P(X >= successes)` for `X ~ Binomial(n, 1/2)`: the one-sided sign test.
pub(crate) fn sign_test_p(successes: usize, n: usize) -> f64 {
    let n_f = count(n);
    let mut ln_choose = 0.0_f64;
    let mut tail = 0.0;
    for k in 0..=n {
        if k >= successes {
            tail += (ln_choose - n_f * std::f64::consts::LN_2).exp();
        }
        // ln C(n, k+1) = ln C(n, k) + ln(n - k) - ln(k + 1).
        if k < n {
            ln_choose += count(n - k).ln() - count(k + 1).ln();
        }
    }
    tail.min(1.0)
}

/// The injected check's probability, for a K variant: that check's `noul`
/// probability, which [`defect_prediction`] keeps as the ordinal.
pub(crate) fn injected_probability(predictions: &Predictions, key: &str) -> Option<f64> {
    predictions
        .get(key)
        .and_then(|prediction| prediction.ordinal)
}

/// Bar D's tally.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct BarD {
    /// Pairs found.
    pub(crate) pairs: usize,
    /// Pairs dropped because the source is already labelled `yes` on the
    /// injected check.
    pub(crate) source_already_yes: usize,
    /// Successes.
    pub(crate) successes: usize,
    /// Failures, including every abstained pair.
    pub(crate) failures: usize,
    /// Pairs counted as failures because the variant gave no probability on
    /// the source or the mutant.
    pub(crate) abstained: usize,
    /// Ties, excluded.
    pub(crate) ties: usize,
}

impl BarD {
    /// Non-tie pairs.
    pub(crate) fn decided(&self) -> usize {
        self.successes + self.failures
    }

    /// Gateable with at least [`MIN_PAIRS`] non-tie pairs.
    pub(crate) fn gateable(&self) -> bool {
        self.decided() >= MIN_PAIRS
    }

    /// The sign test's p-value over the non-tie pairs.
    pub(crate) fn p_value(&self) -> f64 {
        sign_test_p(self.successes, self.decided())
    }

    /// Gateable and significant at [`ALPHA`].
    pub(crate) fn passes(&self) -> bool {
        self.gateable() && self.p_value() <= ALPHA
    }
}

/// Bar D over `pairs` for the predictions in `by_row`. A pair where the
/// variant gave no probability for the injected check, on either row, is a
/// failure (the shared paired rule), never a tie.
pub(crate) fn bar_d(pairs: &[Pair<'_>], by_row: &BTreeMap<&str, &Predictions>) -> BarD {
    let mut tally = BarD {
        pairs: pairs.len(),
        ..BarD::default()
    };
    let probability = |row: &Row, key: &str| {
        by_row
            .get(row.id.as_str())
            .and_then(|predictions| injected_probability(predictions, key))
    };
    for pair in pairs {
        let source_yes = pair
            .source
            .truth
            .get(pair.key)
            .is_some_and(|truth| truth.answer.label() == YES);
        if source_yes {
            tally.source_already_yes += 1;
            continue;
        }
        let answered = probability(pair.source, pair.key).zip(probability(pair.mutant, pair.key));
        let outcome = answered.map_or_else(
            || {
                tally.abstained += 1;
                PairOutcome::Failure
            },
            |(source, mutant)| pair_outcome(source, mutant),
        );
        match outcome {
            PairOutcome::Success => tally.successes += 1,
            PairOutcome::Failure => tally.failures += 1,
            PairOutcome::Tie => tally.ties += 1,
        }
    }
    tally
}

// ---------------------------------------------------------------------------
// All bars for one variant
// ---------------------------------------------------------------------------

/// MP-243's bars for one variant on one run.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Bars {
    /// Bar A's `criterion_sound` margin on natural rows.
    pub(crate) a: KeyMargin,
    /// Bar B: natural rows labelled sound, and how many were answered `yes`.
    pub(crate) b: (usize, usize),
    /// Bar C.
    pub(crate) c: BarC,
    /// Bar D.
    pub(crate) d: BarD,
}

impl Bars {
    /// A: both classes present and strictly above the constant.
    pub(crate) fn a_passes(&self) -> bool {
        self.a.both_classes && self.a.beats_constant()
    }

    /// B: at least one row labelled sound, and at least one answered `yes`.
    pub(crate) fn b_passes(&self) -> bool {
        self.b.0 > 0 && self.b.1 > 0
    }

    /// Selectable: D passes, A and B pass, and C passes or is not gateable.
    pub(crate) fn selectable(&self) -> bool {
        self.d.passes()
            && self.a_passes()
            && self.b_passes()
            && (self.c.passes() || !self.c.gateable())
    }
}

/// Each row's predictions for `label` in `output`.
pub(crate) fn predictions_by_row<'a>(
    output: &'a RunOutput,
    label: &str,
) -> BTreeMap<&'a str, &'a Predictions> {
    output
        .results
        .iter()
        .filter(|result| result.variant == label)
        .map(|result| (result.row_id.as_str(), &result.predictions))
        .collect()
}

/// MP-243's bars for the variant labelled `label`, over `rows`.
///
/// # Panics
/// As [`pairs`] and [`bar_d`].
pub(crate) fn bars(rows: &[Row], output: &RunOutput, label: &str) -> Bars {
    let by_row = predictions_by_row(output, label);
    let natural: Vec<&Row> = rows.iter().filter(|row| is_natural(row)).collect();
    let sound_rows: Vec<&Row> = natural
        .iter()
        .copied()
        .filter(|row| {
            row.truth
                .get(SOUND)
                .is_some_and(|truth| truth.answer.label() == YES)
        })
        .collect();
    let cleared = sound_rows
        .iter()
        .filter(|row| {
            by_row
                .get(row.id.as_str())
                .and_then(|predictions| predictions.get(SOUND))
                .is_some_and(|prediction| prediction.answer == YES)
        })
        .count();
    let mut c = BarC {
        qualifying: Vec::new(),
        beating: Vec::new(),
    };
    for check in &CHECKS {
        let margin = key_margin(&natural, &by_row, check.key);
        if margin.both_classes {
            c.qualifying.push(check.key);
            if margin.beats_constant() {
                c.beating.push(check.key);
            }
        }
    }
    Bars {
        a: key_margin(&natural, &by_row, SOUND),
        b: (sound_rows.len(), cleared),
        c,
        d: bar_d(&pairs(rows), &by_row),
    }
}

/// `key` for two variants on the natural rows both answered, with each
/// one's abstentions and whether each is under [`ABSTENTION_CEILING`].
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Joint {
    /// Natural rows labelled with the key.
    pub(crate) labelled: usize,
    /// Of those, rows both answered.
    pub(crate) both: usize,
    /// The first variant's margin on the jointly answered rows.
    pub(crate) left: KeyMargin,
    /// The second's.
    pub(crate) right: KeyMargin,
    /// Labelled rows each left unanswered.
    pub(crate) abstentions: (usize, usize),
}

impl Joint {
    /// Both variants within the abstention ceiling.
    pub(crate) fn interpretable(&self) -> bool {
        let limit = ABSTENTION_CEILING * count(self.labelled);
        count(self.abstentions.0) <= limit && count(self.abstentions.1) <= limit
    }
}

/// `key` compared between `left` and `right` on the natural rows both
/// answered.
pub(crate) fn joint(rows: &[Row], output: &RunOutput, left: &str, right: &str, key: &str) -> Joint {
    let (l, r) = (
        predictions_by_row(output, left),
        predictions_by_row(output, right),
    );
    let answered = |by: &BTreeMap<&str, &Predictions>, row: &Row| {
        by.get(row.id.as_str()).is_some_and(|p| p.contains_key(key))
    };
    let labelled: Vec<&Row> = rows
        .iter()
        .filter(|row| is_natural(row) && row.truth.contains_key(key))
        .collect();
    let both: Vec<&Row> = labelled
        .iter()
        .copied()
        .filter(|row| answered(&l, row) && answered(&r, row))
        .collect();
    Joint {
        labelled: labelled.len(),
        both: both.len(),
        left: key_margin(&both, &l, key),
        right: key_margin(&both, &r, key),
        abstentions: (
            labelled.iter().filter(|row| !answered(&l, row)).count(),
            labelled.iter().filter(|row| !answered(&r, row)).count(),
        ),
    }
}

fn show(margin: &KeyMargin) -> String {
    format!(
        "{:.1}/{} vs `{}` {:.1} ({})",
        margin.credit,
        margin.rows,
        margin.constant_label,
        margin.constant,
        margin
            .margin_pp()
            .map_or_else(|| "n/a".to_owned(), |pp| format!("{pp:+.1} pp")),
    )
}

const fn verdict(gateable: bool, passes: bool) -> &'static str {
    if !gateable {
        "not gateable (no claim)"
    } else if passes {
        "pass"
    } else {
        "FAIL"
    }
}

/// One K variant's bars block.
fn render_one(out: &mut String, rows: &[Row], output: &RunOutput, label: &str) {
    let bars = bars(rows, output, label);
    let natural = rows.iter().filter(|row| is_natural(row)).count();
    let per_row = if natural == 0 {
        "n/a".to_owned()
    } else {
        format!("{:.1}", 100.0 / count(natural))
    };
    let _ = writeln!(
        out,
        "\n## MP-243 bars: {label} ({natural} natural R rows; one row = {per_row} pp)\n"
    );
    let _ = writeln!(
        out,
        "- A [AGENT-LABELLED, natural rows]: {} -> {}",
        show(&bars.a),
        verdict(bars.a.both_classes, bars.a_passes())
    );
    let _ = writeln!(
        out,
        "- B [AGENT-LABELLED, natural rows]: {} of {} sound rows cleared -> {}",
        bars.b.1,
        bars.b.0,
        verdict(bars.b.0 > 0, bars.b_passes())
    );
    let _ = writeln!(
        out,
        "- C [AGENT-LABELLED, natural rows]: {} of {} qualifying checks beat their constant \
         (qualifying {:?}) -> {}",
        bars.c.beating.len(),
        bars.c.qualifying.len(),
        bars.c.qualifying,
        verdict(bars.c.gateable(), bars.c.passes())
    );
    let d = &bars.d;
    let _ = writeln!(
        out,
        "- D [by-construction pairs]: {} pairs, {} dropped (source already yes), {} success, \
         {} failure ({} abstained), {} tie; p = {:.4} -> {}",
        d.pairs,
        d.source_already_yes,
        d.successes,
        d.failures,
        d.abstained,
        d.ties,
        d.p_value(),
        verdict(d.gateable(), d.passes())
    );
    let _ = writeln!(
        out,
        "- **{}**",
        if bars.selectable() {
            "selectable"
        } else {
            "not selectable"
        }
    );
}

/// MP-243's bars for every K variant in `variants`, and each one against C0
/// when C0 ran. Empty when no K variant ran, so other experiments' runs never
/// build pairs.
///
/// # Panics
/// As [`bars`].
pub(crate) fn render_bars(rows: &[Row], output: &RunOutput, variants: &[&Variant]) -> String {
    let mut out = String::new();
    let c0 = variants.iter().find(|variant| variant.id == "C0");
    for variant in variants
        .iter()
        .filter(|variant| K_VARIANTS.iter().any(|k| k.id == variant.id))
    {
        let label = variant.label();
        render_one(&mut out, rows, output, &label);
        if let Some(c0) = c0 {
            let joint = joint(rows, output, &label, &c0.label(), SOUND);
            let _ = writeln!(
                out,
                "- vs {} on `{SOUND}`, {} of {} natural rows answered by both \
                 (abstentions {} / {}; ceiling {:.0}%{}): {label} {} | {} {}",
                c0.label(),
                joint.both,
                joint.labelled,
                joint.abstentions.0,
                joint.abstentions.1,
                ABSTENTION_CEILING * 100.0,
                if joint.interpretable() {
                    ""
                } else {
                    ", NOT INTERPRETABLE"
                },
                show(&joint.left),
                c0.label(),
                show(&joint.right),
            );
        }
    }
    out
}
