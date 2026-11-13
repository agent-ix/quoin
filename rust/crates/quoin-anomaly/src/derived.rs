// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The values quoin derives from git for one anomaly: the fixing change
//! (FR-136) and the release-rationale change (FR-163).
//!
//! Neither is a record key and neither reads an author, a date or a commit
//! typed into the document; each is what git recorded.

use std::path::Path;

use serde::{Serialize, Serializer};

use crate::error::AnomalyError;
use crate::history::{Commit, History, Signature, StatusEntry, locate, pending_entry};
use crate::record::{Record, release_rationale_text};
use crate::status::Status;

/// The commit a derived value names, and what git reported about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    /// The commit hash.
    pub commit: String,
    /// The commit author's name.
    pub author: String,
    /// The author date, ISO 8601.
    pub authored_at: String,
    /// The reduced signature answer.
    pub signature: Signature,
    /// The letter git reported.
    pub signature_code: char,
}

impl Change {
    fn of_commit(commit: &Commit) -> Self {
        Self {
            commit: commit.hash.clone(),
            author: commit.author.clone(),
            authored_at: commit.authored_at.clone(),
            signature: Signature::from_code(commit.signature_code),
            signature_code: commit.signature_code,
        }
    }

    fn of_entry(entry: &StatusEntry) -> Option<Self> {
        Some(Self {
            commit: entry.commit.clone()?,
            author: entry.author.clone()?,
            authored_at: entry.authored_at.clone()?,
            signature: entry.signature?,
            signature_code: entry.signature_code?,
        })
    }
}

/// A git-derived value, in the four states the report encodes alike.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Derived {
    /// The qualifying commit exists.
    Present(Change),
    /// No qualifying change exists.
    Absent,
    /// The qualifying change exists only in the working tree.
    Pending,
    /// The file is not under git.
    NotApplicable,
}

impl Derived {
    /// The state word: `present`, `absent`, `pending` or `not-applicable`.
    #[must_use]
    pub fn state(&self) -> &'static str {
        match self {
            Self::Present(_) => "present",
            Self::Absent => "absent",
            Self::Pending => "pending",
            Self::NotApplicable => "not-applicable",
        }
    }

    /// The Markdown reading: `<commit> <author> <authoredAt> <signature>
    /// (<signatureCode>)`, `absent`, `absent (pending)` or `not applicable`.
    #[must_use]
    pub fn markdown(&self) -> String {
        match self {
            Self::Present(change) => format!(
                "{} {} {} {} ({})",
                change.commit,
                change.author,
                change.authored_at,
                signature_word(change.signature),
                change.signature_code,
            ),
            Self::Absent => "absent".to_owned(),
            Self::Pending => "absent (pending)".to_owned(),
            Self::NotApplicable => "not applicable".to_owned(),
        }
    }
}

fn signature_word(signature: Signature) -> &'static str {
    match signature {
        Signature::Signed => "signed",
        Signature::Unsigned => "unsigned",
        Signature::Unknown => "unknown",
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DerivedView<'a> {
    state: &'static str,
    commit: Option<&'a str>,
    author: Option<&'a str>,
    authored_at: Option<&'a str>,
    signature: Option<Signature>,
    signature_code: Option<String>,
}

impl Serialize for Derived {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let change = match self {
            Self::Present(change) => Some(change),
            Self::Absent | Self::Pending | Self::NotApplicable => None,
        };
        DerivedView {
            state: self.state(),
            commit: change.map(|c| c.commit.as_str()),
            author: change.map(|c| c.author.as_str()),
            authored_at: change.map(|c| c.authored_at.as_str()),
            signature: change.map(|c| c.signature),
            signature_code: change.map(|c| c.signature_code.to_string()),
        }
        .serialize(serializer)
    }
}

/// The fixing change (FR-136): the first `statusHistory` entry, after the most
/// recent `assessed` entry (or from the first entry when none has that
/// status), whose status is `resolved` or `closed`. Derived only where the
/// working-tree `resolution` is `fixed`.
#[must_use]
pub fn fixing_change(history: &History, record: &Record) -> Derived {
    let History::Entries { entries, .. } = history else {
        return Derived::NotApplicable;
    };
    if record.resolution.as_deref() != Some("fixed") {
        return Derived::Absent;
    }
    let mut all = entries.clone();
    if let Some(status) = record.status()
        && all.last().is_none_or(|last| last.status != status)
    {
        all.push(pending_entry(status));
    }
    let start = all
        .iter()
        .rposition(|entry| entry.status == Status::Assessed)
        .map_or(0, |index| index + 1);
    match all
        .iter()
        .skip(start)
        .find(|entry| matches!(entry.status, Status::Resolved | Status::Closed))
    {
        None => Derived::Absent,
        Some(entry) => Change::of_entry(entry).map_or(Derived::Pending, Derived::Present),
    }
}

/// The release-rationale change (FR-163): the most recent commit on the
/// first-parent history of the default branch, following renames, that changed
/// the text of the `## Release Rationale` section.
///
/// # Errors
///
/// Returns an error when `git` cannot be run or cannot answer a history query.
pub fn rationale_change(file: &Path, record: &Record) -> Result<Derived, AnomalyError> {
    let Some(located) = locate(file)? else {
        return Ok(Derived::NotApplicable);
    };
    let Some(working) = record.release_rationale_text.as_deref() else {
        return Ok(Derived::Absent);
    };
    let mut previous: Option<String> = None;
    let mut changed: Option<&Commit> = None;
    for commit in &located.commits {
        let text = located
            .content(commit)?
            .and_then(|content| release_rationale_text(&content));
        if text != previous {
            changed = Some(commit);
        }
        previous = text;
    }
    // `previous` is now the section text at the tip of the default branch.
    Ok(match changed {
        Some(commit) if previous.as_deref() == Some(working) => {
            Derived::Present(Change::of_commit(commit))
        }
        _ => Derived::Pending,
    })
}
