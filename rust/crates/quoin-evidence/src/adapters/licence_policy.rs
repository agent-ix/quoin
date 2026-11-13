// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! A person-authored `licence-policy-v1` file evaluated over the licences an
//! SBOM states → findings (FR-150).
//!
//! Pure: the evaluation is a function of two texts, the SBOM (read through the
//! FR-041 reader, [`super::read_components`]) and the policy. No registry,
//! licence database or package file is consulted, and no licence list ships
//! with quoin: which licences a project accepts is the person's to say
//! (FR-150-CON-2). Matching is verbatim and ASCII case-insensitive; no
//! expression is parsed.

use serde_json::{Map, Value};

use super::FindingResult;
use super::sbom::{DeclaredLicence, LicenceKind, SbomComponent, read_components};
use crate::error::EvidenceError;
use crate::purl::PurlIdentity;
use crate::types::Finding;

const NAME: &str = "licence-policy";
const FORMAT: &str = "licence-policy-v1";

/// One `ban` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Ban {
    purl: String,
    identity: PurlIdentity,
    reason: Option<String>,
}

/// A parsed policy.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Policy {
    allow: Vec<String>,
    deny: Vec<String>,
    ban: Vec<Ban>,
}

fn refuse(message: impl Into<String>) -> EvidenceError {
    EvidenceError::adapter(NAME, message)
}

/// Read a list of non-empty strings under `key`.
fn texts(object: &Map<String, Value>, key: &str) -> Result<Vec<String>, EvidenceError> {
    let Some(value) = object.get(key) else {
        return Ok(Vec::new());
    };
    value
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .map(|item| {
                    item.as_str()
                        .filter(|text| !text.is_empty())
                        .map(str::to_owned)
                })
                .collect()
        })
        .ok_or_else(|| {
            refuse(format!(
                "policy `{key}` must be a list of non-empty strings"
            ))
        })
}

fn parse_ban(entry: &Value) -> Result<Ban, EvidenceError> {
    let object = entry
        .as_object()
        .ok_or_else(|| refuse("policy `ban` entries must be objects with a `purl`"))?;
    if let Some(key) = object
        .keys()
        .find(|key| !matches!(key.as_str(), "purl" | "reason"))
    {
        return Err(refuse(format!(
            "policy `ban` entry has an unknown key `{key}`"
        )));
    }
    let purl = object
        .get("purl")
        .and_then(Value::as_str)
        .ok_or_else(|| refuse("policy `ban` entry has no `purl` string"))?;
    let identity = PurlIdentity::parse(purl).ok_or_else(|| {
        refuse(format!(
            "policy `ban` purl {purl:?} is not a parseable purl"
        ))
    })?;
    let reason = match object.get("reason") {
        None | Some(Value::Null) => None,
        Some(Value::String(reason)) => Some(reason.clone()),
        Some(_) => return Err(refuse("policy `ban` reason must be a string")),
    };
    Ok(Ban {
        purl: purl.to_owned(),
        identity,
        reason,
    })
}

fn parse_policy(text: &str) -> Result<Policy, EvidenceError> {
    let document = quoin_yaml::from_str(text)
        .map_err(|error| refuse(format!("policy is not valid YAML: {error}")))?;
    let object = document
        .as_object()
        .ok_or_else(|| refuse("policy must be a YAML mapping"))?;
    if let Some(key) = object
        .keys()
        .find(|key| !matches!(key.as_str(), "format" | "allow" | "deny" | "ban"))
    {
        return Err(refuse(format!("policy has an unknown key `{key}`")));
    }
    if object.get("format").and_then(Value::as_str) != Some(FORMAT) {
        return Err(refuse(format!("policy must state `format: {FORMAT}`")));
    }
    let allow = texts(object, "allow")?;
    if allow.is_empty() {
        return Err(refuse("policy `allow` is required and must not be empty"));
    }
    let ban = match object.get("ban") {
        None => Vec::new(),
        Some(Value::Array(entries)) => entries.iter().map(parse_ban).collect::<Result<_, _>>()?,
        Some(_) => return Err(refuse("policy `ban` must be a list")),
    };
    Ok(Policy {
        allow,
        deny: texts(object, "deny")?,
        ban,
    })
}

/// Parse the SBOM `sbom` and evaluate it against the policy `policy`.
///
/// # Errors
///
/// Refuses a policy that is not valid YAML, lacks `format: licence-policy-v1`,
/// has an absent or empty `allow`, an unknown key, or a `ban` entry with no
/// parseable purl; and an SBOM the FR-041 reader refuses. Nothing is recorded
/// from a refused input.
pub fn parse_licence_policy(sbom: &str, policy: &str) -> Result<FindingResult, EvidenceError> {
    let policy = parse_policy(policy)?;
    let components = read_components(sbom)?;
    Ok(evaluate(&components, &policy))
}

fn listed(list: &[String], text: &str) -> bool {
    list.iter().any(|entry| entry.eq_ignore_ascii_case(text))
}

/// The ruleset: the policy's own entries, as canonical JSON.
fn ruleset(policy: &Policy) -> String {
    let ban: Vec<Value> = policy
        .ban
        .iter()
        .map(|ban| match &ban.reason {
            Some(reason) => serde_json::json!({ "purl": ban.purl, "reason": reason }),
            None => serde_json::json!({ "purl": ban.purl }),
        })
        .collect();
    serde_json::json!({
        "format": FORMAT, "allow": policy.allow, "deny": policy.deny, "ban": ban,
    })
    .to_string()
}

fn finding(rule: &str, severity: &str, message: String, path: &str) -> Finding {
    let mut finding = Finding::new(rule);
    finding.severity = Some(severity.to_owned());
    finding.message = Some(message);
    finding.path = Some(path.to_owned());
    finding
}

/// At most one licence finding for a component: denied, else not allowed, else
/// unknown (FR-150 precedence).
fn licence_finding(component: &SbomComponent, policy: &Policy) -> Option<Finding> {
    let mut denied = Vec::new();
    let mut not_allowed = Vec::new();
    let mut unknown = Vec::new();
    for DeclaredLicence { kind, text } in &component.licences {
        if listed(&policy.deny, text) {
            denied.push(text.as_str());
        } else if !listed(&policy.allow, text) {
            match kind {
                LicenceKind::Id => not_allowed.push(text.as_str()),
                LicenceKind::Name | LicenceKind::Expression => unknown.push(text.as_str()),
            }
        }
    }
    let path = &component.symbol;
    if !denied.is_empty() {
        Some(finding("licence-denied", "denied", denied.join(", "), path))
    } else if !not_allowed.is_empty() {
        Some(finding(
            "licence-not-allowed",
            "not-allowed",
            not_allowed.join(", "),
            path,
        ))
    } else if !unknown.is_empty() {
        Some(finding(
            "licence-unknown",
            "unknown",
            unknown.join(", "),
            path,
        ))
    } else if component.licences.is_empty() {
        Some(finding(
            "licence-unknown",
            "unknown",
            "no licence declared".to_owned(),
            path,
        ))
    } else {
        None
    }
}

/// A component's purl matches an entry by identity; an entry without a version
/// matches every version.
fn banned_by<'a>(component: &SbomComponent, policy: &'a Policy) -> Vec<&'a Ban> {
    let Some(identity) = component.purl.as_deref().and_then(PurlIdentity::parse) else {
        return Vec::new();
    };
    policy
        .ban
        .iter()
        .filter(|ban| {
            ban.identity.same_package(&identity)
                && (ban.identity.version.is_none() || ban.identity.version == identity.version)
        })
        .collect()
}

fn evaluate(components: &[SbomComponent], policy: &Policy) -> FindingResult {
    let mut findings = Vec::new();
    for component in components {
        findings.extend(licence_finding(component, policy));
        for ban in banned_by(component, policy) {
            let message = match &ban.reason {
                Some(reason) => format!("banned by {}: {reason}", ban.purl),
                None => format!("banned by {}", ban.purl),
            };
            findings.push(finding(
                "package-banned",
                "banned",
                message,
                &component.symbol,
            ));
        }
    }
    FindingResult {
        findings,
        tool: Some(NAME.to_owned()),
        ruleset: Some(ruleset(policy)),
        rules_evaluated: Some(u64::try_from(components.len()).unwrap_or(u64::MAX)),
        evaluated: Some(components.iter().map(|c| c.symbol.clone()).collect()),
    }
}
