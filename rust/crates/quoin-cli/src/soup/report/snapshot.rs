// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The SOUP section of the release snapshot (FR-167).
//!
//! It is the same [`Report`] `quoin soup report` renders, read through the same
//! [`build`], so a row's `purl`, `version`, `support_status` and
//! `end_of_support` are the very values the report's JSON states. Two members
//! are derived here and nowhere else: a row's `group`, which is FR-144's
//! ordering group, and `not_assessed`, which lists what FR-144 appends after
//! its rows.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    DischargeFailure, Report, Row, build, discharge_report, read_distinct_records, read_support,
    render,
};
use crate::soup::Stop;
use crate::soup::advisory::ScanOutcome;
use crate::soup::policy::LicenceOutcome;
use crate::soup::support::{SupportFile, as_of_date};

/// Everything the SOUP section reads, already opened by the caller.
pub(crate) struct SoupInputs<'a> {
    /// The repository root.
    pub(crate) repo: &'a Path,
    /// The directory of `soup_record` files.
    pub(crate) records_dir: &'a Path,
    /// The `--as-of` instant, as supplied.
    pub(crate) as_of: &'a str,
    /// The parsed `clause-binding-v1` report.
    pub(crate) binding: &'a Value,
    /// The support-data file, when supplied.
    pub(crate) support: Option<&'a Path>,
}

/// The `soup` member: the rows and what FR-144 appends after them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SoupSection {
    /// One row per `soup_record`, in FR-144's order.
    pub(crate) rows: Vec<SoupRow>,
    /// The unmatched findings and the scans that taught nothing.
    pub(crate) not_assessed: Vec<NotAssessed>,
}

/// FR-144's ordering group of a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Group {
    /// An unresolved advisory or policy finding.
    UnresolvedAdvisory,
    /// Past end of support.
    PastEndOfSupport,
    /// Neither.
    Other,
}

/// One SOUP row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SoupRow {
    /// The component's purl.
    pub(crate) purl: String,
    /// The record's `version`, or `not stated`.
    pub(crate) version: String,
    /// The row's ordering group.
    pub(crate) group: Group,
    /// The report's `support_status` value, unchanged.
    pub(crate) support_status: Resolved,
    /// The report's `end_of_support` value, unchanged.
    pub(crate) end_of_support: Resolved,
}

/// Where a resolved support value came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Origin {
    /// No input stated it.
    #[serde(rename = "none")]
    None,
    /// The `soup_record`.
    #[serde(rename = "record")]
    Record,
    /// The `--support` file.
    #[serde(rename = "support file")]
    SupportFile,
}

/// A support value and what stated it, as the SOUP report's JSON states it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Resolved {
    /// The value, or `unknown`.
    pub(crate) value: String,
    /// What stated it.
    pub(crate) origin: Origin,
    /// The support file's source, when it supplied the value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) source: Option<String>,
    /// The support file's differing statement, when it disagrees with the record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) disagrees_with: Option<Disagreement>,
}

/// The support file's statement that differs from the record's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Disagreement {
    /// The support file's value.
    pub(crate) value: String,
    /// Always the support file.
    pub(crate) origin: Origin,
    /// The support file's source.
    pub(crate) source: String,
}

/// One item FR-144 appends after its rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NotAssessed {
    /// `unmatched_advisory`, `unmatched_policy_finding` or a `scan_` state.
    pub(crate) kind: String,
    /// The scanner.
    pub(crate) tool: String,
    /// Defined per `kind` (FR-167).
    pub(crate) detail: String,
}

/// Why the section could not be built.
pub(crate) enum SoupFailure {
    /// An input was refused; `input` is the flag that supplied it.
    Refused {
        input: &'static str,
        code: String,
        message: String,
    },
    /// An input exists but cannot be read.
    Unreadable {
        input: &'static str,
        message: String,
    },
    /// An operational failure.
    Failed(String),
}

/// A refusal the section's step stated, attributed to the flag that supplied
/// the input; an operational failure stays one.
fn refused(input: &'static str, stop: Stop) -> SoupFailure {
    match stop {
        Stop::Refuse(response) => {
            let (code, message) = response
                .diagnostics
                .into_iter()
                .next()
                .map(|diagnostic| (diagnostic.code, diagnostic.message))
                .unwrap_or_default();
            SoupFailure::Refused {
                input,
                code,
                message,
            }
        }
        Stop::Fail(message) => SoupFailure::Failed(message),
    }
}

/// The `soup` member of the snapshot.
pub(crate) fn soup_section(inputs: &SoupInputs<'_>) -> Result<SoupSection, SoupFailure> {
    let as_of_day = as_of_date(inputs.as_of).map_err(|error| SoupFailure::Refused {
        input: "--as-of",
        code: error.code.as_str().to_owned(),
        message: error.message.into_string(),
    })?;
    let support = match inputs.support {
        Some(path) => read_support(path).map_err(|stop| match stop {
            Stop::Fail(message) => SoupFailure::Unreadable {
                input: "--support",
                message,
            },
            refusal @ Stop::Refuse(_) => refused("--support", refusal),
        })?,
        None => SupportFile::default(),
    };
    let existing =
        read_distinct_records(inputs.records_dir).map_err(|stop| refused("--records", stop))?;
    let discharge = discharge_report(inputs.repo, inputs.binding, inputs.as_of).map_err(
        |failure| match failure {
            DischargeFailure::BadBinding(message) => SoupFailure::Refused {
                input: "--binding",
                code: "bad_request".to_owned(),
                message,
            },
            other => refused("--binding", Stop::from(other)),
        },
    )?;
    let report = build(
        inputs.repo,
        existing,
        inputs.as_of,
        as_of_day,
        &support,
        &discharge,
    )
    .map_err(|stop| refused("--records", stop))?;
    // A retained scan file that cannot be read is evidence that failed to load.
    // No scan state covers it, so the snapshot refuses rather than state an
    // older scan of the same tool as the evidence.
    if let Some(path) = report.unreadable_scans.first() {
        return Err(SoupFailure::Unreadable {
            input: "--repo",
            message: format!("the retained scan {path} cannot be read"),
        });
    }
    section(&report)
}

fn section(report: &Report) -> Result<SoupSection, SoupFailure> {
    Ok(SoupSection {
        rows: report.rows.iter().map(row).collect::<Result<Vec<_>, _>>()?,
        not_assessed: not_assessed(report),
    })
}

/// A member of the report's row JSON, read as the closed type.
fn member(full: &mut Value, name: &str) -> Result<Resolved, SoupFailure> {
    let value = full.get_mut(name).map(Value::take).unwrap_or_default();
    serde_json::from_value(value)
        .map_err(|error| SoupFailure::Failed(format!("the SOUP row's {name} is {error}")))
}

fn row(row: &Row) -> Result<SoupRow, SoupFailure> {
    let mut full = render::row_json(row);
    let group = match row.group() {
        0 => Group::UnresolvedAdvisory,
        1 => Group::PastEndOfSupport,
        _ => Group::Other,
    };
    Ok(SoupRow {
        purl: row.join.purl.clone(),
        version: row
            .version
            .clone()
            .unwrap_or_else(|| render::NOT_STATED.to_owned()),
        group,
        support_status: member(&mut full, "support_status")?,
        end_of_support: member(&mut full, "end_of_support")?,
    })
}

/// The `kind` and state word of a scan that taught nothing, or `None` for a
/// scan that was read.
fn scan_state(outcome: &ScanOutcome) -> Option<(&'static str, &'static str)> {
    match outcome {
        ScanOutcome::NotRun => Some(("scan_not_run", "not run")),
        ScanOutcome::Vacuous { .. } => Some(("scan_vacuous", "vacuous")),
        ScanOutcome::Unordered { .. } => Some(("scan_unordered", "unordered")),
        ScanOutcome::Read { .. } => None,
    }
}

fn licence_state(outcome: &LicenceOutcome) -> Option<(&'static str, &'static str)> {
    match outcome {
        LicenceOutcome::NotRun => Some(("scan_not_run", "not run")),
        LicenceOutcome::Vacuous { .. } => Some(("scan_vacuous", "vacuous")),
        LicenceOutcome::Unordered { .. } => Some(("scan_unordered", "unordered")),
        LicenceOutcome::Read { .. } => None,
    }
}

/// The purl a finding names, or empty when it names none.
///
/// A finding carries only a `path`. A path that is a purl is the purl; the
/// cargo scanners (`cargo-audit`, `cargo-deny`) name a crate as `name@version`,
/// which is the cargo purl without its prefix. Any other path (a file location
/// from a SARIF-shaped scan, say) is not a purl and must not reach the file:
/// it can be a host path (NFR-036).
fn purl_of(tool: &str, path: Option<&str>) -> String {
    let Some(path) = path else {
        return String::new();
    };
    if path.starts_with("pkg:") {
        return path.to_owned();
    }
    let crate_id = path.split_once('@').is_some_and(|(name, version)| {
        !name.is_empty()
            && !version.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
            && !version.contains(['/', '\\', ' '])
    });
    if matches!(tool, "cargo-audit" | "cargo-deny") && crate_id {
        format!("pkg:cargo/{path}")
    } else {
        String::new()
    }
}

/// The items after the rows, ordered by `kind`, `tool` and `detail` bytewise.
fn not_assessed(report: &Report) -> Vec<NotAssessed> {
    let mut items = Vec::new();
    let mut push = |kind: &str, tool: &str, detail: String| {
        items.push(NotAssessed {
            kind: kind.to_owned(),
            tool: tool.to_owned(),
            detail,
        });
    };
    for unmatched in &report.unmatched {
        push(
            "unmatched_advisory",
            unmatched.tool.name(),
            format!(
                "{}|{}",
                unmatched.rule_id,
                purl_of(unmatched.tool.name(), unmatched.package.as_deref())
            ),
        );
    }
    for unmatched in &report.unmatched_policy {
        push(
            "unmatched_policy_finding",
            unmatched.tool.name(),
            format!(
                "{}|{}",
                unmatched.rule_id,
                purl_of(unmatched.tool.name(), unmatched.package.as_deref())
            ),
        );
    }
    for state in &report.scans {
        if let Some((kind, word)) = scan_state(&state.outcome) {
            push(kind, state.tool.name(), word.to_owned());
        }
    }
    if let Some((kind, word)) = licence_state(&report.licence.outcome) {
        push(kind, "licence-policy", word.to_owned());
    }
    items.sort_by(|a, b| (&a.kind, &a.tool, &a.detail).cmp(&(&b.kind, &b.tool, &b.detail)));
    items
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use ix_trace_rs::trace;

    use super::purl_of;

    #[trace("FR-167-AC-4", "NFR-036-M-2")]
    #[test]
    fn a_finding_path_is_a_purl_only_when_it_names_one() {
        assert_eq!(
            purl_of("cargo-audit", Some("ghost@9.9.9")),
            "pkg:cargo/ghost@9.9.9"
        );
        assert_eq!(
            purl_of("licence-policy", Some("pkg:npm/fx-vale@1.0.0")),
            "pkg:npm/fx-vale@1.0.0"
        );
        assert_eq!(purl_of("cargo-deny", None), "");
        assert_eq!(purl_of("licence-policy", Some("ghost@9.9.9")), "");
        assert_eq!(
            purl_of("cargo-audit", Some("/home/user/src/Cargo.lock")),
            ""
        );
        assert_eq!(purl_of("cargo-audit", Some("src/lib.rs@1")), "");
    }
}
