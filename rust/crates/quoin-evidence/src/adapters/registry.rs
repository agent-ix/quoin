// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Which adapters exist, and which one a given invocation selects.
//!
//! Registration is a list rather than a hardcoded match so an external user can
//! add an adapter for their own tool. Nothing here is specific to agent-ix
//! repositories: the formats were chosen to exercise different corners of the
//! contract, not because of who produces them.

use super::{
    AdapterResult, FindingResult, agent_eval, audit_script, cargo_mutants, contract_conformance,
    differential_report, entries, junit, sarif, sbom,
};
use crate::error::EvidenceError;

/// One run-shaped adapter: a name, a `--help` line, the tool strings it claims,
/// and a pure text→entries function.
///
/// A function pointer rather than a trait object because there is one method
/// and no state: the purity the retained TypeScript could only state in a
/// doc comment is here a property of the type.
#[derive(Clone, Copy, Debug)]
pub struct RunAdapter {
    /// Registry key, and the value `--adapter` takes.
    pub name: &'static str,
    /// One line for `--help`; what format this reads.
    pub summary: &'static str,
    /// Tool identifiers this adapter claims, matched case-insensitively against
    /// the suite's declared `tool` as a substring. Empty means "never selected
    /// automatically" — an adapter that only runs when named explicitly.
    pub tools: &'static [&'static str],
    /// The parser.
    pub parse: fn(&str) -> Result<AdapterResult, EvidenceError>,
}

/// One finding-shaped adapter.
///
/// A separate registry because the two produce different record types and the
/// command must know which it is writing BEFORE it parses — a scan written into
/// `runs/` would lose the clean-versus-unrun distinction that FR-034 exists to
/// make, silently and at the point of intake.
#[derive(Clone, Copy, Debug)]
pub struct FindingAdapter {
    /// Registry key, and the value `--adapter` takes.
    pub name: &'static str,
    /// One line for `--help`.
    pub summary: &'static str,
    /// Tool identifiers this adapter claims.
    pub tools: &'static [&'static str],
    /// The parser.
    pub parse: fn(&str) -> Result<FindingResult, EvidenceError>,
}

/// Every run-shaped adapter quoin ships, in the order `--help` lists them.
pub static ADAPTERS: &[RunAdapter] = &[
    RunAdapter {
        name: "entries",
        summary: r#"Normalized {"entries": [{symbol, outcome, traceIds?}]} — the default."#,
        tools: &[],
        parse: entries::parse_entries,
    },
    RunAdapter {
        name: "junit",
        summary: "JUnit XML (<testsuite>/<testcase>), as emitted by most runners.",
        tools: &["junit", "pytest", "jest", "vitest", "gotestsum", "surefire"],
        parse: junit::parse_junit,
    },
    RunAdapter {
        name: "cargo-mutants",
        summary: "cargo-mutants outcomes.json — mutation score per mutated function.",
        tools: &["cargo-mutants", "cargo mutants"],
        parse: cargo_mutants::parse_cargo_mutants,
    },
    RunAdapter {
        name: "sbom",
        summary: "CycloneDX or SPDX JSON — one entry per component, so an empty inventory reads as vacuous.",
        tools: &["cyclonedx", "spdx", "syft", "cdxgen", "cargo-cyclonedx"],
        parse: sbom::parse_sbom,
    },
    RunAdapter {
        name: "agent-eval",
        summary: "cli-agent-evals report JSON — one entry per scenario, keyed on the scenario id.",
        tools: &["cli-agent-evals", "agent-evals", "agent-pty"],
        parse: agent_eval::parse_agent_eval,
    },
    RunAdapter {
        name: "contract-conformance",
        summary: concat!(
            "Contract conformance JSONL (",
            "quire.contract.conformance-jsonl/v1",
            ") — one replayed fixture per line."
        ),
        tools: &[
            "quire-contract-conformance",
            "contract-conformance",
            "quire-contract-ir",
        ],
        parse: contract_conformance::parse_contract_conformance,
    },
    RunAdapter {
        name: "differential-report",
        summary: "Domain differential summary (<domain>.differential-summary/v1) — engine versus external reference, case by case.",
        tools: &["differential-report", "tl-mltl", "r2u2", "c2po"],
        parse: differential_report::parse_differential_report,
    },
];

/// Every finding-shaped adapter quoin ships.
pub static FINDING_ADAPTERS: &[FindingAdapter] = &[
    FindingAdapter {
        name: "sarif",
        summary: "SARIF 2.1.0 — semgrep --sarif, CodeQL, ESLint, ZAP via converter.",
        tools: &["sarif", "semgrep", "codeql"],
        parse: sarif::parse_sarif,
    },
    FindingAdapter {
        name: "audit-script",
        summary: "Architecture-conformance audit scripts: '<name>: OK' / '<name>: FAIL — … (AC-ID)'.",
        tools: &[
            "audit-script",
            "make ci",
            "audit-static",
            "import-linter",
            "dependency-cruiser",
        ],
        parse: audit_script::parse_audit_script,
    },
    FindingAdapter {
        name: "cargo-audit",
        summary: "cargo audit --json — RUSTSEC advisories and warning kinds.",
        tools: &["cargo-audit", "cargo audit", "cargo-deny", "cargo deny"],
        parse: sarif::parse_cargo_audit,
    },
];

/// Every registered name, run-shaped first, in `--help` order.
#[must_use]
pub fn adapter_names() -> Vec<&'static str> {
    ADAPTERS
        .iter()
        .map(|adapter| adapter.name)
        .chain(FINDING_ADAPTERS.iter().map(|adapter| adapter.name))
        .collect()
}

/// The options a selection is made from: an explicit `--adapter` and the
/// suite's declared `--tool`.
#[derive(Debug, Clone, Copy, Default)]
pub struct AdapterSelection<'a> {
    /// An explicit `--adapter`. Empty is treated as absent, as in the retained
    /// source, where `""` fails the `!== ""` guard.
    pub adapter: Option<&'a str>,
    /// The suite's declared tool.
    pub tool: Option<&'a str>,
}

/// The finding-shaped adapter for these options, or `None` when the selection
/// is run-shaped.
///
/// Checked before [`select_adapter`] so a `--adapter sarif` never falls through
/// to the run path, where its output would be parsed as entries and fail with a
/// message about JSON shape.
#[must_use]
pub fn select_finding_adapter(selection: AdapterSelection<'_>) -> Option<FindingAdapter> {
    if let Some(name) = selection.adapter.filter(|name| !name.is_empty()) {
        return FINDING_ADAPTERS
            .iter()
            .find(|adapter| adapter.name == name)
            .copied();
    }
    let tool = selection.tool.unwrap_or("").to_lowercase();
    if tool.is_empty() {
        return None;
    }
    FINDING_ADAPTERS
        .iter()
        .find(|adapter| adapter.tools.iter().any(|claim| tool.contains(claim)))
        .copied()
}

/// Choose a run-shaped adapter: an explicit `--adapter` wins, else the suite's
/// declared `tool` selects one, else the normalized default.
///
/// **An unknown explicit name is an error, never a silent fall back to the
/// default.** Falling back would parse the file as normalized entries, fail
/// with a message about JSON shape, and send the reader to look at their `JUnit`
/// file instead of at their typo.
///
/// # Errors
///
/// Returns [`EvidenceError::UnknownAdapter`] when `--adapter` names an adapter
/// no registry holds.
pub fn select_adapter(selection: AdapterSelection<'_>) -> Result<RunAdapter, EvidenceError> {
    if let Some(name) = selection.adapter.filter(|name| !name.is_empty()) {
        return ADAPTERS
            .iter()
            .find(|adapter| adapter.name == name)
            .copied()
            .ok_or_else(|| EvidenceError::UnknownAdapter {
                name: name.to_owned(),
                available: adapter_names().join(", "),
            });
    }
    let tool = selection.tool.unwrap_or("").to_lowercase();
    if !tool.is_empty()
        && let Some(matched) = ADAPTERS
            .iter()
            .find(|adapter| adapter.tools.iter().any(|claim| tool.contains(claim)))
    {
        return Ok(*matched);
    }
    // The escape hatch that keeps the registry from being a gate.
    ADAPTERS
        .first()
        .copied()
        .ok_or_else(|| EvidenceError::adapter("adapter", "no adapters are registered"))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{
        ADAPTERS, AdapterSelection, FINDING_ADAPTERS, adapter_names, select_adapter,
        select_finding_adapter,
    };
    use std::collections::BTreeSet;

    #[test]
    fn the_registry_lists_every_shipped_adapter_run_shaped_first() {
        assert_eq!(
            adapter_names(),
            [
                "entries",
                "junit",
                "cargo-mutants",
                "sbom",
                "agent-eval",
                "contract-conformance",
                "differential-report",
                "sarif",
                "audit-script",
                "cargo-audit",
            ]
        );
        let distinct: BTreeSet<&str> = adapter_names().into_iter().collect();
        assert_eq!(distinct.len(), adapter_names().len());
        assert!(!ADAPTERS.is_empty() && !FINDING_ADAPTERS.is_empty());
    }

    #[test]
    fn an_explicit_name_wins_over_the_declared_tool() {
        let chosen = select_adapter(AdapterSelection {
            adapter: Some("junit"),
            tool: Some("cargo-mutants"),
        })
        .unwrap();
        assert_eq!(chosen.name, "junit");
    }

    #[test]
    fn an_unknown_explicit_name_is_refused_and_lists_what_exists() {
        let error = select_adapter(AdapterSelection {
            adapter: Some("juint"),
            tool: None,
        })
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            format!(
                "adapter: unknown adapter 'juint'. Available: {}",
                adapter_names().join(", ")
            )
        );
    }

    #[test]
    fn a_declared_tool_matches_case_insensitively_as_a_substring() {
        let chosen = select_adapter(AdapterSelection {
            adapter: None,
            tool: Some("PyTest 8.0"),
        })
        .unwrap();
        assert_eq!(chosen.name, "junit");
    }

    #[test]
    fn an_unmatched_tool_falls_back_to_the_normalized_default() {
        let chosen = select_adapter(AdapterSelection {
            adapter: None,
            tool: Some("a tool nobody claims"),
        })
        .unwrap();
        assert_eq!(chosen.name, "entries");
        assert_eq!(
            select_adapter(AdapterSelection::default()).unwrap().name,
            "entries"
        );
    }

    #[test]
    fn a_finding_shaped_selection_is_caught_before_the_run_path() {
        assert_eq!(
            select_finding_adapter(AdapterSelection {
                adapter: Some("sarif"),
                tool: None
            })
            .unwrap()
            .name,
            "sarif"
        );
        assert_eq!(
            select_finding_adapter(AdapterSelection {
                adapter: None,
                tool: Some("semgrep")
            })
            .unwrap()
            .name,
            "sarif"
        );
        assert!(
            select_finding_adapter(AdapterSelection {
                adapter: Some("junit"),
                tool: None
            })
            .is_none()
        );
        assert!(select_finding_adapter(AdapterSelection::default()).is_none());
    }
}
