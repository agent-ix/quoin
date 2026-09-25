// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! A battery of named defect checks over one unit, combined in code
//! (PLAT-1024 experiments 3 and 4). Dev only; informational, gates nothing.
//!
//! The requirement statement (`statement::STATEMENT`) and the acceptance
//! criterion's refusal reasons (`refusal::REFUSAL`) are both batteries: one
//! `noul` per [`Check`], an aggregate key derived by a [`Combiner`], and
//! dev-only mutants that inject one check's defect. Bar D, the report tables
//! and the holistic baseline's derivation live here so both read the same
//! code.
//!
//! # One comparison
//!
//! Every rule here scores a row by a defect score (a check's probability, a
//! combiner's score, or `1 - P(sound)` for a holistic baseline) and flags it
//! by [`flags`]. The graded answers ([`Combiner::sound`],
//! [`holistic_predictions`]) and the report's flags use that one function,
//! so they agree at every score, `0.5` included.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde_json::{Value, json};

use crate::eval_v2_support::corpus::{Row, Truth, TruthKind};
use crate::eval_v2_support::keys::{NO, YES};
use crate::eval_v2_support::variant::{Answered, Prediction, Predictions, RunOutput};
use crate::eval_v2_support::variants::soundness::{BarD, PairOutcome, TAU, credit, pair_outcome};

/// Whether a defect score flags its row at `tau`: at or above it.
pub(crate) fn flags(score: f64, tau: f64) -> bool {
    score >= tau
}

/// A holistic baseline's graded aggregate from its `P(sound)`: the defect
/// score is `1 - p_sound`, flagged by [`flags`] at [`TAU`] exactly as the
/// report flags it. The ordinal is `p_sound`, higher meaning more likely
/// sound.
pub(crate) fn holistic_predictions(sound_key: &'static str, p_sound: f64) -> Predictions {
    let defect = 1.0 - p_sound;
    let flagged = flags(defect, TAU);
    Predictions::from([(
        sound_key,
        Prediction {
            answer: if flagged { NO } else { YES }.to_owned(),
            confidence: Some(if flagged { defect } else { p_sound }),
            ordinal: Some(p_sound),
        },
    )])
}

/// One named defect check.
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

/// How a battery's checks combine into its aggregate: the unit is flagged
/// when at least `at_least` checks are at or above `tau`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Combiner {
    /// How many checks must fire.
    pub(crate) at_least: usize,
    /// The threshold a check fires at.
    pub(crate) tau: f64,
}

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

    /// Whether the unit is flagged.
    pub(crate) fn flags(&self, probabilities: &[f64]) -> bool {
        flags(self.score(probabilities), self.tau)
    }

    /// The aggregate as a graded answer: `no` when flagged. The ordinal is
    /// `1 - score`, higher meaning more likely sound; the confidence is the
    /// score for `no` and `1 - score` for `yes`.
    pub(crate) fn sound(&self, probabilities: &[f64]) -> Prediction {
        let score = self.score(probabilities);
        let flagged = flags(score, self.tau);
        Prediction {
            answer: if flagged { NO } else { YES }.to_owned(),
            confidence: Some(if flagged { score } else { 1.0 - score }),
            ordinal: Some(1.0 - score),
        }
    }
}

/// Each answered row's defect score under one rule; an unanswered row is absent.
pub(crate) type Scores<'a> = BTreeMap<&'a str, f64>;

/// One battery of named defect checks: its aggregate key (`yes` when no
/// check fires), its checks, and the mutation kind whose dev-only rows each
/// inject one check's defect. `statement::STATEMENT` and `refusal::REFUSAL`
/// are the two. Bar D and the report tables read a
/// battery, so a second battery reuses them rather than copying them.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Battery {
    /// The aggregate key.
    pub(crate) sound: &'static str,
    /// The named checks, in report order.
    pub(crate) checks: &'static [Check],
    /// The mutation kind whose rows carry truth for this battery only.
    pub(crate) mutation_kind: &'static str,
    /// Whether a variant graded on this battery asks only rows labelled with
    /// [`Battery::sound`], so no request is spent on a row it cannot score.
    pub(crate) labelled_only: bool,
}

fn primary(row: &Row, key: &str) -> Option<String> {
    row.truth.get(key).map(|truth| truth.answer.label())
}

/// One mutant paired with its source: the source id, and the pair's outcome
/// with whether it was an abstention (a failure because a side went
/// unanswered), or `None` when the source is already on the defect side
/// (dropped).
type Paired<'a> = (&'a str, Option<(PairOutcome, bool)>);

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

/// `n/d` with its percentage, or `n/a` for an empty denominator.
pub(crate) fn pct(n: usize, d: usize) -> String {
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

/// A bar D tally as one report cell.
pub(crate) fn show_d(d: &BarD) -> String {
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
pub(crate) struct RuleLine<'a> {
    /// The rule's name in the report.
    pub(crate) name: String,
    /// Each answered row's defect score.
    pub(crate) scores: Scores<'a>,
    /// The score at which the row is flagged.
    pub(crate) tau: f64,
}

fn flagged(line: &RuleLine<'_>, row: &Row) -> Option<bool> {
    line.scores
        .get(row.id.as_str())
        .map(|s| flags(*s, line.tau))
}

/// Each row's check probabilities for the variant labelled `label`, read by
/// `probabilities`.
pub(crate) fn probability_table<'a>(
    rows: &'a [Row],
    output: &RunOutput,
    label: &str,
    probabilities: impl Fn(&Row, &[Answered]) -> Option<Vec<f64>>,
) -> BTreeMap<&'a str, Vec<f64>> {
    let by_id: BTreeMap<&str, &Row> = rows.iter().map(|row| (row.id.as_str(), row)).collect();
    output
        .results
        .iter()
        .filter(|result| result.variant == label)
        .filter_map(|result| {
            let row = by_id.get(result.row_id.as_str())?;
            probabilities(row, &result.answered).map(|p| (row.id.as_str(), p))
        })
        .collect()
}

/// Each row's defect score `1 - P(sound)` for the holistic variant labelled
/// `label`, read by `p_sound`.
pub(crate) fn holistic_scores<'a>(
    rows: &'a [Row],
    output: &RunOutput,
    label: &str,
    p_sound: fn(&Row, &[Answered]) -> Option<f64>,
) -> Scores<'a> {
    let by_id: BTreeMap<&str, &Row> = rows.iter().map(|row| (row.id.as_str(), row)).collect();
    output
        .results
        .iter()
        .filter(|result| result.variant == label)
        .filter_map(|result| {
            let row = by_id.get(result.row_id.as_str())?;
            p_sound(row, &result.answered).map(|p| (row.id.as_str(), 1.0 - p))
        })
        .collect()
}

impl Battery {
    /// Whether `row` is one of this battery's mutants.
    pub(crate) fn is_mutant(&self, row: &Row) -> bool {
        row.mutation
            .as_ref()
            .is_some_and(|mutation| mutation.kind == self.mutation_kind)
    }

    /// Whether `key` is this battery's aggregate or one of its checks.
    pub(crate) fn grades(&self, key: &str) -> bool {
        key == self.sound || self.checks.iter().any(|check| check.key == key)
    }

    /// The check one of this battery's mutants injected: its one
    /// by-construction `yes`.
    pub(crate) fn injected(&self, row: &Row) -> Option<&'static str> {
        if !self.is_mutant(row) {
            return None;
        }
        self.checks.iter().map(|check| check.key).find(|key| {
            row.truth.get(*key).is_some_and(|truth| {
                truth.kind == TruthKind::ByConstruction && truth.answer.label() == YES
            })
        })
    }

    /// Every mutant whose injected check is in `checks`, paired with its
    /// source, in row order: the pair's scores, shifted so `tau` is the
    /// shared 0.5 crossing, through the shared pair rule. `excluded_key`
    /// names the truth whose `yes` (for a check) or `no` (for the aggregate)
    /// on the source already puts it on the defect side, so the pair is
    /// dropped. An unanswered side is a failure.
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
        &self,
        rows: &'a [Row],
        scores: &Scores<'_>,
        tau: f64,
        checks: &[&str],
        excluded_key: &str,
    ) -> Vec<Paired<'a>> {
        let by_id: BTreeMap<&str, &Row> = rows.iter().map(|row| (row.id.as_str(), row)).collect();
        let defect_label = if excluded_key == self.sound { NO } else { YES };
        let shift = TAU - tau;
        let score = |row: &Row| scores.get(row.id.as_str()).map(|s| s + shift);
        let kind = self.mutation_kind;
        rows.iter()
            .filter(|mutant| {
                self.injected(mutant)
                    .is_some_and(|key| checks.contains(&key))
            })
            .map(|mutant| {
                let id = &mutant.id;
                let source_id = mutant
                    .mutation
                    .as_ref()
                    .and_then(|m| m.source_id.as_deref())
                    .unwrap_or_else(|| panic!("{id}: {kind} mutant has no mutation.source_id"));
                let source = by_id.get(source_id).copied().unwrap_or_else(|| {
                    panic!("{id}: source {source_id} is not among this run's rows")
                });
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

    /// Bar D over every pair. Pairs sharing a source are not independent,
    /// so this line is descriptive; [`Battery::bar_d_per_source`] gates.
    ///
    /// # Panics
    /// When a mutant's pair link is broken (see `paired`).
    pub(crate) fn bar_d(
        &self,
        rows: &[Row],
        scores: &Scores<'_>,
        tau: f64,
        checks: &[&str],
        excluded_key: &str,
    ) -> BarD {
        tally(
            self.paired(rows, scores, tau, checks, excluded_key)
                .into_iter()
                .map(|(_, outcome)| outcome),
        )
    }

    /// Bar D with one pair per source: each source's majority outcome over
    /// its kept pairs (success when successes outnumber failures, failure
    /// when failures outnumber successes, else a tie); a source whose every
    /// pair was dropped counts as dropped. Sources are independent, so this
    /// line gates.
    ///
    /// # Panics
    /// As [`Battery::bar_d`].
    pub(crate) fn bar_d_per_source(
        &self,
        rows: &[Row],
        scores: &Scores<'_>,
        tau: f64,
        checks: &[&str],
        excluded_key: &str,
    ) -> BarD {
        let mut by_source: BTreeMap<&str, Vec<Option<(PairOutcome, bool)>>> = BTreeMap::new();
        for (source, outcome) in self.paired(rows, scores, tau, checks, excluded_key) {
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

    /// The rows a report reads: labelled with the aggregate.
    pub(crate) fn labelled<'a>(&self, rows: &'a [Row]) -> Vec<&'a Row> {
        rows.iter()
            .filter(|row| row.truth.contains_key(self.sound))
            .collect()
    }

    /// `(credit, best constant credit, rows)` for the aggregate over `rows`.
    fn accuracy(&self, line: &RuleLine<'_>, rows: &[&Row]) -> (f64, f64, usize) {
        let answer = |row: &Row| flagged(line, row).map(|flag| if flag { NO } else { YES });
        let truths: Vec<(&Row, &Truth)> = rows
            .iter()
            .filter_map(|row| row.truth.get(self.sound).map(|truth| (*row, truth)))
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

    /// The rule table's header: one mutant column per check.
    pub(crate) fn rule_header(&self) -> String {
        let mut header =
            "| Rule | Sound-row recall (natural) | Defect recall (natural) |".to_owned();
        for check in self.checks {
            let _ = write!(header, " {} mutants |", check.key);
        }
        header.push_str(
            " Credit vs best constant (natural) | Credit vs best constant (natural + mutants) | \
             Bar D aggregate, one pair per source (W-L-T; GATES) | Bar D aggregate, every pair \
             (W-L-T; descriptive, pairs share sources) |\n|",
        );
        for _ in 0..self.checks.len() + 7 {
            header.push_str(" --- |");
        }
        header
    }

    /// One rule's line in the rule table.
    ///
    /// # Panics
    /// As [`Battery::bar_d`].
    pub(crate) fn render_rule(&self, out: &mut String, rows: &[Row], line: &RuleLine<'_>) {
        let all = self.labelled(rows);
        let natural: Vec<&Row> = all
            .iter()
            .copied()
            .filter(|row| row.mutation.is_none())
            .collect();
        let side = |label: &str| -> Vec<&Row> {
            natural
                .iter()
                .copied()
                .filter(|row| primary(row, self.sound).as_deref() == Some(label))
                .collect()
        };
        let (sound, defective) = (side(YES), side(NO));
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
        for check in self.checks {
            let mutants: Vec<&Row> = all
                .iter()
                .copied()
                .filter(|row| self.injected(row) == Some(check.key))
                .collect();
            let _ = write!(per_kind, " {} | ", pct(caught(&mutants), mutants.len()));
        }
        let (ours_n, constant_n, n_n) = self.accuracy(line, &natural);
        let (ours_a, constant_a, n_a) = self.accuracy(line, &all);
        let keys: Vec<&str> = self.checks.iter().map(|check| check.key).collect();
        let d = self.bar_d(rows, &line.scores, line.tau, &keys, self.sound);
        let per_source = self.bar_d_per_source(rows, &line.scores, line.tau, &keys, self.sound);
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

    /// The per-check table: each check's fire rate on sound and defective
    /// natural rows and on mutants, from one probability per check per row
    /// (in [`Battery::checks`] order). `variant` names the variant and its
    /// wording version (with whether that wording is post hoc) in the
    /// table's caption.
    ///
    /// # Panics
    /// As [`Battery::bar_d`].
    pub(crate) fn render_checks(
        &self,
        out: &mut String,
        rows: &[Row],
        table: &BTreeMap<&str, Vec<f64>>,
        tau: f64,
        variant: &str,
    ) {
        let all = self.labelled(rows);
        let natural = |label: &str| -> Vec<&Row> {
            all.iter()
                .copied()
                .filter(|row| {
                    row.mutation.is_none() && primary(row, self.sound).as_deref() == Some(label)
                })
                .collect()
        };
        let (sound, defective) = (natural(YES), natural(NO));
        let _ = writeln!(
            out,
            "\n{variant}, each check at {tau}:\n\n| Check (fires at {tau}) | Fires on sound (natural) | Fires on defective (natural) | \
             Natural labelled yes on it | Fires on its own mutants | Fires on other mutants | Bar \
             D (its own mutants, W-L-T) |\n| --- | --- | --- | --- | --- | --- | --- |"
        );
        for (index, check) in self.checks.iter().enumerate() {
            let fires = |row: &Row| {
                table
                    .get(row.id.as_str())
                    .and_then(|p| p.get(index))
                    .is_some_and(|p| flags(*p, tau))
            };
            let count = |rows: &[&Row]| rows.iter().filter(|row| fires(row)).count();
            let own: Vec<&Row> = all
                .iter()
                .copied()
                .filter(|row| self.injected(row) == Some(check.key))
                .collect();
            let other: Vec<&Row> = all
                .iter()
                .copied()
                .filter(|row| self.injected(row).is_some_and(|key| key != check.key))
                .collect();
            let labelled_yes = all
                .iter()
                .filter(|row| {
                    row.mutation.is_none() && primary(row, check.key).as_deref() == Some(YES)
                })
                .count();
            let scores: Scores<'_> = table
                .iter()
                .filter_map(|(id, p)| p.get(index).map(|p| (*id, *p)))
                .collect();
            let d = self.bar_d(rows, &scores, tau, &[check.key], check.key);
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

    /// One JSON object per labelled row: its truth, the battery's check
    /// probabilities from `table`, the holistic defect score from
    /// `holistic`, and `unit`, for reading flagged rows by hand.
    pub(crate) fn dump(
        &self,
        rows: &[Row],
        table: &BTreeMap<&str, Vec<f64>>,
        holistic: &Scores<'_>,
        unit: fn(&Row) -> Option<String>,
    ) -> Vec<String> {
        self.labelled(rows)
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
                        self.checks
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
                    "injected": self.injected(row),
                    "sound": primary(row, self.sound),
                    "checks": p,
                    "holistic_p_defect": holistic.get(row.id.as_str()),
                    "unit": unit(row),
                    "truth": truth,
                })
                .to_string()
            })
            .collect()
    }
}
