// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Architecture-conformance audit scripts → findings (FR-036).
//!
//! **Why this is the format worth reading for architecture conformance.**
//! Specs and ADRs declare boundaries — "engine/surfaces split", "no solver dep
//! in core", "thin CLI" — that rot silently unless something enforces them. The
//! audit-script pattern is the worked prior art (`quire-rs/scripts/audits/`, a
//! README mapping each script to the criteria it audits), and this reads its
//! output without asking anyone to change it.
//!
//! **The join is literal here, which it is not for most scanners.** An audit
//! script already prints the criterion in its failure line, so the obligation it
//! discharges is stated by the tool rather than mapped after the fact.
//!
//! **A script reporting `OK` is a rule that ran and passed**, which is why
//! `rulesEvaluated` counts every line of either kind. A file with no recognised
//! line means no audit ran, and that is vacuous (FR-034).

use std::sync::LazyLock;

use regex::Regex;

use super::FindingResult;
use crate::error::EvidenceError;
use crate::types::Finding;

/// Compile a pattern that is a source literal. See `junit::literal_regex`.
#[allow(
    clippy::expect_used,
    reason = "patterns are module literals covered by this module's tests and the golden corpus; a failure here is a typo caught by the suite, not a runtime condition"
)]
fn literal_regex(pattern: &'static str) -> Regex {
    Regex::new(pattern).expect("adapter pattern literal must compile")
}

/// `/^([A-Za-z0-9._-]+):\s*(OK|FAIL)\b\s*(?:[—-]\s*)?(.*)$/`
///
/// The convention `quire-rs/scripts/audits/README.md` states and every script
/// there follows: *"All scripts SHALL exit 0 on success and non-zero on
/// violation, with descriptive stderr output."*
static LINE: LazyLock<Regex> =
    LazyLock::new(|| literal_regex(r"^([A-Za-z0-9._\-]+):\s*(OK|FAIL)\b\s*(?:[—\-]\s*)?(.*)$"));

/// Acceptance-criterion ids named in a trailing parenthetical.
///
/// Trailing punctuation is allowed after the closing paren because the real
/// scripts write one: `check_no_schemars: FAIL — 'schemars' present in
/// Cargo.lock (quire-rs FR-003-AC-4).` An anchor demanding end-of-line found
/// nothing, and the finding lost the very criterion that makes it a discharge
/// rather than a complaint.
static TRACE_IDS: LazyLock<Regex> = LazyLock::new(|| literal_regex(r"\(([^)]*)\)[.\s]*$"));

static AC_ID: LazyLock<Regex> =
    LazyLock::new(|| literal_regex(r"\b((?:FR|NFR|StR|US)-\d+-AC-\d+|TC-\d+)\b"));

fn trace_ids_of(message: &str) -> Option<Vec<String>> {
    let tail = TRACE_IDS.captures(message)?;
    let inner = tail.get(1)?.as_str();
    let ids: Vec<String> = AC_ID
        .captures_iter(inner)
        .filter_map(|capture| capture.get(1).map(|m| m.as_str().to_owned()))
        .collect();
    (!ids.is_empty()).then_some(ids)
}

/// Parse audit-script stdout.
///
/// # Errors
///
/// Refuses input with no `<name>: OK` and no `<name>: FAIL` line.
pub fn parse_audit_script(raw: &str) -> Result<FindingResult, EvidenceError> {
    let mut findings: Vec<Finding> = Vec::new();
    let mut evaluated: u64 = 0;
    for line in raw.split('\n') {
        let Some(captures) = LINE.captures(line.trim()) else {
            continue;
        };
        let (Some(name), Some(verdict), Some(rest)) =
            (captures.get(1), captures.get(2), captures.get(3))
        else {
            continue;
        };
        evaluated = evaluated.saturating_add(1);
        if verdict.as_str() == "OK" {
            continue;
        }
        let message = rest.as_str().trim();
        let mut finding = Finding::new(name.as_str());
        finding.severity = Some("violation".to_owned());
        finding.message = (!message.is_empty()).then(|| message.to_owned());
        finding.trace_ids = trace_ids_of(message);
        findings.push(finding);
    }
    if evaluated == 0 {
        return Err(EvidenceError::adapter(
            "audit-script",
            "no `<name>: OK` or `<name>: FAIL` line found — is this audit-script output?",
        ));
    }
    Ok(FindingResult {
        findings,
        tool: Some("audit-script".to_owned()),
        ruleset: None,
        rules_evaluated: Some(evaluated),
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::parse_audit_script;

    /// A FAIL line becomes a finding carrying the criterion the script itself
    /// printed, and OK lines count as rules evaluated — so a fully passing run
    /// is a result rather than a vacuous one.
    ///
    /// Trace: FR-036-AC-2, FR-036-AC-3
    /// Provenance: quoin#458
    #[test]
    fn tc_458_210_counts_ok_lines_as_rules_that_ran_and_passed() {
        let result = parse_audit_script(
            "check_a: OK\ncheck_b: FAIL — boundary crossed (quire-rs FR-003-AC-4).\nnoise\n",
        )
        .unwrap();
        assert_eq!(result.rules_evaluated, Some(2));
        assert_eq!(result.findings.len(), 1);
        assert_eq!(result.findings[0].rule_id, "check_b");
        assert_eq!(result.findings[0].severity.as_deref(), Some("violation"));
        assert_eq!(
            result.findings[0].trace_ids.as_deref(),
            Some(["FR-003-AC-4".to_owned()].as_slice())
        );
    }

    /// Trace: FR-036-AC-2
    /// Provenance: quoin#458
    #[test]
    fn tc_458_211_reads_a_criterion_through_trailing_punctuation() {
        // The incident: an end-of-line anchor found nothing, and the finding
        // lost the criterion that makes it a discharge rather than a complaint.
        for tail in ["(TC-9)", "(TC-9).", "(TC-9). ", "(TC-9)  "] {
            let result = parse_audit_script(&format!("c: FAIL — bad {tail}")).unwrap();
            assert_eq!(
                result.findings[0].trace_ids.as_deref(),
                Some(["TC-9".to_owned()].as_slice()),
                "{tail}"
            );
        }
    }

    #[test]
    fn a_failure_with_no_reason_carries_no_message() {
        let result = parse_audit_script("c: FAIL").unwrap();
        assert_eq!(result.findings[0].message, None);
        assert_eq!(result.findings[0].trace_ids, None);
    }

    /// No recognised line means no audit ran; recording it would manufacture
    /// conformance evidence from a file that proves nothing.
    ///
    /// Trace: FR-036-AC-4
    /// Provenance: quoin#458
    #[test]
    fn tc_458_212_refuses_output_with_no_recognised_line() {
        assert_eq!(
            parse_audit_script("all good\n").unwrap_err().to_string(),
            "audit-script: no `<name>: OK` or `<name>: FAIL` line found — is this audit-script output?"
        );
        assert!(
            parse_audit_script("Compiling quire-rs v0.33.0\nFinished")
                .unwrap_err()
                .to_string()
                .contains("audit-script output")
        );
    }
}
