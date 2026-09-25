// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Acceptance-criterion refusal checks R0 and R1 (PLAT-1024 experiment 4).
//! Dev only; informational, gates nothing.
//!
//! # The unit
//!
//! One acceptance criterion's own text, without its requirement.
//! [`criterion_unit`] reads it; a row with no criterion text has no unit, is
//! not asked, and is not labelled.
//!
//! # The checks
//!
//! The `spec-correctness` skill's clause-grounding step lists the reasons a
//! criterion cannot be grounded as a property. [`CHECKS`] asks the five of
//! them that the criterion's text alone decides: `oracle-is-adjectival`,
//! `domain-unbounded`, `singleton-domain` (here `names_single_witness`),
//! `criterion-describes-its-test` and `static-or-demonstration`. The rest
//! need the source (`symbol-not-found`, `ambiguous-symbol`,
//! `unimplemented`), the classifier's label (`label-from-mention`,
//! `no-state-machine`) or the record (`no-row-id`).
//!
//! # The variants
//!
//! | id | asks | derives |
//! | --- | --- | --- |
//! | `R0` | one holistic `noul`: can the criterion be checked as a property? | `criterion_groundable` = `P(yes) >= 0.5` |
//! | `R1` | one `noul` per refusal reason in [`CHECKS`] | each check at 0.5; `criterion_groundable` by [`R1_COMBINER`] |
//!
//! [`PRE_REGISTERED`] names the two combiners fixed before the first live
//! call; every other combiner in a report is post hoc.
//!
//! # Truth
//!
//! Natural rows are one AGENT-LABELLED pass (`labels-ac-refusal.json`, kind
//! `agent_single`). Mutants of kind [`MUTATION_KIND`] are dev-only and carry
//! by-construction truth: the injected check `yes` and `criterion_groundable`
//! `no`.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde_json::{Value, json};
use typesafe_sdk_questions::{Entry, NoulCriteria, Question, Questions, noul_with};

use crate::eval_v2_support::corpus::Row;
use crate::eval_v2_support::keys::{Mode, NO, YES};
use crate::eval_v2_support::variant::{
    Answered, Ask, Prediction, Predictions, RunOutput, Variant, request, whole_row,
};
use crate::eval_v2_support::variants::soundness::{TAU, defect_prediction, required_noul};
use crate::eval_v2_support::variants::statement::{
    Battery, Check, Combiner, RuleLine, holistic_scores, probability_table,
};

/// The aggregate key: `yes` when no refusal reason applies.
pub(crate) const SOUND: &str = "criterion_groundable";
/// The mutation kind whose rows inject one refusal reason.
pub(crate) const MUTATION_KIND: &str = "ac_refusal";
/// R0's wire key.
pub(crate) const CHECKABLE: &str = "property_checkable";
/// The state field carrying the unit.
pub(crate) const CRITERION_FIELD: &str = "criterion";

/// Every instruction opens with this sentence.
macro_rules! judge {
    ($question:literal) => {
        concat!(
            "Judge the text in `criterion`, read literally and on its own. It is one acceptance \
             criterion of a requirement, shown without the requirement. A property-based check \
             draws inputs from a stated domain and decides pass or fail with a concrete oracle on \
             the observed result. ",
            $question
        )
    };
}

/// The five refusal reasons the criterion's text decides. The `defect` and
/// `clear` text is also the labelling rule the natural rows were labelled
/// to, word for word. Every example in them is synthetic.
pub(crate) const CHECKS: [Check; 5] = [
    Check {
        key: "oracle_is_adjectival",
        instruction: judge!(
            "Is what the criterion requires stated as a quality word, such as 'appropriate', \
             'reasonable', 'clear', 'actionable', 'gracefully' or 'acceptably', with no \
             observable result that decides it?"
        ),
        defect: "The required outcome, in the whole criterion or in any one of its clauses, is a \
                 quality word or phrase ('handled appropriately', 'behaves reasonably', 'a clear \
                 message', 'actionable', 'gracefully', 'acceptably') and no observable result is \
                 given that decides it.",
        clear: "Every required outcome is an observable result: a value returned, printed or \
                recorded, an exit status, an error or reason identifier, a refusal, an equality \
                with another output, or a presence or absence. A quality word next to such a \
                result, or one the criterion itself defines, is clear.",
    },
    Check {
        key: "domain_unbounded",
        instruction: judge!(
            "Does the criterion leave the inputs or cases it covers open, so that nothing in it \
             says which inputs a generator should draw?"
        ),
        defect: "The inputs or cases the criterion, or any one of its clauses, covers are open or \
                 given only by a vague qualifier ('any input', 'all situations', 'edge cases', \
                 'unusual data', 'a bad value', 'in inappropriate circumstances'), with no stated \
                 condition, list or example that says which ones.",
        clear: "Every clause names what its inputs or cases are: a kind of thing with a stated \
                condition ('an invoice with a negative total'), a kind of thing with no \
                qualifier, meaning every member of it ('every uploaded file'), an enumerated \
                list, or an example. A criterion that takes no inputs is clear here.",
    },
    Check {
        key: "names_single_witness",
        instruction: judge!(
            "Is the criterion about exactly one specific case (one named input, value, file, \
             invocation or instance) rather than a class of cases?"
        ),
        defect: "The criterion, or any one of its clauses, is about one specific case: a single \
                 named input, value, file, fixture, command invocation or instance ('the \
                 `README.md` in `docs/`', 'running `billing --version`', 'the `gold` pricing \
                 tier'), so there is exactly one case to check and it is a witness, not a \
                 property.",
        clear: "Every clause is about a class with more than one member, including one \
                introduced by 'a', 'an', 'any', 'every', 'each', 'all' or 'no' ('an expired \
                session token', 'every uploaded file'), or a named set with several members \
                ('the three pricing tiers').",
    },
    Check {
        key: "describes_its_own_test",
        instruction: judge!(
            "Is the criterion written as a description of the check that verifies it, naming \
             the verification method (a proptest, a fuzz run, an audit, a grep, a demonstration) \
             rather than only the behaviour required?"
        ),
        defect: "The criterion, or any one of its clauses, names how it is verified: a proptest, \
                 fuzz run, audit, grep, benchmark, round-trip check, fixture run or demonstration \
                 is its subject or is cited as the proof ('a proptest over generated carts \
                 confirms ...', 'a static audit (`rg` for `unsafe`) shows ...', 'demonstrated by \
                 deploying ...').",
        clear: "The criterion states the required behaviour or fact without naming how it is \
                verified. Describing the inputs or conditions ('with the cache cleared', 'on a \
                leap day') is not naming a verification method; stating that two runs or \
                two implementations give the same output is a required behaviour; and a \
                statement about which checks a repository contains is a fact about the \
                repository.",
    },
    Check {
        key: "static_or_demonstration",
        instruction: judge!(
            "Is what the criterion requires a fact about the source tree or repository, or an \
             end-to-end narrative of someone performing several steps, rather than a result the \
             running system gives for an input?"
        ),
        defect: "What the criterion, or any one of its clauses, requires is a fact about the \
                 source tree or repository (which files, modules, dependencies, crates, \
                 binaries, names or checks exist, are absent, are unchanged or are structured a \
                 certain way: 'the parser is built on the `nom` crate', 'no module outside \
                 `auth/` reads the password table'), or it is an end-to-end narrative of an actor \
                 performing several steps ('an operator installs the plugin, opens the \
                 dashboard, sees the new panel and exports it').",
        clear: "Every required outcome is a result the running system gives for an input or \
                event: an output, a returned value, an exit status, a refusal, a record written, \
                or an equality between two runs' outputs. A prohibition on runtime behaviour \
                ('SHALL NOT round a submitted price') is clear when the behaviour shows in a \
                result for an input.",
    },
];

/// The labelling rule for [`SOUND`].
pub(crate) const SOUND_RULE: &str = "yes when the criterion's own text can be grounded as a \
     property: none of the five refusal checks says yes; no when any of them says yes.";

/// Every key R1 is graded on: [`SOUND`] and each check.
pub(crate) const GRADES: [&str; CHECKS.len() + 1] = {
    let [a, b, c, d, e] = CHECKS;
    [SOUND, a.key, b.key, c.key, d.key, e.key]
};

/// The refusal battery, for bar D and the report tables.
pub(crate) const REFUSAL: Battery = Battery {
    sound: SOUND,
    checks: &CHECKS,
    mutation_kind: MUTATION_KIND,
};

/// R0's holistic question.
pub(crate) const HOLISTIC: &str = judge!(
    "Can this criterion be checked as a property: does it state a domain of inputs and a \
     concrete oracle that decides pass or fail on the observed result?"
);

// ---------------------------------------------------------------------------
// The unit, asks and derivation
// ---------------------------------------------------------------------------

/// The row's unit: its acceptance criterion's text, or `None` when the row
/// has none.
pub(crate) fn criterion_unit(row: &Row) -> Option<String> {
    row.requirement
        .ac_text
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

/// The state: the criterion's id and its unit, nothing else.
pub(crate) fn criterion_state(row: &Row, unit: &str) -> Value {
    json!({
        "criterion_id": row.requirement.ac_id,
        CRITERION_FIELD: unit,
    })
}

fn noul(instruction: &str, yes: &str, no: &str) -> Question {
    noul_with(
        instruction,
        NoulCriteria {
            yes: Some(Entry::from(yes)),
            no: Some(Entry::from(no)),
        },
    )
}

/// R1 v2's `domain_unbounded` question (POST HOC, chosen after dev run 1).
/// On dev, v1 fired on 19 of 37 criteria labelled groundable; of 10 read by
/// hand, 8 named their inputs (a kind of thing with a condition, a list, or
/// every member of a kind) and were flagged for having infinitely many
/// members. The labelling rule already calls a described kind of input
/// clear; v2 says in the question that an infinite domain is not an open
/// one. The yes/no criteria are unchanged.
pub(crate) const DOMAIN_UNBOUNDED_V2: &str = judge!(
    "Does the criterion fail to say what its inputs or cases are, leaving them unstated or \
     described only by a vague qualifier such as 'any input', 'edge cases' or 'unusual data'? A \
     domain may be infinite: a described kind of input ('an invoice with a negative total', \
     'every uploaded file', 'a request whose token has expired') says what a generator should \
     draw even though it has many members, and a criterion that takes no inputs is not open."
);

/// R1 v2's `static_or_demonstration` question (POST HOC, chosen after dev
/// run 1). On dev, v1 fired on 7 of 37 criteria labelled groundable; read by
/// hand, 6 required a result of running the system (loading, discovering,
/// selecting or reporting something) about a subject that lives in the
/// repository (manifests, commands, suites). The labelling rule already
/// judges what the criterion requires; v2 says in the question that the
/// subject living in the repository does not make the requirement a fact
/// about the source tree. The yes/no criteria are unchanged.
pub(crate) const STATIC_V2: &str = judge!(
    "Is what the criterion requires a fact about the source tree or repository (which files, \
     modules, dependencies, names or checks exist, are absent or are structured a certain way), \
     or an end-to-end narrative of someone performing several steps? Judge what must be observed \
     for it to pass, not what it is about: a criterion whose subject lives in the repository (a \
     file, a manifest, a command, a module) but which requires a result of running the system, \
     such as loading, discovering, selecting, reporting or refusing something, is not a fact \
     about the source tree."
);

/// R1's instruction for `check`: v1's, except the two POST HOC rewordings.
fn r1_instruction(check: &Check) -> &'static str {
    match check.key {
        "domain_unbounded" => DOMAIN_UNBOUNDED_V2,
        "static_or_demonstration" => STATIC_V2,
        _ => check.instruction,
    }
}

fn r1_questions() -> Questions {
    CHECKS
        .iter()
        .map(|check| {
            (
                check.key.to_owned(),
                noul(r1_instruction(check), check.defect, check.clear),
            )
        })
        .collect()
}

fn r0_questions() -> Questions {
    std::iter::once((
        CHECKABLE.to_owned(),
        noul(
            HOLISTIC,
            "The criterion states a domain of inputs and a concrete oracle, so a property-based \
             check can be written from it.",
            "It does not.",
        ),
    ))
    .collect()
}

fn asks_with(row: &Row, questions: fn() -> Questions) -> Vec<Ask> {
    criterion_unit(row)
        .map(|unit| Ask {
            unit: None,
            request: request(criterion_state(row, &unit), questions()),
        })
        .into_iter()
        .collect()
}

fn r1_asks(row: &Row) -> Vec<Ask> {
    asks_with(row, r1_questions)
}

fn r0_asks(row: &Row) -> Vec<Ask> {
    asks_with(row, r0_questions)
}

/// R1's graded combiner: any check at 0.5.
pub(crate) const R1_COMBINER: Combiner = Combiner {
    at_least: 1,
    tau: TAU,
};

/// The combiners fixed before the first live call. Both are primary lines;
/// R1's graded answers use the first.
pub(crate) const PRE_REGISTERED: [(&str, Combiner); 2] = [
    ("any >= 0.5", R1_COMBINER),
    (
        "any >= 0.7",
        Combiner {
            at_least: 1,
            tau: 0.7,
        },
    ),
];

/// Combiners recomputed from the same answers after the run: post hoc.
pub(crate) const POST_HOC: [(&str, Combiner); 1] = [(
    ">= 2 >= 0.5",
    Combiner {
        at_least: 2,
        tau: TAU,
    },
)];

/// R1's graded answers from one defect probability per check, in [`CHECKS`]
/// order.
pub(crate) fn derive_checks(probabilities: &[f64; CHECKS.len()]) -> Predictions {
    let mut out: Predictions = CHECKS
        .iter()
        .zip(probabilities)
        .map(|(check, p)| (check.key, defect_prediction(*p)))
        .collect();
    out.insert(SOUND, R1_COMBINER.sound(probabilities));
    out
}

/// Each check's probability on a row R1 answered, in [`CHECKS`] order.
pub(crate) fn r1_probabilities(row: &Row, answered: &[Answered]) -> Option<[f64; CHECKS.len()]> {
    if answered.is_empty() {
        return None;
    }
    let answers = whole_row(answered);
    Some(CHECKS.map(|check| required_noul(&row.id, &answers, check.key)))
}

fn r1_derive(row: &Row, answered: &[Answered]) -> Predictions {
    r1_probabilities(row, answered)
        .map(|p| derive_checks(&p))
        .unwrap_or_default()
}

/// R0's `P(checkable)` on a row it answered.
pub(crate) fn r0_probability(row: &Row, answered: &[Answered]) -> Option<f64> {
    (!answered.is_empty()).then(|| required_noul(&row.id, &whole_row(answered), CHECKABLE))
}

fn r0_derive(row: &Row, answered: &[Answered]) -> Predictions {
    r0_probability(row, answered)
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
pub(crate) const R0: Variant = Variant {
    id: "R0",
    version: 1,
    summary: "acceptance criterion: one holistic noul (checkable as a property: a stated domain and a concrete oracle?)",
    modes: &Mode::ALL,
    references: &[],
    grades: &[SOUND],
    asks: r0_asks,
    derive: r0_derive,
};

/// The battery: one noul per refusal reason, combined in code.
pub(crate) const R1: Variant = Variant {
    id: "R1",
    // v2 (POST HOC, after dev run 1): the `domain_unbounded` and
    // `static_or_demonstration` questions are DOMAIN_UNBOUNDED_V2 and
    // STATIC_V2.
    version: 2,
    summary: "acceptance criterion: five refusal-reason nouls (domain_unbounded allows infinite described domains; static_or_demonstration judges what must be observed); criterion_groundable = no check at or above 0.5",
    modes: &Mode::ALL,
    references: &[],
    grades: &GRADES,
    asks: r1_asks,
    derive: r1_derive,
};

/// Whether `row` is a refusal mutant: it carries truth for the refusal keys
/// only.
pub(crate) fn is_refusal_mutant(row: &Row) -> bool {
    REFUSAL.is_mutant(row)
}

// ---------------------------------------------------------------------------
// The report (informational)
// ---------------------------------------------------------------------------

fn r1_table<'a>(rows: &'a [Row], output: &RunOutput, label: &str) -> BTreeMap<&'a str, Vec<f64>> {
    probability_table(rows, output, label, |row, answered| {
        r1_probabilities(row, answered).map(|p| p.to_vec())
    })
}

/// The report for every R variant in `variants`: R0, R1's two pre-registered
/// combiners (pre-registered only at R1's first version; any later wording
/// makes them post hoc), R1's post-hoc combiners, and R1's per-check tables
/// at 0.5 and 0.7. Empty when no R variant ran.
pub(crate) fn render(rows: &[Row], output: &RunOutput, variants: &[&Variant]) -> String {
    let mut out = String::new();
    let ran = |id: &str| variants.iter().find(|variant| variant.id == id);
    if ran("R0").is_none() && ran("R1").is_none() {
        return out;
    }
    let all = REFUSAL.labelled(rows);
    let _ = writeln!(
        out,
        "\n## Acceptance-criterion refusal reasons ({SOUND}; dev only, informational)\n\n{} \
         labelled rows: {} natural (AGENT-LABELLED, single pass), {} refusal mutants (by \
         construction). Credit counts an alternative reading as half. Bar D: {MUTATION_KIND} \
         mutant vs its source, the shared pair rule (cross the rule's threshold, move at least \
         0.10).\n\n{}",
        all.len(),
        all.iter().filter(|row| row.mutation.is_none()).count(),
        all.iter().filter(|row| row.mutation.is_some()).count(),
        REFUSAL.rule_header(),
    );
    if let Some(r0) = ran("R0") {
        let label = r0.label();
        REFUSAL.render_rule(
            &mut out,
            rows,
            &RuleLine {
                name: format!("{label} holistic P(no) >= 0.5 (baseline)"),
                scores: holistic_scores(rows, output, &label, r0_probability),
                tau: TAU,
            },
        );
    }
    let Some(r1) = ran("R1") else {
        return out;
    };
    let label = r1.label();
    let table = r1_table(rows, output, &label);
    let lines = PRE_REGISTERED
        .iter()
        .map(|line| (line, true))
        .chain(POST_HOC.iter().map(|line| (line, false)));
    for ((name, combiner), registered) in lines {
        let tag = match (registered, r1.version) {
            (true, 1) => " (pre-registered)".to_owned(),
            (true, version) => {
                format!(" (pre-registered combiner; POST HOC wording, v{version})")
            }
            (false, _) => " (POST HOC)".to_owned(),
        };
        REFUSAL.render_rule(
            &mut out,
            rows,
            &RuleLine {
                name: format!("{label} {name}{tag}"),
                scores: table
                    .iter()
                    .map(|(id, p)| (*id, combiner.score(p)))
                    .collect(),
                tau: combiner.tau,
            },
        );
    }
    for tau in [TAU, 0.7] {
        REFUSAL.render_checks(&mut out, rows, &table, tau);
    }
    out
}

/// One line per labelled row: every R1 check's probability, R0's defect
/// score, the row's truth and its unit, for reading flagged rows by hand.
pub(crate) fn dump(rows: &[Row], output: &RunOutput, variants: &[&Variant]) -> Vec<String> {
    let r0 = variants
        .iter()
        .find(|variant| variant.id == "R0")
        .map(|variant| holistic_scores(rows, output, &variant.label(), r0_probability))
        .unwrap_or_default();
    let Some(r1) = variants.iter().find(|variant| variant.id == "R1") else {
        return Vec::new();
    };
    let table = r1_table(rows, output, &r1.label());
    REFUSAL.dump(rows, &table, &r0, criterion_unit)
}
