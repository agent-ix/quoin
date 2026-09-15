// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! SARIF 2.1.0 and `cargo audit --json` → findings.
//!
//! **SARIF is the primary finding adapter** because many scanners converge on
//! it — semgrep (`--sarif`), `CodeQL`, `ESLint`, ZAP via its converter — so one
//! robust reader subsumes several bespoke ones. Bespoke adapters are kept only
//! where a tool emits no SARIF at all, which is why `cargo-audit` lives here
//! beside it rather than under it.
//!
//! A `run` with an empty `results` array is a scan that **happened and found
//! nothing**. That is the whole reason this record type exists, and SARIF
//! already models it: the run envelope is the proof of execution, independent
//! of whether anything was reported.

use serde_json::Value;

use super::FindingResult;
use crate::error::EvidenceError;
use crate::types::Finding;

/// Parse a SARIF 2.1.0 log.
///
/// # Errors
///
/// Refuses input that is not JSON, a log with no `runs` array, and a log whose
/// `runs` array is empty.
pub fn parse_sarif(raw: &str) -> Result<FindingResult, EvidenceError> {
    let log: Value = serde_json::from_str(raw)
        .map_err(|error| EvidenceError::adapter("sarif", format!("input is not JSON: {error}")))?;
    let runs = log.get("runs").and_then(Value::as_array).ok_or_else(|| {
        EvidenceError::adapter("sarif", "no `runs` array — expected a SARIF 2.1.0 log")
    })?;
    if runs.is_empty() {
        // Not the same as a run with no results. A log carrying no run at all
        // is a file that proves nothing executed, and recording it would
        // manufacture exactly the evidence this record type exists to
        // distinguish.
        return Err(EvidenceError::adapter(
            "sarif",
            "`runs` is empty — a log with no run proves no scan executed",
        ));
    }

    let mut findings: Vec<Finding> = Vec::new();
    let mut tools: Vec<String> = Vec::new();
    // `None` until some driver declares a `rules` array. The distinction
    // between "the tool reported ZERO rules" and "the tool reported NO count"
    // is the one FR-034 turns on: the first is a vacuous scan, the second is a
    // question that cannot be asked. Defaulting to 0 and omitting it when 0
    // erased exactly that difference.
    let mut rules_evaluated: Option<u64> = None;
    for run in runs {
        let driver = run.get("tool").and_then(|tool| tool.get("driver"));
        if let Some(driver) = driver {
            if let Some(name) = driver.get("name").and_then(Value::as_str)
                && !name.is_empty()
            {
                tools.push(match driver.get("version").and_then(Value::as_str) {
                    Some(version) => format!("{name} {version}"),
                    None => name.to_owned(),
                });
            }
            if let Some(rules) = driver.get("rules").and_then(Value::as_array) {
                let count = u64::try_from(rules.len()).unwrap_or(u64::MAX);
                rules_evaluated = Some(rules_evaluated.unwrap_or(0).saturating_add(count));
            }
        }
        for result in run
            .get("results")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default()
        {
            let rule_id = result
                .get("ruleId")
                .and_then(Value::as_str)
                .or_else(|| result.get("rule").and_then(|rule| rule.get("id")?.as_str()));
            let Some(rule_id) = rule_id.filter(|id| !id.is_empty()) else {
                continue;
            };
            let location = result
                .get("locations")
                .and_then(Value::as_array)
                .and_then(|locations| locations.first())
                .and_then(|location| location.get("physicalLocation"));
            let mut finding = Finding::new(rule_id);
            finding.severity = result
                .get("level")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            finding.message = result
                .get("message")
                .and_then(|message| message.get("text"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            finding.path = location
                .and_then(|location| location.get("artifactLocation"))
                .and_then(|artifact| artifact.get("uri"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            finding.line = location
                .and_then(|location| location.get("region"))
                .and_then(|region| region.get("startLine"))
                .and_then(Value::as_u64);
            findings.push(finding);
        }
    }

    Ok(FindingResult {
        findings,
        tool: (!tools.is_empty()).then(|| tools.join(", ")),
        ruleset: None,
        rules_evaluated,
    })
}

/// Parse `cargo audit --json` output.
///
/// Kept as a bespoke adapter for one measured reason: **cargo-audit emits no
/// SARIF**, so the SARIF reader cannot subsume it. Its native output is what a
/// consumer's CI actually produces. It also models clean-versus-unrun natively
/// — `vulnerabilities.found: false` beside the `database` and `lockfile` it
/// consulted — which is the same shape this record type generalizes.
///
/// # Errors
///
/// Refuses input that is not JSON and input with no `vulnerabilities` object.
pub fn parse_cargo_audit(raw: &str) -> Result<FindingResult, EvidenceError> {
    let report: Value = serde_json::from_str(raw).map_err(|error| {
        EvidenceError::adapter("cargo-audit", format!("input is not JSON: {error}"))
    })?;
    let vulnerabilities = report.get("vulnerabilities").ok_or_else(|| {
        EvidenceError::adapter(
            "cargo-audit",
            "no `vulnerabilities` object — expected `cargo audit --json` output",
        )
    })?;

    let mut findings: Vec<Finding> = Vec::new();
    let mut push = |entry: &Value, severity: &str| {
        let Some(id) = entry
            .get("advisory")
            .and_then(|advisory| advisory.get("id"))
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
        else {
            return;
        };
        let mut finding = Finding::new(id);
        finding.severity = Some(severity.to_owned());
        finding.message = entry
            .get("advisory")
            .and_then(|advisory| advisory.get("title"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        finding.path = entry
            .get("package")
            .and_then(|package| package.get("name"))
            .and_then(Value::as_str)
            .map(|name| {
                match entry
                    .get("package")
                    .and_then(|package| package.get("version"))
                    .and_then(Value::as_str)
                {
                    Some(version) => format!("{name}@{version}"),
                    None => name.to_owned(),
                }
            });
        findings.push(finding);
    };

    for entry in vulnerabilities
        .get("list")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
    {
        push(entry, "vulnerability");
    }
    // Warnings keep their own kind (`unsound`, `unmaintained`, `yanked`) as the
    // severity string. Flattening them to one word would discard the
    // distinction the tool drew, and quoin normalizes no scanner's severities
    // (FR-034-CON-2).
    //
    // The KINDS are walked in the order the document declares them, which is
    // what `Object.entries` gives the retained reader and what decides the
    // order of the findings this adapter writes into the store. A
    // `serde_json::Map` is a `BTreeMap` — the `preserve_order` feature is off
    // workspace-wide because enabling it breaks the canonical-JSON comparison
    // the native fixture suite performs — so the order is recovered from the raw text
    // by `warning_kind_order` rather than taken from the parsed value. Without
    // this the record's `findings` array would be re-ordered for any report
    // whose warning kinds are not already in byte order, which NFR-025 forbids.
    if let Some(warnings) = report.get("warnings").and_then(Value::as_object) {
        for kind in warning_kind_order(raw, warnings) {
            let Some(entries) = warnings.get(&kind) else {
                continue;
            };
            for entry in entries.as_array().map(Vec::as_slice).unwrap_or_default() {
                push(entry, &kind);
            }
        }
    }

    Ok(FindingResult {
        findings,
        tool: Some("cargo-audit".to_owned()),
        ruleset: None,
        rules_evaluated: report
            .get("database")
            .and_then(|database| database.get("advisory-count"))
            .and_then(Value::as_u64),
    })
}

/// The `warnings` member names in the order the document declares them.
///
/// `serde`'s `MapAccess` yields entries in document order when deserializing
/// straight from the text, which is the one place that order still exists once
/// the document has become a `serde_json::Value`. Falls back to the parsed
/// value's own (byte-ordered) names if the shape is not a map of arrays, which
/// is the only case where nothing is lost by doing so.
fn warning_kind_order(raw: &str, parsed: &serde_json::Map<String, Value>) -> Vec<String> {
    struct KindOrder(Vec<String>);

    impl<'de> serde::Deserialize<'de> for KindOrder {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct Names;

            impl<'de> serde::de::Visitor<'de> for Names {
                type Value = KindOrder;

                fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.write_str("the cargo-audit `warnings` object")
                }

                fn visit_map<A: serde::de::MapAccess<'de>>(
                    self,
                    mut map: A,
                ) -> Result<Self::Value, A::Error> {
                    let mut names = Vec::new();
                    while let Some(name) = map.next_key::<String>()? {
                        map.next_value::<serde::de::IgnoredAny>()?;
                        names.push(name);
                    }
                    Ok(KindOrder(names))
                }
            }

            deserializer.deserialize_map(Names)
        }
    }

    #[derive(serde::Deserialize)]
    struct Document {
        #[serde(default)]
        warnings: Option<KindOrder>,
    }

    serde_json::from_str::<Document>(raw)
        .ok()
        .and_then(|document| document.warnings)
        .map_or_else(
            || parsed.keys().cloned().collect(),
            |KindOrder(names)| names,
        )
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{parse_cargo_audit, parse_sarif};

    /// A SARIF run carrying no result is a scan that HAPPENED, and a driver
    /// declaring `rules: []` reported ZERO rules rather than saying nothing.
    ///
    /// Trace: FR-034-AC-1, FR-034-AC-20
    /// Provenance: quoin#458
    #[test]
    fn tc_458_200_a_run_with_no_results_is_a_clean_scan_and_not_a_refusal() {
        let result = parse_sarif(
            r#"{"runs":[{"tool":{"driver":{"name":"semgrep","version":"1.2.3","rules":[]}},"results":[]}]}"#,
        )
        .unwrap();
        assert!(result.findings.is_empty());
        assert_eq!(result.tool.as_deref(), Some("semgrep 1.2.3"));
        // `rules: []` is a tool that reported ZERO rules, not one that said
        // nothing. The two are different facts and FR-034 turns on it.
        assert_eq!(result.rules_evaluated, Some(0));
    }

    /// A driver that names no ruleset leaves the count unset: the question
    /// cannot be asked, so the record says nothing rather than something wrong.
    ///
    /// Trace: FR-034-AC-20
    /// Provenance: quoin#458
    #[test]
    fn tc_458_201_rules_evaluated_stays_absent_until_a_driver_declares_rules() {
        let result =
            parse_sarif(r#"{"runs":[{"tool":{"driver":{"name":"x"}},"results":[]}]}"#).unwrap();
        assert_eq!(result.rules_evaluated, None);
        assert_eq!(result.tool.as_deref(), Some("x"));
    }

    /// Rule id, level, message and location are read; the nested `rule.id`
    /// form is accepted and a result naming no rule at all is skipped.
    ///
    /// Trace: FR-034-AC-3, FR-034-AC-4
    /// Provenance: quoin#458
    #[test]
    fn tc_458_202_reads_rule_id_severity_message_path_and_line() {
        let result = parse_sarif(
            r#"{"runs":[{"tool":{"driver":{"name":"t"}},"results":[
                {"ruleId":"r1","level":"error","message":{"text":"m"},
                 "locations":[{"physicalLocation":{"artifactLocation":{"uri":"a.ts"},"region":{"startLine":7}}}]},
                {"rule":{"id":"r2"}},
                {"level":"note"}
            ]}]}"#,
        )
        .unwrap();
        assert_eq!(result.findings.len(), 2);
        assert_eq!(result.findings[0].rule_id, "r1");
        assert_eq!(result.findings[0].severity.as_deref(), Some("error"));
        assert_eq!(result.findings[0].path.as_deref(), Some("a.ts"));
        assert_eq!(result.findings[0].line, Some(7));
        assert_eq!(result.findings[1].rule_id, "r2");
    }

    /// A log with no run proves nothing executed, and text that is not JSON is
    /// named as such rather than read as an empty scan.
    ///
    /// Trace: FR-034-AC-2, FR-034-AC-5
    /// Provenance: quoin#458
    #[test]
    fn tc_458_203_refuses_a_log_with_no_run_and_text_that_is_not_json() {
        assert_eq!(
            parse_sarif(r#"{"runs":[]}"#).unwrap_err().to_string(),
            "sarif: `runs` is empty — a log with no run proves no scan executed"
        );
        assert_eq!(
            parse_sarif("{}").unwrap_err().to_string(),
            "sarif: no `runs` array — expected a SARIF 2.1.0 log"
        );
        assert!(
            parse_sarif("{")
                .unwrap_err()
                .to_string()
                .contains("not JSON"),
            "a malformed document must be named as malformed"
        );
    }

    /// `unsound`, `unmaintained` and `yanked` are distinctions cargo-audit
    /// drew; collapsing them would discard what the tool produced.
    ///
    /// Trace: FR-034-AC-7, FR-034-CON-2
    /// Provenance: quoin#458
    #[test]
    fn tc_458_204_cargo_audit_keeps_each_warning_kind_as_its_own_severity() {
        let result = parse_cargo_audit(
            r#"{"database":{"advisory-count":700},
                "vulnerabilities":{"found":true,"list":[
                  {"advisory":{"id":"RUSTSEC-1","title":"t"},"package":{"name":"p","version":"1"}}]},
                "warnings":{"unmaintained":[{"advisory":{"id":"RUSTSEC-2"},"package":{"name":"q"}}],
                            "yanked":[{"advisory":{"id":"RUSTSEC-3"}}]}}"#,
        )
        .unwrap();
        assert_eq!(result.rules_evaluated, Some(700));
        assert_eq!(result.tool.as_deref(), Some("cargo-audit"));
        let severities: Vec<&str> = result
            .findings
            .iter()
            .map(|f| f.severity.as_deref().unwrap_or(""))
            .collect();
        assert_eq!(severities, ["vulnerability", "unmaintained", "yanked"]);
        assert_eq!(result.findings[0].path.as_deref(), Some("p@1"));
        assert_eq!(result.findings[1].path.as_deref(), Some("q"));
        assert_eq!(result.findings[2].path, None);
    }

    /// Output that is not cargo-audit's, and text that is not JSON at all.
    ///
    /// Trace: FR-034-AC-8
    /// Provenance: quoin#458
    #[test]
    fn tc_458_205_refuses_output_that_is_not_cargo_audits() {
        assert_eq!(
            parse_cargo_audit("{}").unwrap_err().to_string(),
            "cargo-audit: no `vulnerabilities` object — expected `cargo audit --json` output"
        );
        assert!(
            parse_cargo_audit("{")
                .unwrap_err()
                .to_string()
                .contains("not JSON")
        );
    }
}
