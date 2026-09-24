// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Corpus v2's own offline check (PLAT-1025). Runs in the default gate: no
//! feature flag, no socket, no key. It reads git objects (the assembly quotes
//! files at `assemble::CONTENT_COMMIT`), so it needs a clone with that commit.
//!
//! The harness (`tc_1027_the_committed_corpus_validates`) checks the schema
//! any corpus must meet. This file checks what PLAT-1025 promised about this
//! one: `corpus.json` is exactly the assembly of its committed inputs, every
//! mechanical or by-construction label is backed by the mutation log, the
//! held-out seal still matches, the per-requirement cap on natural rows holds,
//! every agent label says it is one, and the size and mutation share are what
//! the PR reports.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod eval_v2_support;
mod gap_semantic_support;

use std::collections::{BTreeMap, BTreeSet};

use eval_v2_support::assemble::{
    CONTENT_COMMIT, GitContent, Inputs, corpus_bytes, fixture_dir, labelling_rules, repo_root,
};
use eval_v2_support::corpus::{
    Origin, Row, Source, TruthKind, load_in_repo, validate, verify_seal,
};
use eval_v2_support::criterion_defects::CRITERION_DEFECTS;
use eval_v2_support::keys::KEYS;
use serde_json::Value;

fn corpus() -> Source {
    load_in_repo()
        .expect("corpus.json parses against the harness schema")
        .expect("PLAT-1025 commits corpus.json")
}

/// Trace: FR-110
/// Provenance: PLAT-1025. The file parses, ids are unique, every truth
/// answer is legal for its key and fits the row's mode, and every mutant's
/// source is an unmutated row of the same mode and split.
#[test]
fn tc_1025_the_corpus_is_valid_against_the_shared_schema() {
    let source = corpus();
    let problems = validate(&source.file, Origin::InRepo);
    assert!(problems.is_empty(), "{problems:#?}");
    let mut ids = BTreeSet::new();
    for row in &source.file.rows {
        assert!(ids.insert(row.id.clone()), "duplicate id {}", row.id);
    }
}

/// Provenance: PLAT-1025. The held-out split was sealed in the same commit as
/// the corpus, before any live call; an edit to a held-out row breaks this.
#[test]
fn tc_1025_the_heldout_seal_matches() {
    verify_seal(&corpus()).unwrap_or_else(|error| panic!("{error}"));
}

/// Provenance: PLAT-1025, PR #628 review. `corpus.json` is byte for byte what
/// the builder's assembly makes from the committed inputs (`sample.json`,
/// `mutations.json`, the two label passes, and the quoted files at
/// `CONTENT_COMMIT`). A hand edit to any row, label or header fails here.
#[test]
fn tc_1025_the_corpus_is_the_assembly_of_its_committed_inputs() {
    let inputs = Inputs::load(&fixture_dir());
    let content = GitContent::new(&repo_root(), CONTENT_COMMIT);
    let rebuilt = corpus_bytes(&eval_v2_support::assemble::assemble(&inputs, &content));
    let committed = std::fs::read_to_string(fixture_dir().join("corpus.json")).unwrap();
    if rebuilt != committed {
        let a: Value = serde_json::from_str(&rebuilt).unwrap();
        let b: Value = serde_json::from_str(&committed).unwrap();
        let first = a["rows"]
            .as_array()
            .unwrap()
            .iter()
            .zip(b["rows"].as_array().unwrap())
            .find(|(x, y)| x != y)
            .map(|(x, _)| x["id"].clone());
        panic!(
            "corpus.json is not the assembly of its inputs (rows {} rebuilt vs {} committed; \
             first differing row {first:?}; header equal: {}). Rerun eval_v2_3_assemble.",
            a["rows"].as_array().unwrap().len(),
            b["rows"].as_array().unwrap().len(),
            a["labelling_rules"] == b["labelling_rules"]
                && a["sampling_rule"] == b["sampling_rule"]
        );
    }
}

/// The mutation log, by id.
fn mutation_log() -> BTreeMap<String, Value> {
    let text = std::fs::read_to_string(fixture_dir().join("mutations.json")).unwrap();
    let log: Value = serde_json::from_str(&text).unwrap();
    log["mutations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| (m["id"].as_str().unwrap().to_owned(), m.clone()))
        .collect()
}

/// The sample's natural id (`N-…`) of an unmutated row: the natural entry
/// whose test and code are the row's (tests and code symbols are unique
/// across natural rows; an R row is its criterion). An RC row keeps its
/// source's test in `sample.json`, so an RT row can match an RC natural of
/// the same test too; the natural of the row's own mode wins, and another
/// mode's is taken only for a projection, which has no natural of its own.
fn natural_id(row: &Row, sample: &Value) -> String {
    let natural = sample["natural"].as_array().unwrap();
    let matches = |n: &&Value| {
        let src = &n["source"];
        let same_test = row.test.as_ref().is_none_or(|t| {
            src["test"]["path"] == t.path.as_str()
                && src["test"]["symbol"]
                    .as_str()
                    .is_some_and(|s| s.rsplit("::").next() == Some(t.fn_name.as_str()))
        });
        let same_code = row.code.as_ref().is_none_or(|c| {
            src["code"]["path"] == c.path.as_str() && src["code"]["symbol"] == c.symbol.as_str()
        });
        let same_crit = n["crit_id"].as_str() == row.requirement.ac_id.as_deref()
            && n["req_id"] == row.requirement.fr_id.as_str();
        same_test && same_code && same_crit
    };
    let hit = natural
        .iter()
        .filter(matches)
        .find(|n| n["mode"] == row.mode.as_str())
        .or_else(|| natural.iter().find(matches));
    hit.and_then(|n| n["id"].as_str())
        .unwrap_or_else(|| panic!("{}: no natural row in sample.json", row.id))
        .to_owned()
}

/// A violating mutant whose run is evidence about what its test checks.
fn qualifying_violator(m: &Value) -> bool {
    m["kind"] == "violating_code"
        && m["outcome"] != "dropped"
        && m["breaks_stated_behaviour"] == true
        && (m["outcome"] == "survived" || (m["outcome"] == "caught" && m["failure"] == "assertion"))
}

/// Provenance: PLAT-1025, PR #628 review finding 7. Every mechanical or
/// by-construction label is backed by the mutation log: a mutant row's by
/// its own live entry, with the recorded outcome mechanical truth needs, and
/// only on a key its kind determines; an unmutated row's only by a violating
/// mutant of its own code that qualifies. Relabelling a row's agent answer as
/// mechanical, or a dropped mutant's row left in, fails here.
#[test]
fn tc_1025_every_known_truth_label_is_backed_by_the_mutation_log() {
    let rows = corpus().file.rows;
    let log = mutation_log();
    let sample: Value =
        serde_json::from_str(&std::fs::read_to_string(fixture_dir().join("sample.json")).unwrap())
            .unwrap();
    let mut problems = Vec::new();
    for row in &rows {
        for (key, truth) in &row.truth {
            if !matches!(
                truth.kind,
                TruthKind::Mechanical | TruthKind::ByConstruction
            ) {
                continue;
            }
            let mechanical = truth.kind == TruthKind::Mechanical;
            let mut say = |why: &str| problems.push(format!("{} {key}: {why}", row.id));
            let Some(mutation) = &row.mutation else {
                if !mechanical || key != "test_asserts_intent" {
                    say(
                        "an unmutated row carries known truth only on a settled test_asserts_intent",
                    );
                    continue;
                }
                let n = natural_id(row, &sample);
                if !log
                    .values()
                    .any(|m| m["source"] == n.as_str() && qualifying_violator(m))
                {
                    say("no qualifying violating mutant of its code backs it");
                }
                continue;
            };
            let Some(entry) = log.get(&mutation.id) else {
                say("its mutation is not in mutations.json");
                continue;
            };
            if entry["outcome"] == "dropped" || !entry["skip"].is_null() {
                say("its mutation is dropped or skipped in mutations.json");
                continue;
            }
            if entry["kind"] != mutation.kind.as_str() {
                say("its kind disagrees with mutations.json");
                continue;
            }
            let kind = mutation.kind.as_str();
            let backed = match (kind, key.as_str(), mechanical) {
                ("violating_code", "code_implements_intent", true) => {
                    entry["outcome"] == "caught" && entry["failure"] == "assertion"
                }
                ("violating_code", "test_asserts_intent", true) => log
                    .values()
                    .any(|m| m["source"] == entry["source"] && qualifying_violator(m)),
                ("test_weakening", "test_asserts_intent", true) => {
                    let pair = entry["pair"].as_str().and_then(|p| log.get(p));
                    entry["pair_outcome_original_test"] == "caught"
                        && entry["pair_outcome_weakened_test"] == "survived"
                        && pair.is_some_and(|p| p["failure"] == "assertion")
                }
                (
                    "violating_code",
                    "code_implements_intent" | "divergence_kind" | "severity",
                    false,
                ) => entry["breaks_stated_behaviour"] == true,
                ("additive_code", "code_exceeds_requirement" | "divergence_kind", false) => {
                    entry["outcome"] == "survived" || entry["outcome"] == "n/a"
                }
                ("test_weakening", "divergence_kind" | "severity", false) => {
                    entry["outcome"] == "survived"
                }
                ("test_weakening", "assertion_vacuous", false) => {
                    entry["all_assertions_removed"] == true
                        && entry["pair_outcome_weakened_test"] == "survived"
                }
                ("requirement_text", "test_asserts_intent" | "severity", false) => {
                    entry["requirement_effect"] == "demands_unchecked"
                }
                ("criterion", k, false) => {
                    (k == "criterion_sound" || entry["defect"] == k)
                        && ["pass_a", "pass_b"]
                            .iter()
                            .all(|p| entry["meets_definition"][p]["answer"] == "yes")
                }
                ("requirement_text", "trace_correct" | "divergence_kind", false)
                | (
                    "trace_swap",
                    "trace_correct" | "test_asserts_intent" | "code_implements_intent" | "severity",
                    false,
                ) => true,
                _ => false,
            };
            if !backed {
                say(&format!(
                    "{:?} truth the log entry {} does not back",
                    truth.kind, mutation.id
                ));
            }
        }
    }
    assert!(problems.is_empty(), "{problems:#?}");
}

/// Provenance: PLAT-1025. At most three natural rows (projections included)
/// cite any one requirement, so no single FR dominates (the v1 corpus had 16
/// of 29 rows on FR-101).
#[test]
fn tc_1025_no_requirement_has_more_than_three_natural_rows() {
    let mut per_fr: BTreeMap<String, usize> = BTreeMap::new();
    for row in corpus().file.rows.iter().filter(|r| r.mutation.is_none()) {
        *per_fr.entry(row.requirement.fr_id.clone()).or_default() += 1;
    }
    let over: Vec<_> = per_fr.iter().filter(|(_, n)| **n > 3).collect();
    assert!(over.is_empty(), "{over:?}");
}

/// Provenance: PLAT-1025, PR #624 review. Agent-labelled truth always says
/// so; the header states the rule every label was written against, and its
/// five soundness rules are the shared `criterion_defects` text, word for word.
#[test]
fn tc_1025_agent_labels_say_they_are_agent_labels() {
    let source = corpus();
    let rules = &source.file.labelling_rules;
    assert!(rules["_provenance"].contains("AGENT-LABELLED"));
    assert!(rules["_provenance"].contains("not human ground truth"));
    for spec in &KEYS {
        assert!(
            rules.contains_key(spec.key),
            "no labelling rule for {}",
            spec.key
        );
    }
    for check in &CRITERION_DEFECTS {
        assert_eq!(rules[check.key], check.rule(), "{}", check.key);
    }
    assert_eq!(
        serde_json::to_value(rules).unwrap(),
        labelling_rules(),
        "the header's rules are the builder's"
    );
    for row in &source.file.rows {
        for (key, truth) in &row.truth {
            if matches!(truth.kind, TruthKind::AgentDual | TruthKind::AgentContested) {
                assert!(
                    truth.rationale.contains("AGENT-LABELLED"),
                    "{} {key}: agent truth without the label",
                    row.id
                );
            }
        }
    }
}

/// Provenance: PLAT-1025. The size and mix the PR reports: at least 300
/// rows, at least 40% of them mutation rows, all four modes present, and a
/// patch and a source on every mutation row.
#[test]
fn tc_1025_the_corpus_has_the_reported_shape() {
    let rows = corpus().file.rows;
    assert!(rows.len() >= 300, "{} rows", rows.len());
    let mutated = rows.iter().filter(|r| r.mutation.is_some()).count();
    assert!(
        mutated * 10 >= rows.len() * 4,
        "{mutated} of {} are mutation rows",
        rows.len()
    );
    let modes: BTreeSet<&str> = rows.iter().map(|r| r.mode.as_str()).collect();
    assert_eq!(modes, BTreeSet::from(["R", "RC", "RT", "RTC"]));
    for row in &rows {
        if let Some(m) = &row.mutation {
            assert!(!m.patch.trim().is_empty(), "{}: empty patch", row.id);
            assert!(
                m.source_id.is_some(),
                "{}: every mutant's source is a row",
                row.id
            );
        }
    }
}
