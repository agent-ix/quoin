// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The question keys corpus v2 labels, and the answer space of each
//! (PLAT-1027).
//!
//! One table, [`KEYS`], so the validator, the metrics and every variant read
//! the same fact. A new key is one row here.

use serde::{Deserialize, Serialize};

use crate::gap_semantic_support::{DIVERGENCE_KINDS, SEVERITY_RUBRIC};

/// A binary key's positive label. Truth written as JSON `true` reads as this.
pub(crate) const YES: &str = "yes";
/// A binary key's negative label. Truth written as JSON `false` reads as this.
pub(crate) const NO: &str = "no";
/// Every binary key's answer space.
pub(crate) const BINARY: [&str; 2] = [YES, NO];

/// Which artifacts a row must carry for a key to be labelled on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Needs {
    /// The requirement alone suffices.
    Requirement,
    /// The row must carry a test.
    Test,
    /// The row must carry code.
    Code,
    /// The row must carry a test, code, or both.
    TestOrCode,
}

/// One question key.
#[derive(Debug, Clone, Copy)]
pub(crate) struct KeySpec {
    /// The key, as it appears in a row's `truth`.
    pub(crate) key: &'static str,
    /// Every answer a truth label or a prediction may take.
    pub(crate) space: &'static [&'static str],
    /// Whether `space` is ordered lowest to highest, so ordering quality
    /// (pairwise concordance) is reported for it.
    pub(crate) ordinal: bool,
    /// The answer meaning "no defect", for defect / no-defect recall.
    pub(crate) no_defect: &'static str,
    /// Which artifacts a row must carry for this key to be labelled.
    pub(crate) needs: Needs,
}

const fn binary(key: &'static str, no_defect: &'static str, needs: Needs) -> KeySpec {
    KeySpec {
        key,
        space: &BINARY,
        ordinal: false,
        no_defect,
        needs,
    }
}

/// Every key PLAT-1024 names. Order is report order.
pub(crate) const KEYS: [KeySpec; 18] = [
    binary("trace_correct", YES, Needs::TestOrCode),
    binary("assertion_vacuous", NO, Needs::Test),
    binary("test_asserts_intent", YES, Needs::Test),
    binary("tests_only_its_own_mock", NO, Needs::Test),
    binary("code_implements_intent", YES, Needs::Code),
    binary("code_exceeds_requirement", NO, Needs::Code),
    KeySpec {
        key: "divergence_kind",
        space: &DIVERGENCE_KINDS,
        ordinal: false,
        no_defect: "aligned",
        needs: Needs::TestOrCode,
    },
    // `none < low < medium < high`: the gap-analysis rubric, lowest first.
    KeySpec {
        key: "severity",
        space: &SEVERITY_RUBRIC,
        ordinal: true,
        no_defect: "none",
        needs: Needs::Requirement,
    },
    binary("criterion_sound", YES, Needs::Requirement),
    binary("vague_term", NO, Needs::Requirement),
    binary("no_measurable_threshold", NO, Needs::Requirement),
    binary("untestable", NO, Needs::Requirement),
    binary("compound", NO, Needs::Requirement),
    binary("missing_trigger", NO, Needs::Requirement),
    // The requirement statement's own checks (`variants::statement`).
    binary("fr_statement_sound", YES, Needs::Requirement),
    binary("compound_obligation", NO, Needs::Requirement),
    binary("multiple_readings", NO, Needs::Requirement),
    binary("names_internal_symbol", NO, Needs::Requirement),
];

/// The spec for `key`, or `None` for a key no row may carry.
pub(crate) fn spec(key: &str) -> Option<&'static KeySpec> {
    KEYS.iter().find(|spec| spec.key == key)
}

/// A row's mode: which artifacts it carries besides the requirement.
///
/// Serialized as `R`, `RT`, `RC`, `RTC`, the spelling the corpus uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub(crate) enum Mode {
    /// The requirement alone.
    #[serde(rename = "R")]
    Req,
    /// Requirement and test, no code.
    #[serde(rename = "RT")]
    ReqTest,
    /// Requirement and code, no test.
    #[serde(rename = "RC")]
    ReqCode,
    /// The full triple.
    #[serde(rename = "RTC")]
    ReqTestCode,
}

impl Mode {
    /// Every mode, in report order.
    pub(crate) const ALL: [Self; 4] = [Self::Req, Self::ReqTest, Self::ReqCode, Self::ReqTestCode];

    /// The corpus spelling.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Req => "R",
            Self::ReqTest => "RT",
            Self::ReqCode => "RC",
            Self::ReqTestCode => "RTC",
        }
    }

    /// Whether a row in this mode carries a test.
    pub(crate) const fn has_test(self) -> bool {
        matches!(self, Self::ReqTest | Self::ReqTestCode)
    }

    /// Whether a row in this mode carries code.
    pub(crate) const fn has_code(self) -> bool {
        matches!(self, Self::ReqCode | Self::ReqTestCode)
    }

    /// Whether a row in this mode carries what `needs` asks for.
    pub(crate) const fn satisfies(self, needs: Needs) -> bool {
        match needs {
            Needs::Requirement => true,
            Needs::Test => self.has_test(),
            Needs::Code => self.has_code(),
            Needs::TestOrCode => self.has_test() || self.has_code(),
        }
    }
}
