// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The four gap-analysis battery questions PLAT-839 left ungraded on the
//! shipped corpus, graded against agent-labelled ground truth (PLAT-1014).
//!
//! `code_implements_intent`, `code_exceeds_requirement`, `divergence_kind` and
//! `severity` are asked on every live call of the gap-analysis lens. MP-233
//! graded them against targeted mutants of a sample of triples. Nobody has
//! graded what they say about the 29 triples as they actually ship. This file
//! does, against `fixtures/gap-semantic-remaining4-labels.json`. It is
//! evaluation-only, behind the non-default `live-api` feature, and changes
//! nothing in `quoin-jev`'s `src/` or in `skills/gap-analysis/`:
//!
//! ```bash
//! cd rust && cargo test -p quoin-jev --features live-api --test live_gap_remaining4 -- --nocapture
//! ```
//!
//! # The ground truth is agent-labelled
//!
//! One Claude agent session wrote every label by reading each triple's
//! requirement, test and code, before this file made its first live call.
//! It is not human ground truth, and unlike MP-232's `assertion_vacuous`
//! truth nothing in it was checked against a compiler or a test runner. The
//! labelling rule for each question is stated in the fixture's own `rules`,
//! and every row carries a `rationale`. Where a second reading is defensible
//! the fixture records it as contested, and a lens answer matching it counts
//! as agreement, exactly as the criterion-strength and EARS corpora do.
//!
//! **What this corpus makes these questions measure.** Eighteen of the 29
//! triples carry a `Trace:` tag naming a requirement that describes a
//! different subsystem or a process step (16 FR-101 rows, plus `GAP-02` and
//! `GAP-27`), which the corpus's own `note`s already recorded before PLAT-839's
//! first live call. On those rows the labels are `code_implements_intent: no`,
//! `code_exceeds_requirement: yes`, `divergence_kind: code_exceeds_requirement`
//! and `severity: high`. So on this corpus, all four questions are dominated
//! by one skill: noticing that the cited requirement is not about this code.
//! That is a real gap-analysis finding (step 5's rubric names it as a `high`),
//! but a pass here is not evidence the lens judges code-vs-requirement
//! semantics on triples whose trace is correct. The 11 correctly-traced rows
//! are reported separately so that difference is visible.
//!
//! # Pre-registered bars, fixed and committed before the first live call
//!
//! One pass, one `FullBatteryV1` request per corpus triple (the battery's
//! current wording; these four questions are word-for-word identical in
//! `FullBattery`). Each of the four questions is gated on its own, over its
//! own population, with the shared grading maths in `support/grading.rs`:
//!
//! - **Bar A, margin (MP-222):** agreement, primary plus contested, exceeds
//!   the best constant predictor that population admits, computed from the
//!   labels by `trivial_baseline`.
//! - **Bar B, defect recall (MP-223):** above 0 — of the rows whose label is a
//!   defect, the lens flags at least one as some defect.
//! - **Bar C, no-defect recall (MP-224):** above 0 — of the rows whose label is
//!   the no-defect answer, the lens clears at least one.
//!
//! Defect / no-defect classes: `code_implements_intent` `no` / `yes`;
//! `code_exceeds_requirement` `yes` / `no`; `divergence_kind` anything but
//! `aligned` / `aligned`; `severity` graded at its verdict (`high` -> `FAIL`,
//! `medium`/`low` -> `CONDITIONAL`, else `PASS`), anything but `PASS` /
//! `PASS`. `divergence_kind` is gated as ASKED. The label derived from the
//! same response's `noul` answers by MP-233's rule is reported, not gated:
//! choosing the better of two predictors after seeing both is a free point.
//!
//! Reported, never gated: the same four under `FullBattery`, which
//! replicates the question set MP-230's GO was measured on; `severity` at its
//! exact rubric level; the clean-tier subsets; the 11 correctly-traced rows
//! alone; calibration of the two `noul` answers.
//!
//! A bar that fails is reported as failing, and every bar is evaluated before
//! the test fails, so one NO-GO never hides the others.

#![cfg(feature = "live-api")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod gap_battery_support;
mod gap_remaining4_support;
mod gap_semantic_support;

use std::collections::BTreeMap;

use typesafe_sdk_client::Client;
use typesafe_sdk_env::Process;

use gap_battery_support::{BatteryAnswers, answers};
use gap_remaining4_support::grading::{
    Graded, defect_recall, no_defect_recall, report, tally, trivial_baseline,
};
use gap_remaining4_support::{Grades, gated, grade, labels};
use gap_semantic_support::{GapTriple, Variant, build_request, corpus};
use quoin_jev::JevErrorCode;

/// Builds a client against the real service. Panics rather than skipping when
/// no key resolves — same discipline as the sibling live files: a silent skip
/// is how a suite reports green over a measurement that never ran.
fn live_client() -> Client {
    let config = quoin_jev::config::resolve(&Process).unwrap_or_else(|error| {
        assert_eq!(
            error.code,
            JevErrorCode::MissingKey,
            "unexpected config failure: {error}"
        );
        panic!(
            "the `live-api` feature is on but no API key resolved. This test \
             does not skip. Set TYPESAFE_API_KEY, or drop --features live-api."
        );
    });
    quoin_jev::client::production(config).expect("builds a real transport")
}

/// One `variant` request per triple, answers indexed by triple id. A
/// transport failure aborts the pass rather than being scored, so a triple
/// the lens fails to answer cannot quietly leave the denominator.
async fn ask_all(
    client: &Client,
    triples: &[GapTriple],
    variant: Variant,
) -> (BTreeMap<String, BatteryAnswers>, BTreeMap<String, String>) {
    let mut said = BTreeMap::new();
    let mut models = BTreeMap::new();
    for triple in triples {
        let response = client
            .system_one(build_request(triple, variant))
            .await
            .unwrap_or_else(|error| panic!("{}: {error}", triple.id));
        *models.entry(response.model.clone()).or_insert(0usize) += 1;
        said.insert(triple.id.clone(), answers(&response));
    }
    let models = models
        .into_iter()
        .map(|(model, count)| (model, count.to_string()))
        .collect();
    (said, models)
}

/// Records one bar's verdict instead of panicking on the spot, then fails the
/// test at the end naming every bar that did not hold.
#[derive(Debug, Default)]
struct Bars {
    failed: Vec<String>,
}

impl Bars {
    fn check(&mut self, held: bool, bar: &str, why: &str) {
        println!("{} {bar}: {why}", if held { "PASS" } else { "FAIL" });
        if !held {
            self.failed.push(format!("{bar}: {why}"));
        }
    }

    /// Bars A, B and C for one question's population.
    fn question(&mut self, name: &str, rows: &[Graded], no_defect: &str) {
        let counts = tally(rows);
        let agreement = counts.agreement().unwrap_or(0.0);
        let (baseline_label, baseline) = trivial_baseline(rows);
        self.check(
            agreement > baseline,
            &format!("{name} BAR A margin"),
            &format!(
                "agreement {agreement:.1}% ({}/{}) vs. constant predictor `{baseline_label}` \
                 {baseline:.1}%, margin {:+.1}pp ({} unanswered, {} unrecognized)",
                counts.agreed(),
                counts.total(),
                agreement - baseline,
                counts.unanswered,
                counts.unrecognized,
            ),
        );
        let defects = defect_recall(rows, no_defect);
        self.check(
            defects.is_some_and(|value| value > 0.0),
            &format!("{name} BAR B defect recall"),
            &format!("{defects:?} (no-defect label `{no_defect}`)"),
        );
        let clears = no_defect_recall(rows, no_defect);
        self.check(
            clears.is_some_and(|value| value > 0.0),
            &format!("{name} BAR C no-defect recall"),
            &format!("{clears:?} (no-defect label `{no_defect}`)"),
        );
    }

    fn settle(self) {
        assert!(
            self.failed.is_empty(),
            "{} pre-registered bar(s) did not hold:\n  - {}",
            self.failed.len(),
            self.failed.join("\n  - ")
        );
    }
}

/// The ids whose trace tag names a requirement that does describe the
/// covered code: the rows NOT labelled as a trace mismatch.
fn correctly_traced(grades: &Grades) -> Vec<String> {
    grades
        .divergence_asked
        .iter()
        .filter(|row| row.expected != "code_exceeds_requirement")
        .map(|row| row.fixture_id.clone())
        .collect()
}

/// The one-line summary of a population: agreement, baseline, margin, both
/// recalls. Ungated.
fn summary(name: &str, rows: &[Graded], no_defect: &str) -> String {
    let counts = tally(rows);
    let agreement = counts.agreement().unwrap_or(0.0);
    let (label, baseline) = trivial_baseline(rows);
    let show = |value: Option<f64>| value.map_or_else(|| "n/a".to_owned(), |v| format!("{v:.1}%"));
    format!(
        "| {name} | {} | {agreement:.1}% | `{label}` {baseline:.1}% | {:+.1}pp | {} | {} |",
        rows.len(),
        agreement - baseline,
        show(defect_recall(rows, no_defect)),
        show(no_defect_recall(rows, no_defect)),
    )
}

fn print_summary(title: &str, grades: &Grades, only: Option<&[String]>) {
    let keep = |rows: &[Graded]| -> Vec<Graded> {
        rows.iter()
            .filter(|row| only.is_none_or(|ids| ids.contains(&row.fixture_id)))
            .cloned()
            .collect()
    };
    println!("\n### {title}\n");
    println!(
        "| Question | Rows | Agreement | Constant predictor | Margin | Defect recall | No-defect recall |"
    );
    println!("| --- | --- | --- | --- | --- | --- | --- |");
    for (name, rows, no_defect) in gated(grades) {
        println!("{}", summary(name, &keep(rows), no_defect));
    }
    println!(
        "{}",
        summary(
            "divergence_kind (derived, ungated)",
            &keep(&grades.divergence_derived),
            "aligned"
        )
    );
    println!(
        "{}",
        summary(
            "severity (exact level, ungated)",
            &keep(&grades.severity_level),
            "none"
        )
    );
}

/// **The gate.** Provenance: PLAT-1014, PLAT-839. Grades the four questions
/// against the agent-labelled key and checks every bar pre-registered in this
/// file's module doc.
#[tokio::test]
async fn the_remaining_four_questions_beat_doing_nothing() {
    let client = live_client();
    let triples = corpus();
    let key = labels();

    let (v1, v1_models) = ask_all(&client, &triples, Variant::FullBatteryV1).await;
    let (v0, v0_models) = ask_all(&client, &triples, Variant::FullBattery).await;
    println!("models answering, FullBatteryV1: {v1_models:?}; FullBattery: {v0_models:?}");

    let gated_grades = grade(&key, &v1);
    let replication = grade(&key, &v0);

    for (name, rows, no_defect) in gated(&gated_grades) {
        println!(
            "{}",
            report(&format!("{name} — FullBatteryV1 (gated)"), rows, no_defect)
        );
    }
    println!(
        "{}",
        report(
            "divergence_kind derived from the noul answers — FullBatteryV1 (ungated)",
            &gated_grades.divergence_derived,
            "aligned"
        )
    );
    println!(
        "{}",
        report(
            "severity at its exact rubric level — FullBatteryV1 (ungated)",
            &gated_grades.severity_level,
            "none"
        )
    );

    print_summary("FullBatteryV1, all rows (gated)", &gated_grades, None);
    let traced = correctly_traced(&gated_grades);
    print_summary(
        &format!(
            "FullBatteryV1, the {} correctly-traced rows only (ungated)",
            traced.len()
        ),
        &gated_grades,
        Some(&traced),
    );
    let clean: Vec<String> = key
        .iter()
        .filter(|label| {
            label.divergence_kind_contested.is_empty() && label.severity_contested.is_empty()
        })
        .map(|label| label.id.clone())
        .collect();
    print_summary(
        &format!(
            "FullBatteryV1, the {} rows with no contested reading (ungated)",
            clean.len()
        ),
        &gated_grades,
        Some(&clean),
    );
    print_summary("FullBattery (v0), all rows (ungated)", &replication, None);

    let mut bars = Bars::default();
    for (name, rows, no_defect) in gated(&gated_grades) {
        assert!(
            rows.len() >= 20,
            "{name}: population below the pre-registered minimum of 20: {}",
            rows.len()
        );
        bars.question(name, rows, no_defect);
    }
    bars.settle();
}
