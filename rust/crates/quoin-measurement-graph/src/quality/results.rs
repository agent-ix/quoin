// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The scored results a measured population produces.
//!
//! Ports `dimensionCountSchema`, `confusionMatrixSchema`, `recallSchema` and
//! `resultsSchema` (`src/measurement/graph-adapters.ts:220-268`).

use std::num::NonZeroU64;

use serde::Deserialize;

use crate::scalars::{MalformedScalar, NonEmptyText, malformed};

use super::population::Dimension;

/// One count, sliced along one dimension.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DimensionCount {
    /// The axis.
    pub dimension: Dimension,
    /// The slice.
    pub key: NonEmptyText,
    /// How many.
    pub count: u64,
}

/// One confusion matrix, sliced along one dimension.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfusionMatrix {
    /// The axis.
    pub dimension: Dimension,
    /// The slice.
    pub key: NonEmptyText,
    /// Recovered and expected.
    pub true_positive: u64,
    /// Recovered and not expected.
    pub false_positive: u64,
    /// Expected and not recovered.
    pub false_negative: u64,
    /// Neither recovered nor expected.
    pub true_negative: u64,
}

impl ConfusionMatrix {
    /// The four components, in the order the transcription emits them.
    #[must_use]
    pub fn components(&self) -> [(&'static str, u64); 4] {
        [
            ("true_positive", self.true_positive),
            ("false_positive", self.false_positive),
            ("false_negative", self.false_negative),
            ("true_negative", self.true_negative),
        ]
    }
}

/// A proportion in `0..=1`.
///
/// `z.number().min(0).max(1)`. A newtype rather than a bare `f64` because the
/// bound is the whole of what makes it a ratio, and a `f64` that escaped the
/// bound would be transcribed as a measured value with `shape: "ratio"`.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Deserialize)]
#[serde(try_from = "f64")]
pub struct UnitRatio(f64);

impl UnitRatio {
    /// The proportion.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for UnitRatio {
    type Error = MalformedScalar;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        // `!(0.0..=1.0).contains(…)` rather than two comparisons: NaN is
        // outside every range, and zod's `.min(0).max(1)` refuses it too
        // because `NaN >= 0` is false.
        if (0.0..=1.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(malformed(format!("expected a ratio in 0..=1, got {value}")))
        }
    }
}

/// How much of what was expected along one dimension was recovered.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recall {
    /// The axis.
    pub dimension: Dimension,
    /// The slice.
    pub key: NonEmptyText,
    /// How many were recovered.
    pub recovered: u64,
    /// How many were expected. `z.number().int().min(1)`: a recall over an
    /// empty expectation is not a ratio.
    pub expected: NonZeroU64,
    /// The proportion.
    pub ratio: UnitRatio,
}

/// The scores a measured population produces.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Results {
    /// At least four confusion matrices.
    pub confusion_matrices: Vec<ConfusionMatrix>,
    /// What could not be resolved.
    pub unresolved: Vec<DimensionCount>,
    /// What resolved to more than one target.
    pub ambiguous: Vec<DimensionCount>,
    /// At least one recall row.
    pub recall: Vec<Recall>,
}

impl Results {
    /// The two array floors `resultsSchema` declares, which serde cannot.
    ///
    /// # Errors
    ///
    /// [`MalformedScalar`] naming the member and the floor it is under.
    pub fn check_floors(&self) -> Result<(), MalformedScalar> {
        if self.confusion_matrices.len() < 4 {
            return Err(malformed(format!(
                "confusion_matrices: expected at least 4, got {}",
                self.confusion_matrices.len()
            )));
        }
        if self.recall.is_empty() {
            return Err(malformed("recall: expected at least 1, got 0"));
        }
        Ok(())
    }
}
