// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The release-history file, taken as data (FR-168).
//!
//! The file belongs to Engineering Assurance
//! (`engineering-assurance.release-history/v1`). Quoin reads it by the members
//! it names and no others, copies the entries as given and orders them. It
//! reads no record file, fetches no commit and resolves no hash.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The protocol the history file must state.
const PROTOCOL: &str = "engineering-assurance.release-history/v1";

/// The previous release, as the history file states it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PreviousRelease {
    /// The previous release's tag name.
    pub(crate) tag: String,
    /// The release commit the tag peels to.
    pub(crate) commit: String,
}

/// One change-assurance record, as the history file lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChangeRecord {
    /// The record's id.
    pub(crate) record_id: String,
    /// The record's revision.
    pub(crate) revision: u64,
    /// The record file's repository-relative path.
    pub(crate) path: String,
    /// The commit that added the file.
    pub(crate) commit: String,
}

/// The members of the history file Quoin reads.
#[derive(Debug)]
pub(crate) struct History {
    /// The baseline commit the file was computed for.
    pub(crate) baseline_commit: String,
    /// The previous release, or `None` for a first release.
    pub(crate) previous_release: Option<PreviousRelease>,
    /// The records, ordered by `record_id`, `revision` then `commit`.
    pub(crate) records: Vec<ChangeRecord>,
}

/// Why the file is not a release-history file; `member` names what is wrong.
#[derive(Debug)]
pub(crate) struct Invalid {
    /// The protocol, member or entry pair at fault.
    pub(crate) member: String,
    /// A sentence for an operator.
    pub(crate) message: String,
}

fn invalid(member: impl Into<String>, message: impl Into<String>) -> Invalid {
    Invalid {
        member: member.into(),
        message: message.into(),
    }
}

fn text(value: &Value, member: &str, path: &str) -> Result<String, Invalid> {
    match value.get(member) {
        Some(Value::String(text)) => Ok(text.clone()),
        Some(_) => Err(invalid(
            format!("{path}{member}"),
            format!("{path}{member} is not a string"),
        )),
        None => Err(invalid(
            format!("{path}{member}"),
            format!("the history file lacks {path}{member}"),
        )),
    }
}

fn previous_release(value: &Value) -> Result<Option<PreviousRelease>, Invalid> {
    match value.get("previous_release") {
        None => Err(invalid(
            "previous_release",
            "the history file lacks previous_release",
        )),
        Some(Value::Null) => Ok(None),
        Some(previous @ Value::Object(_)) => Ok(Some(PreviousRelease {
            tag: text(previous, "tag", "previous_release.")?,
            commit: text(previous, "commit", "previous_release.")?,
        })),
        Some(_) => Err(invalid(
            "previous_release",
            "previous_release is neither null nor an object",
        )),
    }
}

fn record(entry: &Value, index: usize) -> Result<ChangeRecord, Invalid> {
    let path = format!("records[{index}].");
    let revision = match entry.get("revision") {
        Some(number) => number.as_u64().ok_or_else(|| {
            invalid(
                format!("{path}revision"),
                format!("{path}revision is not a non-negative integer"),
            )
        })?,
        None => {
            return Err(invalid(
                format!("{path}revision"),
                format!("the history file lacks {path}revision"),
            ));
        }
    };
    Ok(ChangeRecord {
        record_id: text(entry, "record_id", &path)?,
        revision,
        path: text(entry, "path", &path)?,
        commit: text(entry, "commit", &path)?,
    })
}

/// Read the named members of a history file's text.
///
/// # Errors
///
/// Returns the protocol, member or entry pair that makes the file invalid.
pub(crate) fn parse(source: &str) -> Result<History, Invalid> {
    let document: Value = serde_json::from_str(source)
        .map_err(|error| invalid("document", format!("the history file is not JSON: {error}")))?;
    let protocol = text(&document, "protocol", "")?;
    if protocol != PROTOCOL {
        return Err(invalid(
            format!("protocol {protocol}"),
            format!("the history file's protocol is {protocol}, not {PROTOCOL}"),
        ));
    }
    let baseline_commit = text(&document, "baseline_commit", "")?;
    let previous_release = previous_release(&document)?;
    let entries = match document.get("records") {
        Some(Value::Array(entries)) => entries,
        Some(_) => return Err(invalid("records", "records is not a list")),
        None => return Err(invalid("records", "the history file lacks records")),
    };
    let mut records = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| record(entry, index))
        .collect::<Result<Vec<_>, _>>()?;
    records.sort_by(|a, b| {
        (&a.record_id, a.revision, &a.commit).cmp(&(&b.record_id, b.revision, &b.commit))
    });
    let duplicate = records.windows(2).find_map(|pair| match pair {
        [a, b] if a.record_id == b.record_id && a.revision == b.revision => Some((a, b)),
        _ => None,
    });
    if let Some((first, second)) = duplicate {
        return Err(invalid(
            format!(
                "{} revision {} at commits {} and {}",
                first.record_id, first.revision, first.commit, second.commit
            ),
            format!(
                "two entries carry record_id {} and revision {}",
                first.record_id, first.revision
            ),
        ));
    }
    Ok(History {
        baseline_commit,
        previous_release,
        records,
    })
}
