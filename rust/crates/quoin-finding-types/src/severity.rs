// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The finding severity vocabulary, held open (quoin#383).

use std::fmt;

use serde::{Deserialize, Serialize};

/// How serious one finding is.
///
/// # Why this is a newtype and not a closed enum
///
/// `src/auditor/audit.ts:34` declares `type Severity = "low" | "medium" |
/// "high"`, and mirroring that as a three-variant Rust enum is the obvious
/// move. It is also the one this workspace has already paid for once: PR #492
/// declared exactly that enum for the graph-analysis port, and the workspace
/// gate then refused a finding `build_case` has always accepted, because
/// `quoin-assurance`'s captured corpus carries `severity: "error"`.
///
/// **A TypeScript type is not a runtime check.** The union is erased before
/// any value reaches the wire, nothing between the producer and here validates
/// it, and the retained corpora are wider than the declaration. Grepped across
/// `rust/`, `corpus/` and `tests/` before this type was written, the severities
/// actually retained are `error`, `medium`, `warning`, `high`, `advisory`,
/// `violation`, `yanked`, `unmaintained`, `vulnerability` and `unsound` — ten
/// spellings, of which the declaration admits two.
///
/// Where the retained data is wider than the declared type, the retained data
/// wins. [`Severity::LOW`], [`Severity::MEDIUM`] and [`Severity::HIGH`] name
/// the three the auditor itself mints; everything else round-trips unchanged.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
// Domain-qualified for the same reason as `AuditFinding`: `Severity` is
// already taken at the boundary, and `Severity2` names nothing.
#[cfg_attr(feature = "schema", schemars(rename = "AuditSeverity"))]
#[serde(transparent)]
pub struct Severity(Box<str>);

impl Severity {
    /// The spelling the auditor mints for work in progress.
    pub const LOW: &'static str = "low";
    /// The spelling the auditor mints for a heuristic or an ordinary gap.
    pub const MEDIUM: &'static str = "medium";
    /// The spelling the auditor mints for evidence that claims what it lacks.
    pub const HIGH: &'static str = "high";

    /// Every spelling the auditor itself produces, in declaration order.
    ///
    /// A reader's note, never a constraint: [`Severity::parse`] admits any
    /// string, which is the whole point of the type.
    pub const MINTED: [&'static str; 3] = [Self::LOW, Self::MEDIUM, Self::HIGH];

    /// Read a severity from whatever the producer wrote.
    ///
    /// Total by construction. There is no failing counterpart, because there
    /// is no value the retained implementation would have rejected.
    pub fn parse(text: impl Into<String>) -> Self {
        Self(text.into().into_boxed_str())
    }

    /// `low`.
    #[must_use]
    pub fn low() -> Self {
        Self::parse(Self::LOW)
    }

    /// `medium`.
    #[must_use]
    pub fn medium() -> Self {
        Self::parse(Self::MEDIUM)
    }

    /// `high`.
    #[must_use]
    pub fn high() -> Self {
        Self::parse(Self::HIGH)
    }

    /// The spelling, verbatim.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::Severity;

    #[test]
    fn the_three_minted_spellings_say_themselves() {
        assert_eq!(Severity::low().as_str(), "low");
        assert_eq!(Severity::medium().as_str(), "medium");
        assert_eq!(Severity::high().as_str(), "high");
        assert_eq!(Severity::MINTED, ["low", "medium", "high"]);
    }

    #[test]
    fn a_severity_the_declaration_never_admitted_round_trips() {
        // The exact value that refused a finding in PR #492: `quoin-assurance`
        // holds it in a captured corpus, and `build_case` has always taken it.
        for retained in [
            "error",
            "warning",
            "advisory",
            "violation",
            "yanked",
            "unmaintained",
            "vulnerability",
            "unsound",
        ] {
            let json = format!("\"{retained}\"");
            let severity: Severity = serde_json::from_str(&json).expect("any string is a severity");
            assert_eq!(severity.as_str(), retained);
            assert_eq!(serde_json::to_string(&severity).unwrap(), json);
        }
    }

    #[test]
    fn a_severity_is_a_bare_string_on_the_wire() {
        // `#[serde(transparent)]`: a newtype that serialised as `["high"]` or
        // `{"0":"high"}` would change every finding byte the store holds.
        assert_eq!(
            serde_json::to_string(&Severity::high()).unwrap(),
            "\"high\""
        );
    }
}
