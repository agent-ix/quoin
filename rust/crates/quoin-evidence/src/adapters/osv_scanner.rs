// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! OSV-Scanner's JSON finding adapter (FR-149), kept pure over the supplied report text.

use std::collections::BTreeMap;

use serde_json::Value;

use super::FindingResult;
use crate::error::EvidenceError;
use crate::purl::PurlIdentity;
use crate::types::Finding;

/// Parse an OSV-Scanner JSON report into one finding per vulnerability per package.
///
/// The report does not identify its database or how many rules were evaluated;
/// both facts stay unset. A finding's path is the package purl where present,
/// otherwise its `name@version` identity.
///
/// # Errors
///
/// Refuses malformed JSON, missing or empty result/package collections,
/// vulnerabilities without ids, and vulnerable packages without an unambiguous
/// syntactically valid identity.
pub fn parse_osv_scanner(raw: &str) -> Result<FindingResult, EvidenceError> {
    let report: Value =
        serde_json::from_str(raw).map_err(|error| refuse(format!("input is not JSON: {error}")))?;
    if !report.is_object() {
        return Err(refuse("expected a JSON object report"));
    }
    let results = report
        .get("results")
        .and_then(Value::as_array)
        .filter(|results| !results.is_empty())
        .ok_or_else(|| refuse("missing or empty `results` array"))?;

    let mut findings = Vec::new();
    for (source_index, source) in results.iter().enumerate() {
        let source_label = format!("source {}", source_index.saturating_add(1));
        let packages = source
            .get("packages")
            .and_then(Value::as_array)
            .filter(|packages| !packages.is_empty())
            .ok_or_else(|| refuse(format!("{source_label} has no non-empty `packages` array")))?;
        for (package_index, item) in packages.iter().enumerate() {
            let label = format!(
                "{source_label}, package {}",
                package_index.saturating_add(1)
            );
            let item = item
                .as_object()
                .ok_or_else(|| refuse(format!("{label} is not a package result object")))?;
            let vulnerabilities = item
                .get("vulnerabilities")
                .and_then(Value::as_array)
                .ok_or_else(|| refuse(format!("{label} has no `vulnerabilities` array")))?;
            let group_severities = group_severities(item.get("groups"));
            if vulnerabilities.is_empty() {
                continue;
            }

            let package = item
                .get("package")
                .and_then(Value::as_object)
                .ok_or_else(|| {
                    refuse(format!(
                        "{label} vulnerability has no package object or package identity"
                    ))
                })?;
            let path = package_path(package).ok_or_else(|| {
                refuse(format!(
                    "{label} vulnerability has no valid package identity (purl or name and version)"
                ))
            })?;

            for (vulnerability_index, vulnerability) in vulnerabilities.iter().enumerate() {
                let vuln_label = format!(
                    "{label}, vulnerability {}",
                    vulnerability_index.saturating_add(1)
                );
                let id = vulnerability
                    .get("id")
                    .and_then(Value::as_str)
                    .filter(|id| !id.is_empty())
                    .ok_or_else(|| refuse(format!("{vuln_label} has no non-empty `id`")))?;
                let mut finding = Finding::new(id);
                finding.path = Some(path.clone());
                finding.severity = group_severities.get(id).cloned();
                finding.message = vulnerability
                    .get("summary")
                    .and_then(Value::as_str)
                    .filter(|summary| !summary.is_empty())
                    .map(ToOwned::to_owned)
                    .or_else(|| Some(id.to_owned()));
                findings.push(finding);
            }
        }
    }

    Ok(FindingResult {
        findings,
        tool: Some("osv-scanner".to_owned()),
        evaluated: None,
        ruleset: None,
        rules_evaluated: None,
    })
}

fn refuse(message: impl Into<String>) -> EvidenceError {
    EvidenceError::adapter("osv-scanner", message)
}

fn package_path(package: &serde_json::Map<String, Value>) -> Option<String> {
    if let Some(purl) = package.get("purl") {
        let purl = purl.as_str()?;
        return valid_purl(purl).then(|| purl.to_owned());
    }
    let name = package.get("name")?.as_str()?.trim();
    let version = package.get("version")?.as_str()?.trim();
    (!name.is_empty() && !version.is_empty()).then(|| format!("{name}@{version}"))
}

fn valid_purl(purl: &str) -> bool {
    let Some(identity) = PurlIdentity::parse(purl) else {
        return false;
    };
    if identity.kind.is_empty() || identity.name.is_empty() {
        return false;
    }
    purl.as_bytes().iter().enumerate().all(|(index, byte)| {
        *byte != b'%' || {
            let Some(hex) = purl
                .as_bytes()
                .get(index.saturating_add(1)..index.saturating_add(3))
            else {
                return false;
            };
            hex.len() == 2 && hex.iter().all(u8::is_ascii_hexdigit)
        }
    })
}

fn group_severities(groups: Option<&Value>) -> BTreeMap<String, String> {
    let mut severities = BTreeMap::new();
    let Some(groups) = groups.and_then(Value::as_array) else {
        return severities;
    };
    for group in groups {
        let Some(group) = group.as_object() else {
            continue;
        };
        let Some(severity) = group.get("max_severity").and_then(Value::as_str) else {
            continue;
        };
        let Some(ids) = group.get("ids").and_then(Value::as_array) else {
            continue;
        };
        for id in ids
            .iter()
            .filter_map(Value::as_str)
            .filter(|id| !id.is_empty())
        {
            severities
                .entry(id.to_owned())
                .or_insert_with(|| severity.to_owned());
        }
    }
    severities
}
