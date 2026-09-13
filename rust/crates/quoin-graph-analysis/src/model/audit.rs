// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The FR-032 audit envelope, and the canonical order its report is read in.
//!
//! # The envelope's two literals are not fields
//!
//! `format: "quoin-audit-envelope"` and `format_version: 1` are `z.literal`s
//! (`input.ts:74-75`). Nothing downstream reads them and nothing emits them,
//! so they are proven by [`AuditEnvelope::parse`] and then gone — the value's
//! existence is the proof. A `format: String` field here would be a value a
//! reader could still be wrong about.
//!
//! # The report is the auditor's, carried whole
//!
//! `auditFinding`, `unevaluatedCheck` and `auditReport` are zod
//! `.passthrough()` schemas (`input.ts:46-70`): the graph view validates the
//! join surface and preserves every other member unchanged, because those
//! members are copied verbatim into the change-impact report. The types are
//! `quoin-finding-types`', which carry their undeclared members in a flattened
//! map for exactly this reason.

use quoin_finding_types::AuditReport;
use quoin_store::json::order::cmp_utf16;
use serde::Serialize;
use serde_json::Value;

use crate::error::{GraphError, Result};
use crate::ids::{RepositoryId, Revision};
use crate::json as reader;
use crate::model::premises::{AcceptedPremises, export_premises};

/// The envelope's format literal (`input.ts:74`).
pub const AUDIT_FORMAT: &str = "quoin-audit-envelope";

/// The envelope's format-version literal (`input.ts:75`).
pub const AUDIT_FORMAT_VERSION: u32 = 1;

/// The immutable repository identity an audit was taken at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceIdentity {
    /// The repository.
    pub repository: RepositoryId,
    /// The revision: 40 lowercase hexadecimal digits.
    pub revision: Revision,
}

/// One FR-032 audit, with the identity it was taken under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEnvelope {
    /// Where and when the audit was taken.
    pub source: SourceIdentity,
    /// The exact identity copied from the export, not the caller's acceptance
    /// set (`input.ts:86`).
    pub export: AcceptedPremises,
    /// The FR-032 payload, canonically ordered and otherwise untouched.
    pub report: AuditReport,
}

/// The envelope writes back the document it was read from, the two literals
/// included.
///
/// They are not fields — nothing may hold a value for them that could be
/// wrong — so they are written from the constants that
/// [`AuditEnvelope::parse`] proved, in the retained member order
/// (`input.ts:82`).
impl Serialize for AuditEnvelope {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct as _;
        let mut envelope = serializer.serialize_struct("AuditEnvelope", 5)?;
        envelope.serialize_field("format", AUDIT_FORMAT)?;
        envelope.serialize_field("format_version", &AUDIT_FORMAT_VERSION)?;
        envelope.serialize_field("source", &self.source)?;
        envelope.serialize_field("export", &self.export)?;
        envelope.serialize_field("report", &self.report)?;
        envelope.end()
    }
}

impl AuditEnvelope {
    /// Read the strict envelope contract from one JSON document.
    ///
    /// # Errors
    ///
    /// One line naming the member that failed and why.
    pub fn parse(value: &Value) -> std::result::Result<Self, String> {
        let root = reader::object(value, "<root>")?;
        reader::strict(
            root,
            "",
            &["format", "format_version", "source", "export", "report"],
        )?;
        reader::literal_text(reader::member(root, "", "format")?, "format", AUDIT_FORMAT)?;
        reader::literal_number(
            reader::member(root, "", "format_version")?,
            "format_version",
            u64::from(AUDIT_FORMAT_VERSION),
        )?;
        let source = parse_source(reader::member(root, "", "source")?)?;
        let export = AcceptedPremises::parse(reader::member(root, "", "export")?)
            .map_err(|reason| format!("export.{reason}"))?;
        let report = parse_report(reader::member(root, "", "report")?)?;
        let report = canonicalize_report(report).map_err(|error| error.to_string())?;
        Ok(Self {
            source,
            export,
            report,
        })
    }
}

fn parse_source(value: &Value) -> std::result::Result<SourceIdentity, String> {
    let object = reader::object(value, "source")?;
    reader::strict(object, "source.", &["repository", "revision"])?;
    Ok(SourceIdentity {
        repository: RepositoryId::parse(reader::text(
            reader::member(object, "source.", "repository")?,
            "source.repository",
        )?)
        .map_err(|reason| format!("source.repository: {reason}"))?,
        revision: Revision::parse(reader::text(
            reader::member(object, "source.", "revision")?,
            "source.revision",
        )?)
        .map_err(|reason| format!("source.revision: {reason}"))?,
    })
}

fn parse_report(value: &Value) -> std::result::Result<AuditReport, String> {
    let object = reader::object(value, "report")?;
    // `.passthrough()`: nothing is refused for being undeclared, and the three
    // declared collections are checked member by member below.
    for (index, finding) in reader::array(
        reader::member(object, "report.", "findings")?,
        "report.findings",
    )?
    .iter()
    .enumerate()
    {
        check_finding(finding, &format!("report.findings.{index}"))?;
    }
    for (index, healthy) in reader::array(
        reader::member(object, "report.", "healthy")?,
        "report.healthy",
    )?
    .iter()
    .enumerate()
    {
        let at = format!("report.healthy.{index}");
        if reader::text(healthy, &at)?.is_empty() {
            return Err(format!("{at}: expected a non-empty string"));
        }
    }
    for (index, check) in reader::array(
        reader::member(object, "report.", "unevaluated")?,
        "report.unevaluated",
    )?
    .iter()
    .enumerate()
    {
        check_unevaluated(check, &format!("report.unevaluated.{index}"))?;
    }
    serde_json::from_value(value.clone()).map_err(|error| format!("report: {error}"))
}

fn check_finding(value: &Value, at: &str) -> std::result::Result<(), String> {
    let object = reader::object(value, at)?;
    non_empty(object, at, "obligation")?;
    non_empty(object, at, "kind")?;
    reader::text(
        reader::member(object, &format!("{at}."), "summary")?,
        &format!("{at}.summary"),
    )?;
    let severity = reader::text(
        reader::member(object, &format!("{at}."), "severity")?,
        &format!("{at}.severity"),
    )?;
    if !["low", "medium", "high"].contains(&severity) {
        return Err(format!(
            "{at}.severity: expected one of \"low\", \"medium\", \"high\""
        ));
    }
    Ok(())
}

fn check_unevaluated(value: &Value, at: &str) -> std::result::Result<(), String> {
    let object = reader::object(value, at)?;
    non_empty(object, at, "check")?;
    non_empty(object, at, "obligation")?;
    reader::text(
        reader::member(object, &format!("{at}."), "reason")?,
        &format!("{at}.reason"),
    )?;
    for (index, suite) in reader::array(
        reader::member(object, &format!("{at}."), "suites")?,
        &format!("{at}.suites"),
    )?
    .iter()
    .enumerate()
    {
        reader::text(suite, &format!("{at}.suites.{index}"))?;
    }
    Ok(())
}

fn non_empty(
    object: &serde_json::Map<String, Value>,
    at: &str,
    name: &str,
) -> std::result::Result<(), String> {
    let path = format!("{at}.{name}");
    if reader::text(reader::member(object, &format!("{at}."), name)?, &path)?.is_empty() {
        return Err(format!("{path}: expected a non-empty string"));
    }
    Ok(())
}

/// `canonicalizeAuditReport` (`input.ts:202`).
///
/// Findings and unevaluated checks are ordered by the canonical text of the
/// whole entry — `stableKey` — and each check's suites are ordered before that
/// key is taken, exactly as the retained `.map()` runs before its `.sort()`.
///
/// # Errors
///
/// [`crate::GraphError::Canonicalization`] when an entry has no canonical
/// spelling.
pub fn canonicalize_report(mut report: AuditReport) -> Result<AuditReport> {
    for check in &mut report.unevaluated {
        check.suites.sort_by(|left, right| cmp_utf16(left, right));
    }
    report.healthy.sort_by(|left, right| cmp_utf16(left, right));
    sort_by_stable_key(&mut report.findings)?;
    sort_by_stable_key(&mut report.unevaluated)?;
    Ok(report)
}

/// Order one collection by each entry's canonical text.
///
/// The keys are computed once per entry rather than inside the comparator:
/// `Array.prototype.sort` recomputes `stableKey` on every comparison, and that
/// is an implementation detail rather than an observable one.
fn sort_by_stable_key<T: Serialize>(entries: &mut Vec<T>) -> Result<()> {
    let taken = std::mem::take(entries);
    let mut keyed: Vec<(String, T)> = Vec::with_capacity(taken.len());
    for entry in taken {
        let value = serde_json::to_value(&entry).map_err(|error| GraphError::Canonicalization {
            what: "an audit report entry",
            detail: error.to_string(),
        })?;
        keyed.push((crate::json::stable_key(&value)?, entry));
    }
    keyed.sort_by(|(left, _), (right, _)| cmp_utf16(left, right));
    entries.extend(keyed.into_iter().map(|(_, entry)| entry));
    Ok(())
}

/// `validateAcceptedAssurancePremises` (`input.ts:131`).
///
/// # Errors
///
/// The retained sentence, when the two identities are not the same document.
pub fn validate_accepted_premises(
    export: &quoin_quire::model::AssuranceExport,
    accepted: &AcceptedPremises,
) -> Result<std::result::Result<(), String>> {
    let actual = crate::json::stable_key(&export_premises(export))?;
    let expected = crate::json::stable_key(&serde_json::to_value(accepted).map_err(|error| {
        GraphError::Canonicalization {
            what: "the accepted premises",
            detail: error.to_string(),
        }
    })?)?;
    Ok(if actual == expected {
        Ok(())
    } else {
        Err(
            "the accepted format, version, modules, and schema digests must exactly match the \
             assurance export"
                .to_owned(),
        )
    })
}

/// `validateAuditIdentity` (`input.ts:149`).
///
/// # Errors
///
/// The retained sentence for whichever half disagrees, source first.
pub fn validate_audit_identity(
    audit: &AuditEnvelope,
    export: &quoin_quire::model::AssuranceExport,
) -> Result<std::result::Result<(), String>> {
    if audit.source.repository.as_str() != export.source.repository
        || audit.source.revision.as_str() != export.source.revision
    {
        return Ok(Err(
            "the audit source repository/revision does not match the assurance export".to_owned(),
        ));
    }
    match validate_accepted_premises(export, &audit.export)? {
        Ok(()) => Ok(Ok(())),
        Err(_) => Ok(Err(
            "the audit export format/module premises do not match the assurance export".to_owned(),
        )),
    }
}
