// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Agent-IX

//! Verdict policy over declared-vocabulary coverage (FR-037).
//!
//! quire-rs FR-059 answers *"which declared values does no document claim?"*.
//! That is a deterministic fact about the spec, and it lives in the engine.
//!
//! **What it does not answer is whether an exclusion was earned.** The engine
//! accepts a bare list — `quality_attributes_not_applicable: [safety,
//! compliance]` — and every value in it stops being reported. Measured on the
//! quoin repository: 7 findings before, **5 after adding that one line**, with
//! no reason written anywhere. So the cheapest way to make the check quiet is to
//! excuse everything, and nothing notices.
//!
//! That is the policy question ADR-0011 leaves to quoin, and it is what this
//! module decides: an exclusion is a **claim about the product** and must carry
//! a written reason, in the same document, naming the value.

use std::collections::BTreeSet;

use crate::declarations::VocabularyDeclaration;
use crate::ids::{VocabularyName, VocabularyValue};

/// What kind of gap a finding records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FindingKind {
    /// No document claims the value and nothing excuses it.
    Unowned,
    /// A document excuses the value with no written reason.
    UnjustifiedExclusion,
    /// A document excuses a value the vocabulary does not declare.
    UndeclaredExclusion,
}

impl FindingKind {
    /// The wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unowned => "unowned",
            Self::UnjustifiedExclusion => "unjustified-exclusion",
            Self::UndeclaredExclusion => "undeclared-exclusion",
        }
    }
}

/// How much a finding is worth.
///
/// Note the asymmetry, which **is** the policy: `unowned` is medium and an
/// unjustified exclusion is high. Saying nothing about reliability is an
/// admitted gap a reader can see. Excusing it without a reason is an assertion
/// of completeness with nothing behind it, and it removes the finding that would
/// have prompted the work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// An admitted gap.
    Medium,
    /// An assertion with nothing behind it.
    High,
}

impl Severity {
    /// The wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

/// One gap in declared-vocabulary coverage.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CompletenessFinding {
    /// Declaration this concerns, e.g. `quality-characteristics`.
    pub vocabulary: VocabularyName,
    /// The vocabulary value, e.g. `safety`.
    pub value: VocabularyValue,
    /// What kind of gap it is.
    pub kind: FindingKind,
    /// How much it is worth.
    pub severity: Severity,
    /// Human-readable detail. Not contractual.
    pub message: String,
    /// Document carrying the exclusion, for the two exclusion kinds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<String>,
}

/// The per-vocabulary tally.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct VocabularyRollup {
    /// Which declaration.
    pub vocabulary: VocabularyName,
    /// Values in the declared enum.
    pub declared: usize,
    /// Values some document claims.
    pub owned: usize,
    /// Values a document excuses, justified or not.
    pub excused: usize,
    /// Values neither claimed nor excused.
    pub unowned: usize,
}

/// One document's declared-vocabulary frontmatter, as read from the bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentClaims {
    /// Path, relative to the bundle root, with `/` separators.
    pub path: String,
    /// Values this document claims via the declaration's `field`.
    pub claims: Vec<String>,
    /// Values this document excuses via `justified_absence_field`.
    pub excuses: Vec<String>,
    /// Raw body text, searched for the written reason behind each excuse.
    pub body: String,
}

/// `UNCHECKED` is not a fourth flavour of pass.
///
/// A bundle whose module set declares no vocabulary has not been assessed, and
/// `PASS` over it is the green-matrix-over-dead-links result this program was
/// created to stop — the first draft of the TypeScript printed exactly that, and
/// the criterion written to forbid it caught it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Verdict {
    /// Checked, nothing found.
    Pass,
    /// Checked, only admitted gaps, and not `--strict`.
    Conditional,
    /// A high-severity finding, or an admitted gap under `--strict`.
    Fail,
    /// Nothing was checked. Not a pass.
    Unchecked,
}

impl Verdict {
    /// The wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Conditional => "CONDITIONAL",
            Self::Fail => "FAIL",
            Self::Unchecked => "UNCHECKED",
        }
    }
}

/// What one assessment produced.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CompletenessReport {
    /// One tally per declaration.
    pub rollups: Vec<VocabularyRollup>,
    /// Every gap found.
    pub findings: Vec<CompletenessFinding>,
    /// The verdict over them.
    pub verdict: Verdict,
}

/// One declaration's tally and findings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assessment {
    /// The tally.
    pub rollup: VocabularyRollup,
    /// The gaps.
    pub findings: Vec<CompletenessFinding>,
}

/// Reasons that are not reasons.
///
/// A rationale cell reading `-` or `TBD` is an author acknowledging the question
/// and declining to answer it, which is exactly what an unjustified exclusion
/// is. Listed explicitly rather than caught by a length floor: a floor teaches
/// people to pad, and `n/a` and a forty-character sentence saying nothing are
/// the same problem measured differently.
const NON_ANSWERS: [&str; 9] = ["-", "—", "n/a", "na", "none", "tbd", "todo", "?", ""];

/// The smallest number of words that can state a reason.
///
/// "controls no hardware" is three. Two cannot carry subject and predicate, and
/// one is a label. This is a floor on *structure*, not on length — it rejects
/// `safety: no` without inviting padding, which a character count does.
const MIN_REASON_WORDS: usize = 3;

/// Assess one vocabulary declaration against what the bundle's documents say.
#[must_use]
pub fn assess_vocabulary(
    declaration: &VocabularyDeclaration,
    documents: &[DocumentClaims],
) -> Assessment {
    let declared: BTreeSet<&str> = declaration.values.iter().map(String::as_str).collect();
    let mut owned: BTreeSet<&str> = BTreeSet::new();
    let mut excused: BTreeSet<&str> = BTreeSet::new();
    let mut findings: Vec<CompletenessFinding> = Vec::new();

    let absence_field = declaration
        .justified_absence_field
        .as_deref()
        .unwrap_or("undefined");

    for document in documents {
        for value in &document.claims {
            if let Some(known) = declared.get(value.as_str()) {
                owned.insert(known);
            }
        }
        for value in &document.excuses {
            let Some(known) = declared.get(value.as_str()).copied() else {
                // A typo'd exclusion excuses nothing — the real value keeps
                // reporting — while reading, to its author, as handled. Worth
                // naming on its own.
                findings.push(CompletenessFinding {
                    vocabulary: declaration.name.clone(),
                    value: VocabularyValue::new(value.clone()),
                    kind: FindingKind::UndeclaredExclusion,
                    severity: Severity::High,
                    document: Some(document.path.clone()),
                    message: format!(
                        "'{value}' is excused under '{absence_field}' but is not one of the {} \
                         values '{}' declares, so it excuses nothing",
                        declared.len(),
                        declaration.name
                    ),
                });
                continue;
            };
            excused.insert(known);
            if written_reason_for(value, &document.body).is_none() {
                findings.push(CompletenessFinding {
                    vocabulary: declaration.name.clone(),
                    value: VocabularyValue::new(value.clone()),
                    kind: FindingKind::UnjustifiedExclusion,
                    severity: Severity::High,
                    document: Some(document.path.clone()),
                    message: format!(
                        "'{value}' is excused under '{absence_field}' with no written reason; \
                         state why it does not apply in a table row naming '{value}'"
                    ),
                });
            }
        }
    }

    for value in &declaration.values {
        if owned.contains(value.as_str()) || excused.contains(value.as_str()) {
            continue;
        }
        findings.push(CompletenessFinding {
            vocabulary: declaration.name.clone(),
            value: VocabularyValue::new(value.clone()),
            kind: FindingKind::Unowned,
            severity: Severity::Medium,
            document: None,
            message: format!(
                "no document claims '{value}' for '{}', and nothing records it under '{}'",
                declaration.field,
                declaration
                    .justified_absence_field
                    .as_deref()
                    .unwrap_or("a justified-absence field")
            ),
        });
    }

    let unowned = declaration
        .values
        .iter()
        .filter(|v| !owned.contains(v.as_str()) && !excused.contains(v.as_str()))
        .count();

    Assessment {
        rollup: VocabularyRollup {
            vocabulary: declaration.name.clone(),
            declared: declared.len(),
            owned: owned.len(),
            excused: excused.len(),
            unowned,
        },
        findings,
    }
}

/// The written reason for excusing `value`, or `None` when there is none.
///
/// A table row whose first cell names the value and whose remaining cells carry
/// a real sentence. A table is required rather than free prose because the value
/// name will occur in passing — "safety" appears in any document discussing
/// safety — and a mention is not a justification.
#[must_use]
pub fn written_reason_for(value: &str, body: &str) -> Option<String> {
    for line in body.split('\n') {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        // `split("|").slice(1, -1)`: drop the empty fields either side of the
        // leading and trailing pipes. A row with no trailing pipe therefore
        // loses its last cell, which is the TypeScript's behaviour too.
        let fields: Vec<&str> = trimmed.split('|').collect();
        if fields.len() < 2 {
            continue;
        }
        let cells: Vec<String> = fields
            .get(1..fields.len() - 1)
            .unwrap_or_default()
            .iter()
            .map(|cell| {
                cell.trim()
                    .trim_start_matches('`')
                    .trim_end_matches('`')
                    .trim()
                    .to_owned()
            })
            .collect();
        if cells.len() < 2 {
            continue;
        }
        let Some(first) = cells.first() else { continue };
        if !first.eq_ignore_ascii_case(value) {
            continue;
        }
        for cell in cells.iter().skip(1) {
            if is_reason(cell) {
                return Some(cell.clone());
            }
        }
    }
    None
}

/// True when a cell states something rather than declining to.
fn is_reason(cell: &str) -> bool {
    let lowered = cell.to_lowercase();
    let normalized = lowered.trim_end_matches(['.', ' ', '\t', '\n', '\r']);
    if NON_ANSWERS.contains(&normalized) {
        return false;
    }
    cell.split_whitespace().count() >= MIN_REASON_WORDS
}

/// The verdict over every finding.
///
/// `--strict` promotes an admitted gap to a failure; it does not invent one. The
/// default is advisory because a new check landing as a hard error across a
/// corpus teaches people to disable it, and a disabled check reports nothing
/// forever — which is the outcome this whole area exists to prevent.
#[must_use]
pub fn verdict_for(
    findings: &[CompletenessFinding],
    strict: bool,
    vocabularies_checked: usize,
) -> Verdict {
    if vocabularies_checked == 0 {
        return Verdict::Unchecked;
    }
    if findings.iter().any(|f| f.severity == Severity::High) {
        return Verdict::Fail;
    }
    if findings.is_empty() {
        return Verdict::Pass;
    }
    if strict {
        Verdict::Fail
    } else {
        Verdict::Conditional
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trace: FR-037
    #[test]
    fn tc_378_210_non_answers_are_rejected_and_real_sentences_accepted() {
        for non_answer in [
            "-", "—", "n/a", "N/A", "none", "TBD", "todo", "?", "", "tbd.",
        ] {
            assert!(!is_reason(non_answer), "{non_answer:?} is not a reason");
        }
        assert!(is_reason("controls no hardware"));
        assert!(
            !is_reason("not applicable"),
            "two words cannot state a reason"
        );
    }

    /// Trace: FR-037
    #[test]
    fn tc_378_211_a_mention_in_prose_is_not_a_justification() {
        assert_eq!(
            written_reason_for("safety", "safety does not apply to this product at all\n"),
            None
        );
    }

    /// Trace: FR-037
    #[test]
    fn tc_378_212_unchecked_outranks_every_other_verdict() {
        let high = CompletenessFinding {
            vocabulary: VocabularyName::from("v"),
            value: VocabularyValue::from("a"),
            kind: FindingKind::UnjustifiedExclusion,
            severity: Severity::High,
            message: String::new(),
            document: None,
        };
        assert_eq!(verdict_for(&[], false, 0), Verdict::Unchecked);
        assert_eq!(verdict_for(&[high], true, 0), Verdict::Unchecked);
    }

    /// Trace: FR-037
    #[test]
    fn tc_378_213_strict_promotes_only_admitted_gaps() {
        let medium = CompletenessFinding {
            vocabulary: VocabularyName::from("v"),
            value: VocabularyValue::from("a"),
            kind: FindingKind::Unowned,
            severity: Severity::Medium,
            message: String::new(),
            document: None,
        };
        assert_eq!(
            verdict_for(std::slice::from_ref(&medium), false, 1),
            Verdict::Conditional
        );
        assert_eq!(verdict_for(&[medium], true, 1), Verdict::Fail);
        assert_eq!(verdict_for(&[], true, 1), Verdict::Pass);
    }
}
