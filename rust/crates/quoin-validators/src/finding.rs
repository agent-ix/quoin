// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The validator payload (quoin#377).
//!
//! Field order below is the order `JSON.stringify` emits for the TypeScript
//! object literal in `src/validators/gates.ts`, and serde preserves declaration
//! order, so the two implementations produce byte-identical JSON. That is not
//! decoration: the golden corpus compares serialised payloads.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ids::{LineNumber, ObligationId, RepoPath};

/// The class of defect a finding reports.
///
/// One variant today. It is an enum and not a `&'static str` because the kind is
/// the payload's discriminant: a second validator adds a variant here and every
/// `match` on it becomes a compiler-checked edit site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum FindingKind {
    /// A declared, wired shell gate that counts forbidden matches but never
    /// asserts the count, so it succeeds while the forbidden text is present.
    GateThatGatesNothing,
}

impl FindingKind {
    /// The wire spelling of the kind.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GateThatGatesNothing => "gate-that-gates-nothing",
        }
    }
}

impl fmt::Display for FindingKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One located gate-that-gates-nothing defect.
///
/// `subject`, `changeTarget`, `remedy` and `summary` are advisory prose for the
/// operator. Verdict parity is defined on `(kind, obligation, path, line,
/// wiredBy)`; the prose is reproduced faithfully because it is part of the
/// emitted payload, not because its wording is contractual.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmptyGateFinding {
    /// Always [`FindingKind::GateThatGatesNothing`] today.
    pub kind: FindingKind,
    /// The obligation the gate comment claims to enforce.
    pub obligation: ObligationId,
    /// The shell script holding the unasserted count.
    pub path: RepoPath,
    /// The 1-based line of the unasserted count.
    pub line: LineNumber,
    /// The build or CI file that wires the script, proving it is a gate.
    pub wired_by: RepoPath,
    /// Human label for the gate.
    pub subject: String,
    /// `path:line`, the exact locus an operator must edit.
    pub change_target: String,
    /// What to change.
    pub remedy: String,
    /// The full three-way join, stated once.
    pub summary: String,
}

impl EmptyGateFinding {
    /// The `(path, line, obligation)` key findings are ordered by.
    ///
    /// Exposed so a caller sorting a merged result set orders it the same way
    /// this crate does.
    #[must_use]
    pub fn sort_key(&self) -> (&str, u64, &str) {
        (
            self.path.as_str(),
            self.line.get(),
            self.obligation.as_str(),
        )
    }
}

/// The payload of `quoin validate --json`: `{ "findings": [ ... ] }`.
///
/// A `Serialize` struct rather than a hand-built `serde_json::Value`, so the
/// emitted shape has a field list a reviewer can read and the compiler checks
/// every branch populates it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateReport {
    /// Findings, ordered by `(path, line, obligation)`.
    pub findings: Vec<EmptyGateFinding>,
}

/// What a caller should do with a report.
///
/// The TypeScript command encodes this as `flags.strict && findings.length > 0`
/// inline. Naming the three states keeps "advisory by default" a decision the
/// caller can read rather than a conjunction it must re-derive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Verdict {
    /// No findings.
    Clean,
    /// Findings present, reported without failing (the default).
    Advisory,
    /// Findings present under `--strict`.
    Failing,
}

impl Verdict {
    /// The process exit code this verdict implies.
    #[must_use]
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::Clean | Self::Advisory => 0,
            Self::Failing => 1,
        }
    }
}

impl GateReport {
    /// Wrap findings for emission.
    #[must_use]
    pub const fn new(findings: Vec<EmptyGateFinding>) -> Self {
        Self { findings }
    }

    /// The verdict for this report at the given strictness.
    #[must_use]
    pub const fn verdict(&self, strict: bool) -> Verdict {
        if self.findings.is_empty() {
            Verdict::Clean
        } else if strict {
            Verdict::Failing
        } else {
            Verdict::Advisory
        }
    }

    /// The human-readable lines the `validate` command prints, in order.
    #[must_use]
    pub fn human_lines(&self) -> Vec<String> {
        if self.findings.is_empty() {
            return vec!["repository QA gates: no findings".to_owned()];
        }
        self.findings
            .iter()
            .map(|finding| {
                format!(
                    "[warning] {}: {}:{}: {}",
                    finding.kind, finding.path, finding.line, finding.summary
                )
            })
            .chain(std::iter::once(format!(
                "{} gate finding(s)",
                self.findings.len()
            )))
            .collect()
    }
}
