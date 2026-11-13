// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! TEMPORARY STAND-IN for the quire-rs record extractor (agent-ix/quire-rs#393).
//!
//! FR-136 says Quoin repeats none of the module's record mapping. That
//! extractor does not exist yet, so this reader repeats the part of
//! spec-artifacts-process's `mappings.yaml` that the checks here need. Remove
//! this module's table and frontmatter reading, and read the extractor's
//! `title`, `status`, `resolution`, `closedBy`, `severity`, `safetyImpact`,
//! `assessment`, `releaseRationale` and `fixVerification` keys instead, when #393 lands. The exception is recorded in FR-136.
//!
//! Vocabulary checks (an unknown `Kind` or `Impact`, an unknown `status`) are
//! `quire validate`'s under spec-artifacts-process FR-014/FR-015 and are not
//! made here: such a document still yields its record.
//!
//! The parts of an `Anomaly` document this crate reads: `title`, `status`,
//! `resolution`, `closed_by`, `safety_impact`, `severity`, the
//! `## Safety Impact Assessment` table, the `## Release Rationale` table
//! (the report of FR-162 and FR-163 widens the reader to `severity`, `title`
//! and `releaseRationale`) and the `## Fix Verification` table (FR-175 widens
//! it to the rows' `Kind`, `Target` and `Rationale`).
//!
//! Field vocabularies and the legality of a field combination are the module's
//! (spec-artifacts-process FR-014/FR-015) and `quire validate`'s. A document
//! that fails only those checks still yields its record; only a document the
//! mapping cannot read (no status, a table with the wrong columns) yields none.

use serde::Serialize;
use serde_json::Value;

use crate::status::Status;

const COLUMNS: [&str; 4] = ["Kind", "Target", "Impact", "Rationale"];
const SECTION: &str = "## Safety Impact Assessment";
const RATIONALE_COLUMNS: [&str; 2] = ["Author", "Rationale"];
const RATIONALE_SECTION: &str = "## Release Rationale";
const VERIFICATION_COLUMNS: [&str; 3] = ["Kind", "Target", "Rationale"];
const VERIFICATION_SECTION: &str = "## Fix Verification";

/// One assessment row, as authored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// `Requirement`, `Hazard` or `None`.
    pub kind: String,
    /// The authored reference token; absent for a row whose `Target` is `-`.
    pub target: Option<String>,
    /// How the anomaly bears on the target.
    pub impact: String,
    /// The engineer's judgement, verbatim.
    pub rationale: String,
    /// 1-based document line of the row.
    pub line: usize,
}

/// One `## Release Rationale` row, as authored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RationaleEntry {
    /// The person the rationale names, verbatim.
    pub author: String,
    /// The written reason, verbatim.
    pub rationale: String,
}

/// One `## Fix Verification` row, as authored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationRow {
    /// `Criterion` or `Test` (or a word outside the vocabulary, which is
    /// `quire validate`'s to report).
    pub kind: String,
    /// The authored reference token; absent for a row whose `Target` is empty
    /// or `-`.
    pub target: Option<String>,
    /// What the row says demonstrates the fix, verbatim.
    pub rationale: String,
    /// 1-based document line of the row.
    pub line: usize,
}

/// The record of one `Anomaly` document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The document id.
    pub id: String,
    /// The authored `title`, when present (FR-162 Row: absent reads as `null`).
    pub title: Option<String>,
    /// The authored `status` word, verbatim; see [`Record::status`].
    pub status_word: String,
    /// 1-based document line of the frontmatter `status` field.
    pub status_line: usize,
    /// The authored `resolution`, when present.
    pub resolution: Option<String>,
    /// The authored `closed_by`, when present.
    pub closed_by: Option<String>,
    /// The authored `safety_impact`.
    pub safety_impact: String,
    /// The authored `severity`, when present; never defaulted or derived.
    pub severity: Option<String>,
    /// The assessment rows; `None` while the section is absent.
    pub assessment: Option<Vec<Row>>,
    /// The text of the assessment section, trimmed; `None` while it is absent.
    pub assessment_text: Option<String>,
    /// The release-rationale rows; `None` while the section is absent.
    pub release_rationale: Option<Vec<RationaleEntry>>,
    /// The text of the release-rationale section, trimmed; `None` while it is
    /// absent.
    pub release_rationale_text: Option<String>,
    /// The fix-verification rows in document order; `None` while the section
    /// is absent.
    pub fix_verification: Option<Vec<VerificationRow>>,
}

impl Record {
    /// The status, when the authored word is in the vocabulary.
    #[must_use]
    pub fn status(&self) -> Option<Status> {
        Status::parse(&self.status_word)
    }
}

/// The mapping error codes that stop a record being built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingErrorCode {
    /// A required frontmatter key is absent.
    Missing,
    /// The assessment table's header is not `Kind | Target | Impact | Rationale`.
    Columns,
    /// A row has the wrong number of cells.
    RowArity,
    /// The assessment section holds no table rows.
    MinRows,
}

impl MappingErrorCode {
    /// The code as printed.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Columns => "columns",
            Self::RowArity => "row-arity",
            Self::MinRows => "min-rows",
        }
    }
}

/// Why a document yields no record: the mapping's error code and its line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappingError {
    /// The mapping error code.
    pub code: MappingErrorCode,
    /// 1-based document line the error is reported at.
    pub line: usize,
}

fn error(code: MappingErrorCode, line: usize) -> MappingError {
    MappingError { code, line }
}

/// The frontmatter YAML and the number of document lines it occupies.
fn split_frontmatter(text: &str) -> Option<(String, usize)> {
    let mut lines = text.lines();
    if lines.next()? != "---" {
        return None;
    }
    let mut yaml = Vec::new();
    for (index, line) in lines.enumerate() {
        if line == "---" {
            return Some((yaml.join("\n"), index + 2));
        }
        yaml.push(line);
    }
    None
}

/// The frontmatter of any document as a JSON value, when it has one.
#[must_use]
pub fn frontmatter(text: &str) -> Option<Value> {
    let (yaml, _) = split_frontmatter(text)?;
    quoin_yaml::from_str(&yaml).ok()
}

fn scalar(value: &Value, key: &str) -> Option<String> {
    match value.get(key)? {
        Value::String(text) => Some(text.clone()),
        Value::Null | Value::Array(_) | Value::Object(_) => None,
        other => Some(other.to_string()),
    }
}

/// Split one table row on its unescaped pipes, trimming each cell.
pub(crate) fn cells(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    let mut out = Vec::new();
    let mut current = String::new();
    let mut chars = inner.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' && chars.peek() == Some(&'|') {
            current.push('|');
            chars.next();
        } else if ch == '|' {
            out.push(current.trim().to_owned());
            current.clear();
        } else {
            current.push(ch);
        }
    }
    out.push(current.trim().to_owned());
    out
}

fn is_separator(row: &[String]) -> bool {
    !row.is_empty()
        && row
            .iter()
            .all(|cell| !cell.is_empty() && cell.chars().all(|ch| matches!(ch, '-' | ':' | ' ')))
}

/// The section under `heading`: the 1-based line of its heading and the lines
/// that follow it up to the next level-2 heading.
fn section<'a>(text: &'a str, heading: &str) -> Option<(usize, Vec<(usize, &'a str)>)> {
    let mut lines = text.lines().enumerate();
    let line = lines.find(|(_, line)| line.trim_end() == heading)?.0 + 1;
    let body = lines
        .take_while(|(_, line)| !line.starts_with("## "))
        .map(|(index, line)| (index + 1, line))
        .collect();
    Some((line, body))
}

/// The table under a section heading: each non-separator row as its `N` cells
/// and its 1-based line.
fn table<const N: usize>(
    heading: usize,
    body: &[(usize, &str)],
    columns: [&str; N],
) -> Result<Vec<(usize, [String; N])>, MappingError> {
    let mut table = body
        .iter()
        .skip_while(|(_, line)| !line.trim_start().starts_with('|'))
        .take_while(|(_, line)| line.trim_start().starts_with('|'));
    let Some(&(header_line, header)) = table.next() else {
        return Err(error(MappingErrorCode::MinRows, heading));
    };
    if cells(header) != columns {
        return Err(error(MappingErrorCode::Columns, header_line));
    }
    // A pipe block without a delimiter row is not a table (the extractor reads
    // no row from it), so it holds no rows.
    if !table
        .next()
        .is_some_and(|&(_, line)| is_separator(&cells(line)))
    {
        return Err(error(MappingErrorCode::MinRows, header_line));
    }
    let mut out = Vec::new();
    for &(line, raw) in table {
        let row = cells(raw);
        if is_separator(&row) {
            continue;
        }
        let row =
            <[String; N]>::try_from(row).map_err(|_| error(MappingErrorCode::RowArity, line))?;
        out.push((line, row));
    }
    if out.is_empty() {
        return Err(error(MappingErrorCode::MinRows, header_line));
    }
    Ok(out)
}

fn rows(heading: usize, body: &[(usize, &str)]) -> Result<Vec<Row>, MappingError> {
    Ok(table(heading, body, COLUMNS)?
        .into_iter()
        .map(|(line, [kind, target, impact, rationale])| Row {
            kind,
            target: (!target.is_empty() && target != "-").then_some(target),
            impact,
            rationale,
            line,
        })
        .collect())
}

fn rationale_entries(
    heading: usize,
    body: &[(usize, &str)],
) -> Result<Vec<RationaleEntry>, MappingError> {
    Ok(table(heading, body, RATIONALE_COLUMNS)?
        .into_iter()
        .map(|(_, [author, rationale])| RationaleEntry { author, rationale })
        .collect())
}

fn verification_rows(
    heading: usize,
    body: &[(usize, &str)],
) -> Result<Vec<VerificationRow>, MappingError> {
    Ok(table(heading, body, VERIFICATION_COLUMNS)?
        .into_iter()
        .map(|(line, [kind, target, rationale])| VerificationRow {
            kind,
            target: (!target.is_empty() && target != "-").then_some(target),
            rationale,
            line,
        })
        .collect())
}

/// The trimmed text of the section under `heading`, when the document has it.
fn section_text(text: &str, heading: &str) -> Option<String> {
    let (_, body) = section(text, heading)?;
    let joined = body
        .iter()
        .map(|&(_, line)| line)
        .collect::<Vec<_>>()
        .join("\n");
    Some(joined.trim().to_owned())
}

/// The assessment-section text of a document, trimmed, when it has the section.
#[must_use]
pub fn assessment_text(text: &str) -> Option<String> {
    section_text(text, SECTION)
}

/// The release-rationale-section text of a document, trimmed, when it has the
/// section.
#[must_use]
pub fn release_rationale_text(text: &str) -> Option<String> {
    section_text(text, RATIONALE_SECTION)
}

/// The 1-based line of the frontmatter `status` field, or 1 when there is none.
#[must_use]
pub fn status_line(text: &str) -> usize {
    text.lines()
        .enumerate()
        .skip(1)
        .take_while(|&(_, line)| line != "---")
        .find(|(_, line)| line.starts_with("status:"))
        .map_or(1, |(index, _)| index + 1)
}

/// The authored `status` word of a document, without building a record.
#[must_use]
pub fn status_word(text: &str) -> Option<String> {
    scalar(&frontmatter(text)?, "status")
}

/// Build the record of one `Anomaly` document.
///
/// # Errors
///
/// Returns the mapping error when the document cannot be read as an anomaly.
pub fn parse_record(text: &str) -> Result<Record, MappingError> {
    let (yaml, _) = split_frontmatter(text).ok_or_else(|| error(MappingErrorCode::Missing, 1))?;
    let front = quoin_yaml::from_str(&yaml).map_err(|_| error(MappingErrorCode::Missing, 1))?;
    let status_line = status_line(text);
    let status =
        scalar(&front, "status").ok_or_else(|| error(MappingErrorCode::Missing, status_line))?;
    let safety_impact = scalar(&front, "safety_impact")
        .ok_or_else(|| error(MappingErrorCode::Missing, status_line))?;
    let id = scalar(&front, "id").ok_or_else(|| error(MappingErrorCode::Missing, 1))?;
    let (assessment, assessment_text) = match section(text, SECTION) {
        None => (None, None),
        Some((heading, body)) => (Some(rows(heading, &body)?), assessment_text(text)),
    };
    let (release_rationale, release_rationale_text) = match section(text, RATIONALE_SECTION) {
        None => (None, None),
        Some((heading, body)) => (
            Some(rationale_entries(heading, &body)?),
            release_rationale_text(text),
        ),
    };
    let fix_verification = match section(text, VERIFICATION_SECTION) {
        None => None,
        Some((heading, body)) => Some(verification_rows(heading, &body)?),
    };
    Ok(Record {
        id,
        title: scalar(&front, "title"),
        status_word: status,
        status_line,
        resolution: scalar(&front, "resolution"),
        closed_by: scalar(&front, "closed_by"),
        safety_impact,
        severity: scalar(&front, "severity"),
        assessment,
        assessment_text,
        release_rationale,
        release_rationale_text,
        fix_verification,
    })
}
