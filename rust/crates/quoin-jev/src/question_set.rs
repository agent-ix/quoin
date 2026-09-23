// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Typed access to `skills/spec-criterion-strength-analysis/assets/question-set.json`
//! (PLAT-837).
//!
//! This module mirrors that asset; it does not restate its content. The
//! asset is owned by the fixture/question-set author (see PR #570's
//! `SKILL.md`: "Consume this unchanged. It was built to be handed to a
//! client exactly as-is") and is actively being revised on the same branch
//! this crate lands on, so the parser here is deliberately tolerant of
//! fields it does not read (no `#[serde(deny_unknown_fields)]`): the asset's
//! owner adding a field such as `scope_provenance` -- which happened once
//! already, mid-review -- must not fail this crate's parse. This is a
//! considered exception to `rust-style`'s "deny unknown fields on every
//! request type" rule, which governs wire input from *untrusted* external
//! callers; this file is a co-owned, same-repo asset, not that boundary.

use serde::Deserialize;
use typesafe_sdk_questions::{Question, Questions, choice_of, noul, score};

use crate::error::{JevError, JevErrorCode, Result};

/// One `noul` question entry.
#[derive(Debug, Clone, Deserialize)]
pub struct NoulEntry {
    /// The question's id, used as (part of) its wire key.
    pub id: String,
    /// The instructions sent to Jev.
    pub question: String,
}

/// The `weakness_kind` choice block.
#[derive(Debug, Clone, Deserialize)]
pub struct ChoiceBlock {
    /// The question's id.
    pub id: String,
    /// The instructions sent to Jev.
    ///
    /// In the asset rather than in this crate for the same reason the `noul`
    /// questions are: the wording is part of the measured instrument, and a
    /// wording change has to be something a test can vary and a reviewer can
    /// see in the asset's diff (PLAT-917).
    pub question: String,
    /// The closed answer space, in the order the ticket states it.
    pub answer_space: Vec<String>,
}

/// One `adverse_case_coverage` rubric level.
#[derive(Debug, Clone, Deserialize)]
pub struct RubricLevel {
    /// The numeric level, 0-based.
    pub level: u8,
    /// The rubric's label for this level.
    pub label: String,
}

/// The `adverse_case_coverage` score block.
#[derive(Debug, Clone, Deserialize)]
pub struct ScoreBlock {
    /// The question's id.
    pub id: String,
    /// The rubric, indexed from zero.
    pub rubric: Vec<RubricLevel>,
}

/// The whole question-set asset, as much of it as this crate reads.
#[derive(Debug, Clone, Deserialize)]
pub struct QuestionSet {
    /// The five `noul` questions, asked once per AC row.
    pub noul: Vec<NoulEntry>,
    /// The `weakness_kind` choice, asked once per AC row.
    pub choice: ChoiceBlock,
    /// The `adverse_case_coverage` score, asked once per FR.
    pub score: ScoreBlock,
}

impl QuestionSet {
    /// Parses the asset from its JSON text.
    ///
    /// # Errors
    /// [`JevErrorCode::InvalidConfig`] when the text is not the documented
    /// shape.
    pub fn parse(json: &str) -> Result<Self> {
        serde_json::from_str(json)
            .map_err(|error| JevError::new(JevErrorCode::InvalidConfig, error.to_string()))
    }

    /// The wire key one AC row's `weakness_kind` answer carries.
    #[must_use]
    pub fn weakness_kind_key(&self, ac_id: &str) -> String {
        format!("{ac_id}::{}", self.choice.id)
    }

    /// The wire key one AC row's `noul` question `entry` carries.
    #[must_use]
    pub fn noul_key(ac_id: &str, entry: &NoulEntry) -> String {
        format!("{ac_id}::{}", entry.id)
    }

    /// The wire key the FR-scoped `adverse_case_coverage` answer carries.
    ///
    /// Never AC-prefixed: the rubric is aggregate over the FR's whole AC set
    /// (`question-set.json`'s own `score.scope`), so one FR contributes
    /// exactly one such key regardless of how many AC rows it has.
    #[must_use]
    pub fn adverse_case_coverage_key(&self) -> &str {
        &self.score.id
    }

    /// The five `noul` questions plus the `weakness_kind` choice for one AC
    /// row, keyed as [`Self::noul_key`]/[`Self::weakness_kind_key`] name them.
    pub fn ac_row_questions<'a>(
        &'a self,
        ac_id: &'a str,
    ) -> impl Iterator<Item = (String, Question)> + 'a {
        let noul_pairs = self
            .noul
            .iter()
            .map(move |entry| (Self::noul_key(ac_id, entry), noul(entry.question.as_str())));
        let weakness_kind = std::iter::once((
            self.weakness_kind_key(ac_id),
            choice_of(
                self.choice.question.as_str(),
                self.choice.answer_space.iter().map(String::as_str),
            ),
        ));
        noul_pairs.chain(weakness_kind)
    }

    /// The single `adverse_case_coverage` score question for one FR.
    #[must_use]
    pub fn adverse_case_coverage_question(&self) -> (String, Question) {
        let labels = self.score.rubric.iter().map(|level| level.label.as_str());
        (
            self.adverse_case_coverage_key().to_owned(),
            score(
                "Rate this FR's acceptance-criteria set for adverse-case coverage against the rubric.",
                labels,
            ),
        )
    }

    /// Builds the complete `Questions` map for one FR: every AC row's five
    /// `noul` plus `weakness_kind`, and the FR's single `adverse_case_coverage`
    /// -- the whole thing PLAT-837's request shape asks be one Jev call.
    pub fn questions_for_fr<'a>(&self, ac_ids: impl IntoIterator<Item = &'a str>) -> Questions {
        let mut questions = Questions::new();
        for ac_id in ac_ids {
            for (key, question) in self.ac_row_questions(ac_id) {
                questions.insert(key, question);
            }
        }
        let (key, question) = self.adverse_case_coverage_question();
        questions.insert(key, question);
        questions
    }

    /// The rubric's labels, indexed by level, for rendering a score's legend
    /// back into readable text.
    #[must_use]
    pub fn rubric_label(&self, level: u8) -> Option<&str> {
        self.score
            .rubric
            .iter()
            .find(|entry| entry.level == level)
            .map(|entry| entry.label.as_str())
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::QuestionSet;

    /// The real asset, read at compile time so a test failure here means the
    /// asset genuinely changed shape, not that a fixture drifted from it.
    const ASSET: &str = include_str!(
        "../../../../skills/spec-criterion-strength-analysis/assets/question-set.json"
    );

    /// Provenance: PLAT-837
    #[test]
    fn the_real_asset_parses() {
        let set = QuestionSet::parse(ASSET).expect("the shipped asset parses");
        assert_eq!(set.noul.len(), 5);
        assert_eq!(set.choice.answer_space.len(), 6);
        assert_eq!(set.score.rubric.len(), 4);
    }

    /// Provenance: PLAT-837
    #[test]
    fn one_fr_with_two_ac_rows_yields_thirteen_questions() {
        let set = QuestionSet::parse(ASSET).expect("parses");
        // 2 AC rows * (5 noul + 1 choice) + 1 FR-scoped score = 13.
        let questions = set.questions_for_fr(["FR-001-AC-1", "FR-001-AC-2"]);
        assert_eq!(questions.len(), 13);
        assert!(questions.contains_key("FR-001-AC-1::falsifiable"));
        assert!(questions.contains_key("FR-001-AC-1::weakness_kind"));
        assert!(questions.contains_key("adverse_case_coverage"));
    }

    /// Provenance: PLAT-837. The score is asked exactly once per FR
    /// regardless of AC-row count -- not once per row.
    #[test]
    fn adverse_case_coverage_is_not_ac_prefixed() {
        let set = QuestionSet::parse(ASSET).expect("parses");
        let questions = set.questions_for_fr(["FR-001-AC-1", "FR-001-AC-2", "FR-001-AC-3"]);
        let coverage_keys: Vec<_> = questions
            .keys()
            .filter(|k| k.contains("adverse_case_coverage"))
            .collect();
        assert_eq!(coverage_keys, vec!["adverse_case_coverage"]);
    }
}
