// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin soup report` (FR-144): one row per `soup_record`, the advisories
//! attached to it (FR-142) and its support (FR-143).
//!
//! The report is a view. It reads the records below `--records`, the retained
//! scans, the stored discharge facts (through the `--binding` report) and the
//! optional `--support` file, and it writes nothing. A value no input stated
//! is `unknown` or `not stated`; nothing is scored or defaulted (NFR-030).

mod render;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use clap::ArgMatches;
use quoin_assurance::DischargeReport;
use quoin_core::protocol::Response;
use quoin_evidence::DiskEvidence;
use quoin_evidence::store::latest_scans;
use serde_json::{Map, Value};

use super::advisory::{self, RecordJoin, ScanState, Unmatched};
use super::plan;
use super::support::{
    Evaluation, RecordSupport, SupportError, SupportFile, as_of_date, parse_support, record_support,
};
use super::{Stop, arg, read_all_records, refuse};
use crate::core_runtime::invoke;

/// A support file larger than this is not read.
const MAX_SUPPORT_BYTES: u64 = 4 * 1024 * 1024;

/// A list a record may leave out (`Absent`), state empty (`Empty`) or fill.
#[derive(Debug, PartialEq, Eq)]
enum Listed<T> {
    Absent,
    Empty,
    Items(Vec<T>),
}

/// One reviewer-stated anomaly.
#[derive(Debug, PartialEq, Eq)]
struct Anomaly {
    statement: String,
    source_ref: Option<String>,
}

/// One row of the report.
struct Row {
    join: RecordJoin,
    version: Option<String>,
    serves: Listed<String>,
    anomalies: Listed<Anomaly>,
    support: Evaluation,
}

impl Row {
    fn unresolved(&self) -> bool {
        self.join
            .advisories
            .iter()
            .any(|a| matches!(a.resolution, advisory::Resolution::Unresolved(_)))
    }

    /// 0: an unresolved advisory; 1: past end of support; 2: the rest.
    fn group(&self) -> u8 {
        if self.unresolved() {
            0
        } else if self.support.past_end_of_support {
            1
        } else {
            2
        }
    }
}

/// Everything the renderers print.
struct Report {
    as_of: String,
    rows: Vec<Row>,
    scans: Vec<ScanState>,
    unmatched: Vec<Unmatched>,
    /// Retained scan files that could not be read, which no scan state covers.
    unreadable_scans: Vec<String>,
}

pub(super) fn run(arguments: &ArgMatches) -> Result<Response, Stop> {
    let repo = PathBuf::from(arg(arguments, "repo").unwrap_or("."));
    let records_dir =
        arg(arguments, "records").map_or_else(|| repo.join("spec/soup"), PathBuf::from);
    let as_of = arg(arguments, "as-of").ok_or_else(|| "--as-of is required".to_owned())?;
    let binding_path =
        arg(arguments, "binding").ok_or_else(|| "--binding is required".to_owned())?;

    if !repo.exists() {
        return Err(refuse(
            "soup.path_missing",
            format!(
                "--repo {} does not exist (no such file or directory)",
                repo.display()
            ),
            &[("path", repo.display().to_string())],
        ));
    }
    let as_of_day = as_of_date(as_of).map_err(support_refusal)?;
    let support = match arg(arguments, "support") {
        Some(path) => read_support(Path::new(path))?,
        None => SupportFile::default(),
    };

    let existing = read_all_records(&records_dir)?;
    if let Some((first, second)) = plan::duplicate_records(&existing) {
        return Err(refuse(
            "soup.duplicate_records",
            format!(
                "two records carry the purl {}: {} and {}",
                first.purl,
                first.path.display(),
                second.path.display()
            ),
            &[
                ("first", first.path.display().to_string()),
                ("second", second.path.display().to_string()),
            ],
        ));
    }

    let discharge = discharge_report(&repo, binding_path, as_of)?;
    let mut unreadable_scans = Vec::new();
    let scans = latest_scans(&DiskEvidence::new(&repo), &mut unreadable_scans)
        .map_err(|error| format!("cannot read the retained scans: {error}"))?;
    let joined = advisory::join(&existing, &scans, &discharge);
    let mut fronts: BTreeMap<PathBuf, Map<String, Value>> = existing
        .into_iter()
        .map(|record| (record.path, record.front))
        .collect();

    let mut rows = Vec::with_capacity(joined.records.len());
    for record in joined.records {
        let front = fronts.remove(&record.path).unwrap_or_default();
        rows.push(row(record, &front, &support, as_of_day)?);
    }
    rows.sort_by(|a, b| {
        (a.group(), &a.join.purl, &a.join.path).cmp(&(b.group(), &b.join.purl, &b.join.path))
    });
    unreadable_scans.sort();
    let report = Report {
        as_of: as_of.to_owned(),
        rows,
        scans: joined.scans,
        unmatched: joined.unmatched,
        unreadable_scans,
    };
    let rendered = if arguments.get_flag("json") {
        render::json(&report)?
    } else {
        render::markdown(&report)
    };
    Ok(Response::ok(serde_json::json!({ "rendered": rendered })))
}

/// The discharge report of the `--binding` clause set over the stored facts.
fn discharge_report(repo: &Path, binding: &str, as_of: &str) -> Result<DischargeReport, Stop> {
    let bytes =
        std::fs::read(binding).map_err(|error| format!("cannot read {binding}: {error}"))?;
    let binding: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("clause binding is not JSON: {error}"))?;
    // The discharge report is evaluated at an instant; a `--as-of` date is
    // the start of that UTC day.
    let instant = if as_of.contains('T') {
        as_of.to_owned()
    } else {
        format!("{as_of}T00:00:00Z")
    };
    let response = invoke(
        "evidence.discharge_report",
        &serde_json::json!({ "repo": repo, "binding": binding, "asOf": instant }),
    )?;
    if !response.outcome.carries_payload() {
        return Err(Stop::Refuse(Box::new(response)));
    }
    serde_json::from_value(response.payload)
        .map_err(|error| Stop::Fail(format!("evidence.discharge_report returned {error}")))
}

fn read_support(path: &Path) -> Result<SupportFile, Stop> {
    let length = std::fs::metadata(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?
        .len();
    if length > MAX_SUPPORT_BYTES {
        return Err(refuse(
            "soup.support_too_large",
            format!("{} is larger than 4 MiB", path.display()),
            &[("path", path.display().to_string())],
        ));
    }
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    parse_support(&text).map_err(support_refusal)
}

fn support_refusal(error: SupportError) -> Stop {
    let context: Vec<(&str, String)> = error
        .context
        .iter()
        .map(|(key, value)| (key.as_str(), value.clone()))
        .collect();
    refuse(error.code.as_str(), error.message.into_string(), &context)
}

/// The row of one joined record, from the frontmatter read with it.
fn row(
    join: RecordJoin,
    front: &Map<String, Value>,
    support: &SupportFile,
    as_of: time::Date,
) -> Result<Row, Stop> {
    let own: RecordSupport = record_support(front).map_err(|error| {
        refuse(
            error.code.as_str(),
            format!("{}: {}", join.path.display(), error.message),
            &[
                ("path", join.path.display().to_string()),
                ("purl", join.purl.clone()),
            ],
        )
    })?;
    let serves = list(front, "serves", &join, |item| {
        item.as_str().map(str::to_owned)
    })?;
    let anomalies = list(front, "known_anomalies", &join, |item| {
        let object = item.as_object()?;
        let source_ref = match object.get("source_ref") {
            None | Some(Value::Null) => None,
            Some(value) => Some(value.as_str()?.to_owned()),
        };
        Some(Anomaly {
            statement: object.get("statement")?.as_str()?.to_owned(),
            source_ref,
        })
    })?;
    let version = match front.get("version") {
        None | Some(Value::Null) => None,
        Some(Value::String(version)) => Some(version.clone()),
        Some(_) => {
            return Err(refuse(
                "soup.record_field_invalid",
                format!(
                    "{} states `version` in a shape the soup_record type does not allow",
                    join.path.display()
                ),
                &[
                    ("path", join.path.display().to_string()),
                    ("field", "version".to_owned()),
                ],
            ));
        }
    };
    Ok(Row {
        version,
        serves,
        anomalies,
        support: support.evaluate(&join.purl, own, as_of),
        join,
    })
}

/// A list field of a record. A field that is not a list of the expected shape
/// is a refusal naming the record, never read as absent or empty.
fn list<T>(
    front: &Map<String, Value>,
    key: &str,
    join: &RecordJoin,
    item: impl Fn(&Value) -> Option<T>,
) -> Result<Listed<T>, Stop> {
    let Some(value) = front.get(key) else {
        return Ok(Listed::Absent);
    };
    let items: Option<Vec<T>> = value
        .as_array()
        .and_then(|values| values.iter().map(&item).collect());
    match items {
        Some(items) if items.is_empty() => Ok(Listed::Empty),
        Some(items) => Ok(Listed::Items(items)),
        None => Err(refuse(
            "soup.record_field_invalid",
            format!(
                "{} states `{key}` in a shape the soup_record type does not allow",
                join.path.display()
            ),
            &[
                ("path", join.path.display().to_string()),
                ("field", key.to_owned()),
            ],
        )),
    }
}
