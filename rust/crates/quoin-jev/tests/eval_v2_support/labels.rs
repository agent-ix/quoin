// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The agent-label readers for the committed corpus fixtures: how a
//! `labels-*.json` entry becomes per-key truth, and where the fixtures live.
//! The corpus itself is a plain committed fixture; nothing here rebuilds it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a fixture reader over committed inputs: a malformed input IS the failure report"
)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_json::{Value, json};

use super::variants::battery::Battery;
use super::variants::refusal;
use super::variants::statement;

/// `tests/fixtures/eval-v2/`.
pub(crate) fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/eval-v2")
}

fn s(value: &Value, key: &str) -> String {
    value[key]
        .as_str()
        .unwrap_or_else(|| panic!("{key} missing in {value}"))
        .to_owned()
}

fn u(value: &Value, key: &str) -> usize {
    usize::try_from(
        value[key]
            .as_u64()
            .unwrap_or_else(|| panic!("{key} in {value}")),
    )
    .unwrap()
}

// ---------------------------------------------------------------------------
// Truth
// ---------------------------------------------------------------------------

fn truth(answer: &str, kind: &str, alternatives: &[String], rationale: &str) -> Value {
    json!({"answer": answer, "kind": kind, "alternatives": alternatives, "rationale": rationale})
}

/// The label for `key` on natural row `id`, merged from the two passes.
fn natural_truth(a: &Value, b: &Value, id: &str, key: &str) -> Option<Value> {
    let la = a[id][key]["answer"].as_str()?;
    let lb = b[id][key]["answer"].as_str()?;
    let ra = a[id][key]["why"].as_str().unwrap_or("");
    let rb = b[id][key]["why"].as_str().unwrap_or("");
    Some(if la == lb {
        truth(
            la,
            "agent_dual",
            &[],
            &format!("AGENT-LABELLED, two independent passes agree. A: {ra} | B: {rb}"),
        )
    } else {
        truth(
            la,
            "agent_contested",
            &[lb.to_owned()],
            &format!(
                "AGENT-LABELLED, passes disagree (A={la}, B={lb}); not resolved. A: {ra} | B: {rb}"
            ),
        )
    })
}

/// One entry of a single-pass label file for `battery`: its aggregate and
/// every check, kind `agent_single`, with the labeller's other defensible
/// reading, if any, as the alternative.
///
/// # Errors
/// When any of the battery's keys is missing or not `yes`/`no`, or the
/// aggregate is not `no` exactly when some check is `yes`: a malformed entry
/// is refused, never silently dropped.
fn single_pass_truth(
    battery: &Battery,
    id: &str,
    entry: &Value,
) -> Result<BTreeMap<String, Value>, String> {
    let mut out = BTreeMap::new();
    let keys = std::iter::once(battery.sound).chain(battery.checks.iter().map(|check| check.key));
    for key in keys {
        let label = &entry[key];
        let answer = label["answer"]
            .as_str()
            .filter(|answer| ["yes", "no"].contains(answer))
            .ok_or_else(|| format!("{id}: {key} has no yes/no answer"))?;
        let alternatives: Vec<String> = label["alternatives"]
            .as_array()
            .map(|alts| {
                alts.iter()
                    .filter_map(|a| a.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        let why = label["why"].as_str().unwrap_or("");
        out.insert(
            key.to_owned(),
            truth(
                answer,
                "agent_single",
                &alternatives,
                &format!("AGENT-LABELLED, single pass, not audited. {why}"),
            ),
        );
    }
    let fires = battery
        .checks
        .iter()
        .any(|check| out[check.key]["answer"] == "yes");
    let sound = out[battery.sound]["answer"] == "yes";
    if sound == fires {
        return Err(format!(
            "{id}: {} is {} but {} check says yes",
            battery.sound,
            if sound { "yes" } else { "no" },
            if fires { "a" } else { "no" }
        ));
    }
    Ok(out)
}

/// The requirement-statement truths for one entry of
/// `labels-fr-statement.json` ([`single_pass_truth`] on the statement
/// battery).
///
/// # Errors
/// As [`single_pass_truth`].
pub(crate) fn statement_truth(id: &str, entry: &Value) -> Result<BTreeMap<String, Value>, String> {
    single_pass_truth(&statement::STATEMENT, id, entry)
}

/// The refusal truths for one entry of `labels-ac-refusal.json`
/// ([`single_pass_truth`] on the refusal battery).
///
/// # Errors
/// As [`single_pass_truth`].
pub(crate) fn refusal_truth(id: &str, entry: &Value) -> Result<BTreeMap<String, Value>, String> {
    single_pass_truth(&refusal::REFUSAL, id, entry)
}

/// One single-pass label entry's truths by key, or why it is refused.
type TruthFn = fn(&str, &Value) -> Result<BTreeMap<String, Value>, String>;

/// Every entry of a single-pass label file's `labels`, checked against
/// `sample.json`: each id must be a dev natural row, and each entry must
/// pass `truth`.
///
/// # Errors
/// Naming the first unknown or held-out id, or malformed entry.
fn single_pass_labels(
    file: &str,
    labels: &Value,
    sample: &Value,
    truth: TruthFn,
) -> Result<BTreeMap<String, BTreeMap<String, Value>>, String> {
    let labels = labels
        .as_object()
        .ok_or_else(|| format!("{file}: `labels` is not an object"))?;
    let natural: Vec<&str> = sample["natural"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|n| n["id"].as_str())
        .collect();
    labels
        .iter()
        .map(|(id, entry)| {
            if !natural.contains(&id.as_str()) {
                return Err(format!("{id}: not a natural row of sample.json"));
            }
            if sample["split"][id] != "dev" {
                return Err(format!("{id}: not a dev row; {file} labels are dev only"));
            }
            Ok((id.clone(), truth(id, entry)?))
        })
        .collect()
}

/// Every entry of `labels-fr-statement.json`'s `labels`, checked by
/// [`single_pass_labels`] and [`statement_truth`].
///
/// # Errors
/// As [`single_pass_labels`].
pub(crate) fn statement_labels(
    labels: &Value,
    sample: &Value,
) -> Result<BTreeMap<String, BTreeMap<String, Value>>, String> {
    single_pass_labels("labels-fr-statement.json", labels, sample, statement_truth)
}

/// Every entry of `labels-ac-refusal.json`'s `labels`, checked by
/// [`single_pass_labels`] and [`refusal_truth`].
///
/// # Errors
/// As [`single_pass_labels`].
pub(crate) fn refusal_labels(
    labels: &Value,
    sample: &Value,
) -> Result<BTreeMap<String, BTreeMap<String, Value>>, String> {
    single_pass_labels("labels-ac-refusal.json", labels, sample, refusal_truth)
}
