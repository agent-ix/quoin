// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Experiment 2 diagnostics (PLAT-1024): why K1 and E5 flag sound rows.
//! Dev split only; informational, gates nothing.
//!
//! When [`OUT_ENV`] names a directory, the live runner writes there, from
//! answers already in the run (a replayed cassette costs no call):
//!
//! - `k-<label>.jsonl`: one line per K row, every check's `P(defect)`, the
//!   row's truth and its criterion text;
//! - `e5-<label>.jsonl`: one line per E5 row, every asked unit's
//!   `P(necessary)` and text;
//! - `report.md`: per-check fire rates, E5's unnecessary-unit counts, and
//!   the combiner alternatives recomputed from those answers. Every
//!   combiner is post hoc: chosen after seeing dev.
//!
//! Bar D reuses the harness's own pairing and sign test: K through
//! [`soundness::pairs`], [`soundness::pair_outcome`] and [`BarD`], E5
//! through [`metrics::paired_contrast`]. A stricter threshold `t` is read by
//! shifting each ordinal by `0.5 - t`, so the shared `tau = 0.5` crossing is
//! the `t` crossing and every difference (the delta test) is unchanged.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use serde_json::json;

use super::corpus::Row;
use super::keys::{NO, YES};
use super::metrics::{self, ContrastSpec, Direction};
use super::variant::{Prediction, Predictions, RowResult, RunOutput};
use super::variants::exceeds::{self, E5};
use super::variants::soundness::{
    self, BarD, CHECKS, PairOutcome, SOUND, injected_check, is_natural, pair_outcome, pairs,
    required_noul,
};

/// The directory the diagnostics are written to; unset writes nothing.
pub(crate) const OUT_ENV: &str = "QUOIN_JEV_EXP2_OUT";

/// The primary truth label for `key`, if the row carries one.
fn primary(row: &Row, key: &str) -> Option<String> {
    row.truth.get(key).map(|truth| truth.answer.label())
}

fn pct(n: usize, d: usize) -> String {
    if d == 0 {
        "n/a".to_owned()
    } else {
        let (n, d) = (
            f64::from(u32::try_from(n).unwrap_or(u32::MAX)),
            f64::from(u32::try_from(d).unwrap_or(u32::MAX)),
        );
        format!("{:.1}%", 100.0 * n / d)
    }
}

/// The `k`-th highest value (1-based), or 0 when there are fewer.
fn kth_highest(values: &[f64], k: usize) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| b.total_cmp(a));
    k.checked_sub(1)
        .and_then(|index| sorted.get(index))
        .copied()
        .unwrap_or(0.0)
}

// ---------------------------------------------------------------------------
// K: one row's check probabilities
// ---------------------------------------------------------------------------

struct KRow<'a> {
    row: &'a Row,
    /// `P(defect)` per check, in [`CHECKS`] order.
    p: Vec<f64>,
}

fn k_rows<'a>(rows: &'a [Row], output: &RunOutput, label: &str) -> Vec<KRow<'a>> {
    let by_id: BTreeMap<&str, &Row> = rows.iter().map(|row| (row.id.as_str(), row)).collect();
    output
        .results
        .iter()
        .filter(|result| result.variant == label)
        .filter_map(|result| {
            let row = by_id.get(result.row_id.as_str())?;
            let answers = super::variant::whole_row(&result.answered);
            let p = CHECKS
                .iter()
                .map(|check| required_noul(&row.id, &answers, check.key))
                .collect();
            Some(KRow { row, p })
        })
        .collect()
}

/// A K combiner: which checks count, how many must fire, at what threshold.
struct KRule {
    name: String,
    dropped: Option<usize>,
    at_least: usize,
    tau: f64,
}

impl KRule {
    /// The row's unsoundness score: the `at_least`-th highest kept check.
    fn score(&self, p: &[f64]) -> f64 {
        let kept: Vec<f64> = p
            .iter()
            .enumerate()
            .filter(|(index, _)| Some(*index) != self.dropped)
            .map(|(_, p)| *p)
            .collect();
        kth_highest(&kept, self.at_least)
    }

    fn flags(&self, p: &[f64]) -> bool {
        self.score(p) >= self.tau
    }
}

fn check_index(key: &str) -> Option<usize> {
    CHECKS.iter().position(|check| check.key == key)
}

/// Bar D on the aggregate verdict: over MP-243's pairs, the rule's score on
/// the source against the mutant, shifted so the rule's threshold is the
/// harness's `TAU`.
fn k_aggregate_d(rows: &[Row], by_id: &BTreeMap<&str, &KRow<'_>>, rule: &KRule) -> BarD {
    let shift = soundness::TAU - rule.tau;
    let all = pairs(rows);
    let mut tally = BarD {
        pairs: all.len(),
        ..BarD::default()
    };
    for pair in &all {
        if primary(pair.source, pair.key).as_deref() == Some(YES) {
            tally.source_already_yes += 1;
            continue;
        }
        let score = |row: &Row| by_id.get(row.id.as_str()).map(|k| rule.score(&k.p) + shift);
        match score(pair.source).zip(score(pair.mutant)) {
            None => {
                tally.abstained += 1;
                tally.failures += 1;
            }
            Some((source, mutant)) => match pair_outcome(source, mutant) {
                PairOutcome::Success => tally.successes += 1,
                PairOutcome::Failure => tally.failures += 1,
                PairOutcome::Tie => tally.ties += 1,
            },
        }
    }
    tally
}

/// MP-243's own bar D (the injected check's probability), under a rule's
/// threshold and dropped check. A pair on a dropped check has no answer and
/// is a failure, as the harness counts an abstention.
fn k_check_d(rows: &[Row], by_id: &BTreeMap<&str, &KRow<'_>>, rule: &KRule) -> BarD {
    let shift = soundness::TAU - rule.tau;
    let predictions: Vec<(String, Predictions)> = by_id
        .iter()
        .map(|(id, k)| {
            let mut out = Predictions::new();
            for (index, check) in CHECKS.iter().enumerate() {
                if Some(index) == rule.dropped {
                    continue;
                }
                let p = k.p.get(index).copied().unwrap_or(0.0) + shift;
                out.insert(
                    check.key,
                    Prediction {
                        answer: if p >= soundness::TAU { YES } else { NO }.to_owned(),
                        confidence: None,
                        ordinal: Some(p),
                    },
                );
            }
            ((*id).to_owned(), out)
        })
        .collect();
    let map: BTreeMap<&str, &Predictions> =
        predictions.iter().map(|(id, p)| (id.as_str(), p)).collect();
    soundness::bar_d(&pairs(rows), &map)
}

fn show_d(d: &BarD) -> String {
    format!(
        "{}-{}-{} (p = {:.4}{})",
        d.successes,
        d.failures,
        d.ties,
        d.p_value(),
        if d.abstained > 0 {
            format!(", {} abstained", d.abstained)
        } else {
            String::new()
        }
    )
}

#[allow(
    clippy::too_many_lines,
    reason = "a diagnostic report, read top to bottom; splitting it scatters one table across helpers"
)]
fn k_report(out: &mut String, rows: &[Row], output: &RunOutput, label: &str) -> Vec<String> {
    let krows = k_rows(rows, output, label);
    let sound: Vec<&KRow<'_>> = krows
        .iter()
        .filter(|k| primary(k.row, SOUND).as_deref() == Some(YES))
        .collect();
    let defective: Vec<&KRow<'_>> = krows
        .iter()
        .filter(|k| primary(k.row, SOUND).as_deref() == Some(NO))
        .collect();
    let mutants = defective
        .iter()
        .filter(|k| k.row.mutation.is_some())
        .count();
    let _ = writeln!(
        out,
        "\n## {label}: per-check fire rates (P >= 0.5)\n\n{} rows; `{SOUND}` = yes on {} \
         (all natural), = no on {} ({} mutants, {} natural).\n\n\
         | Check | Fires on sound | Fires on defective | Only check firing: sound | Only check \
         firing: defective | Injected in mutants | Fires on its own mutants |\n\
         | --- | --- | --- | --- | --- | --- | --- |",
        krows.len(),
        sound.len(),
        defective.len(),
        mutants,
        defective.len() - mutants,
    );
    let fires = |k: &KRow<'_>, index: usize| k.p.get(index).is_some_and(|p| *p >= 0.5);
    let only = |k: &KRow<'_>, index: usize| {
        fires(k, index) && (0..CHECKS.len()).filter(|i| fires(k, *i)).count() == 1
    };
    for (index, check) in CHECKS.iter().enumerate() {
        let own: Vec<&&KRow<'_>> = defective
            .iter()
            .filter(|k| injected_check(k.row) == Some(check.key))
            .collect();
        let _ = writeln!(
            out,
            "| {} | {}/{} | {}/{} | {} | {} | {} | {}/{} |",
            check.key,
            sound.iter().filter(|k| fires(k, index)).count(),
            sound.len(),
            defective.iter().filter(|k| fires(k, index)).count(),
            defective.len(),
            sound.iter().filter(|k| only(k, index)).count(),
            defective.iter().filter(|k| only(k, index)).count(),
            own.len(),
            own.iter().filter(|k| fires(k, index)).count(),
            own.len(),
        );
    }
    let fired_count = |k: &KRow<'_>| (0..CHECKS.len()).filter(|i| fires(k, *i)).count();
    let _ = writeln!(
        out,
        "\nChecks firing per row:\n\n| Checks firing | Sound rows | Defective rows |\n| --- | --- | --- |"
    );
    for n in 0..=CHECKS.len() {
        let _ = writeln!(
            out,
            "| {n} | {} | {} |",
            sound.iter().filter(|k| fired_count(k) == n).count(),
            defective.iter().filter(|k| fired_count(k) == n).count()
        );
    }

    // Combiners.
    let by_id: BTreeMap<&str, &KRow<'_>> = krows.iter().map(|k| (k.row.id.as_str(), k)).collect();
    let mut rules = vec![KRule {
        name: "(a) any >= 0.5 (as shipped)".to_owned(),
        dropped: None,
        at_least: 1,
        tau: 0.5,
    }];
    for check in CHECKS {
        rules.push(KRule {
            name: format!("(b) any >= 0.5, `{}` dropped", check.key),
            dropped: check_index(check.key),
            at_least: 1,
            tau: 0.5,
        });
    }
    rules.push(KRule {
        name: "(c) >= 2 checks >= 0.5".to_owned(),
        dropped: None,
        at_least: 2,
        tau: 0.5,
    });
    for tau in [0.6, 0.7] {
        rules.push(KRule {
            name: format!("(d) any >= {tau}"),
            dropped: None,
            at_least: 1,
            tau,
        });
    }
    let _ = writeln!(
        out,
        "\n### {label}: combiners (POST HOC on dev, chosen after seeing it)\n\n\
         Sound recall = sound rows cleared; defect recall = `{SOUND}` = no rows flagged. \
         Bar D as success-failure-tie. Aggregate D: the rule's own score (k-th highest kept \
         check) must cross its threshold from source to mutant. Check D: MP-243's definition, \
         the injected check's probability at the rule's threshold.\n\n\
         | Rule | Sound recall | Defect recall | Mutant recall | Aggregate D | Check D |\n\
         | --- | --- | --- | --- | --- | --- |"
    );
    for rule in &rules {
        let cleared = sound.iter().filter(|k| !rule.flags(&k.p)).count();
        let caught = defective.iter().filter(|k| rule.flags(&k.p)).count();
        let caught_mutants = defective
            .iter()
            .filter(|k| k.row.mutation.is_some() && rule.flags(&k.p))
            .count();
        let _ = writeln!(
            out,
            "| {} | {}/{} {} | {}/{} {} | {}/{} | {} | {} |",
            rule.name,
            cleared,
            sound.len(),
            pct(cleared, sound.len()),
            caught,
            defective.len(),
            pct(caught, defective.len()),
            caught_mutants,
            mutants,
            show_d(&k_aggregate_d(rows, &by_id, rule)),
            show_d(&k_check_d(rows, &by_id, rule)),
        );
    }

    krows
        .iter()
        .map(|k| {
            let truth: BTreeMap<&str, serde_json::Value> = k
                .row
                .truth
                .iter()
                .map(|(key, truth)| {
                    (
                        key.as_str(),
                        json!({
                            "answer": truth.answer.label(),
                            "kind": format!("{:?}", truth.kind),
                            "alternatives": truth.alternatives.iter().map(super::corpus::TruthAnswer::label).collect::<Vec<_>>(),
                            "rationale": truth.rationale,
                        }),
                    )
                })
                .collect();
            let p: BTreeMap<&str, f64> = CHECKS
                .iter()
                .zip(&k.p)
                .map(|(check, p)| (check.key, *p))
                .collect();
            json!({
                "id": k.row.id,
                "natural": is_natural(k.row),
                "mutation": k.row.mutation.as_ref().map(|m| json!({"kind": m.kind, "source_id": m.source_id, "description": m.description})),
                "injected": injected_check(k.row),
                "sound": primary(k.row, SOUND),
                "p": p,
                "criterion": super::variant::criterion(k.row).text,
                "requirement_statement": k.row.requirement.statement,
                "truth": truth,
            })
            .to_string()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// E5
// ---------------------------------------------------------------------------

struct ERow<'a> {
    row: &'a Row,
    /// `(unit index, P(necessary))` per asked unit.
    units: Vec<(usize, f64)>,
}

impl ERow<'_> {
    /// Per-unit `1 - P(necessary)`.
    fn u(&self) -> Vec<f64> {
        self.units.iter().map(|(_, p)| 1.0 - p).collect()
    }
}

fn e_rows<'a>(rows: &'a [Row], output: &RunOutput, label: &str) -> Result<Vec<ERow<'a>>, String> {
    let by_id: BTreeMap<&str, &Row> = rows.iter().map(|row| (row.id.as_str(), row)).collect();
    output
        .results
        .iter()
        .filter(|result| result.variant == label)
        .filter_map(|result| by_id.get(result.row_id.as_str()).map(|row| (result, *row)))
        .map(|(result, row)| {
            let assessment = exceeds::assess_necessity(row, &result.answered)?;
            Ok(ERow {
                row,
                units: assessment
                    .readings
                    .iter()
                    .map(|r| (r.unit, r.necessary))
                    .collect(),
            })
        })
        .collect()
}

struct ERule {
    name: String,
    at_least: usize,
    /// A unit is unnecessary when `P(necessary) < 1 - tau`, i.e. `u > tau`.
    tau: f64,
}

impl ERule {
    fn score(&self, u: &[f64]) -> f64 {
        kth_highest(u, self.at_least)
    }

    fn flags(&self, u: &[f64]) -> bool {
        // As shipped: unnecessary iff P(necessary) < 0.5, i.e. u > 0.5.
        self.score(u) > self.tau
    }
}

fn e_bar_d(
    rows: &[Row],
    erows: &[ERow<'_>],
    rule: &ERule,
) -> Result<metrics::PairedContrast, String> {
    let label = format!("exp2 {}", rule.name);
    let shift = exceeds::PAIRED_TAU - rule.tau;
    let output = RunOutput {
        results: erows
            .iter()
            .map(|e| {
                let ordinal = rule.score(&e.u()) + shift;
                RowResult {
                    row_id: e.row.id.clone(),
                    variant: label.clone(),
                    predictions: Predictions::from([(
                        exceeds::KEY,
                        Prediction {
                            answer: if rule.flags(&e.u()) { YES } else { NO }.to_owned(),
                            confidence: None,
                            ordinal: Some(ordinal),
                        },
                    )]),
                    answered: Vec::new(),
                }
            })
            .collect(),
        ..RunOutput::default()
    };
    metrics::paired_contrast(
        rows,
        &output,
        &ContrastSpec {
            variant: &label,
            modes: E5.modes,
            key: exceeds::KEY,
            kinds: &exceeds::PAIRED_KINDS,
            direction: Direction::Up,
            tau: exceeds::PAIRED_TAU,
            delta: exceeds::PAIRED_DELTA,
            defect_side: &[YES],
        },
        &exceeds::source_id,
    )
}

#[allow(
    clippy::too_many_lines,
    reason = "a diagnostic report, read top to bottom; splitting it scatters one table across helpers"
)]
fn e_report(
    out: &mut String,
    rows: &[Row],
    output: &RunOutput,
    label: &str,
) -> Result<Vec<String>, String> {
    let erows = e_rows(rows, output, label)?;
    let key = exceeds::KEY;
    let sound: Vec<&ERow<'_>> = erows
        .iter()
        .filter(|e| primary(e.row, key).as_deref() == Some(NO))
        .collect();
    let defective: Vec<&ERow<'_>> = erows
        .iter()
        .filter(|e| primary(e.row, key).as_deref() == Some(YES))
        .collect();
    let unnecessary = |e: &ERow<'_>| e.units.iter().filter(|(_, p)| *p < 0.5).count();
    let _ = writeln!(
        out,
        "\n## {label}: unnecessary units (P(necessary) < 0.5) per row\n\n{} rows ran; `{key}` = no \
         (sound) on {}, = yes on {} ({} mutants); {} unlabelled.\n\n\
         | Unnecessary units | Sound rows | Defective rows |\n| --- | --- | --- |",
        erows.len(),
        sound.len(),
        defective.len(),
        defective
            .iter()
            .filter(|e| e.row.mutation.is_some())
            .count(),
        erows.len() - sound.len() - defective.len(),
    );
    for n in 0..=5 {
        let bucket = |e: &&&ERow<'_>| {
            let count = unnecessary(e);
            if n == 5 { count >= 5 } else { count == n }
        };
        let _ = writeln!(
            out,
            "| {}{} | {} | {} |",
            n,
            if n == 5 { "+" } else { "" },
            sound.iter().filter(bucket).count(),
            defective.iter().filter(bucket).count()
        );
    }
    let units = |group: &[&ERow<'_>]| -> (usize, usize) {
        (
            group.iter().map(|e| unnecessary(e)).sum(),
            group.iter().map(|e| e.units.len()).sum(),
        )
    };
    let ((su, sa), (du, da)) = (units(&sound), units(&defective));
    let _ = writeln!(
        out,
        "\nUnits unnecessary / asked: sound rows {su}/{sa} ({}), defective rows {du}/{da} ({}).",
        pct(su, sa),
        pct(du, da)
    );

    let mut rules = vec![ERule {
        name: "(a) any unit P<0.5 (as shipped)".to_owned(),
        at_least: 1,
        tau: 0.5,
    }];
    for k in [2, 3] {
        rules.push(ERule {
            name: format!("(c) >= {k} units P<0.5"),
            at_least: k,
            tau: 0.5,
        });
    }
    for p in [0.3, 0.2] {
        rules.push(ERule {
            name: format!("(d) any unit P<{p}"),
            at_least: 1,
            tau: 1.0 - p,
        });
    }
    rules.push(ERule {
        name: "(c+d) >= 2 units P<0.3".to_owned(),
        at_least: 2,
        tau: 0.7,
    });
    let _ = writeln!(
        out,
        "\n### {label}: combiners (POST HOC on dev, chosen after seeing it)\n\n\
         Sound recall = `{key}` = no rows answered no; defect recall = `{key}` = yes rows answered \
         yes. Bar D: the shared paired contrast (additive_code pairs) on the rule's own score \
         (k-th highest `1 - P(necessary)`), shifted so its threshold is the shared tau.\n\n\
         | Rule | Sound recall | Defect recall | Bar D (s-f-t) | p |\n| --- | --- | --- | --- | --- |"
    );
    for rule in &rules {
        let cleared = sound.iter().filter(|e| !rule.flags(&e.u())).count();
        let caught = defective.iter().filter(|e| rule.flags(&e.u())).count();
        let d = e_bar_d(rows, &erows, rule)?;
        let _ = writeln!(
            out,
            "| {} | {}/{} {} | {}/{} {} | {}-{}-{} | {} |",
            rule.name,
            cleared,
            sound.len(),
            pct(cleared, sound.len()),
            caught,
            defective.len(),
            pct(caught, defective.len()),
            d.successes(),
            d.failures(),
            d.ties(),
            d.p_value()
                .map_or_else(|| "n/a".to_owned(), |p| format!("{p:.4}")),
        );
    }

    Ok(erows
        .iter()
        .map(|e| {
            let units = exceeds::row_e5_units(e.row);
            json!({
                "id": e.row.id,
                "mode": e.row.mode.as_str(),
                "truth": primary(e.row, key),
                "truth_kind": e.row.truth.get(key).map(|t| format!("{:?}", t.kind)),
                "rationale": e.row.truth.get(key).map(|t| t.rationale.clone()),
                "mutation": e.row.mutation.as_ref().map(|m| json!({"kind": m.kind, "source_id": m.source_id, "description": m.description})),
                "fr_statement": e.row.requirement.statement,
                "ac_text": e.row.requirement.ac_text,
                "code_path": e.row.code.as_ref().map(|c| c.path.clone()),
                "code_body": e.row.code.as_ref().map(|c| c.body.clone()),
                "units": e.units.iter().map(|(index, p)| json!({
                    "index": index,
                    "p_necessary": p,
                    "label": units.get(*index).map(|u| u.label.clone()),
                    "text": units.get(*index).map(|u| u.text.clone()),
                })).collect::<Vec<_>>(),
            })
            .to_string()
        })
        .collect())
}

fn file_label(label: &str) -> String {
    label.replace('@', "-")
}

/// Writes the diagnostics for every K and E5 variant in `output` to `dir`.
///
/// # Errors
/// When a file cannot be written or an E5 answer is malformed.
pub(crate) fn write(dir: &Path, rows: &[Row], output: &RunOutput) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|error| format!("{}: {error}", dir.display()))?;
    let labels: std::collections::BTreeSet<&str> =
        output.results.iter().map(|r| r.variant.as_str()).collect();
    let mut report =
        String::from("# Experiment 2: K1 / E5 false alarms (dev only; informational)\n");
    let put = |name: String, lines: &[String]| {
        let path = dir.join(name);
        std::fs::write(&path, lines.join("\n") + "\n")
            .map_err(|error| format!("{}: {error}", path.display()))
    };
    for label in &labels {
        if label.starts_with("K1@") || label.starts_with("K2@") {
            let lines = k_report(&mut report, rows, output, label);
            put(format!("k-{}.jsonl", file_label(label)), &lines)?;
        } else if *label == E5.label() {
            let lines = e_report(&mut report, rows, output, label)?;
            put(format!("e5-{}.jsonl", file_label(label)), &lines)?;
        }
    }
    let path = dir.join("report.md");
    std::fs::write(&path, report).map_err(|error| format!("{}: {error}", path.display()))
}
