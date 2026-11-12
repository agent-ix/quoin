// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The parts of an `Anomaly` document this crate reads: `status`, `resolution`,
//! `closed_by`, `safety_impact` and the `## Safety Impact Assessment` table.
//!
//! Field vocabularies and the legality of a field combination are the module's
//! (spec-artifacts-process FR-014/FR-015) and `quire validate`'s. A document
//! that fails only those checks still yields its record; only a document the
//! mapping cannot read (no status, a table with the wrong columns) yields none.

use serde_json::Value;

const COLUMNS: [&str; 4] = ["Kind", "Target", "Impact", "Rationale"];
const KINDS: [&str; 3] = ["Requirement", "Hazard", "None"];
const IMPACTS: [&str; 5] = ["Affects", "Mitigates", "Introduces", "Unaffected", "None"];
const SECTION: &str = "## Safety Impact Assessment";

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

/// The record of one `Anomaly` document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The document id.
    pub id: String,
    /// The authored `status` word.
    pub status: String,
    /// 1-based document line of the frontmatter `status` field.
    pub status_line: usize,
    /// The authored `resolution`, when present.
    pub resolution: Option<String>,
    /// The authored `closed_by`, when present.
    pub closed_by: Option<String>,
    /// The authored `safety_impact`.
    pub safety_impact: String,
    /// The assessment rows; `None` while the section is absent.
    pub assessment: Option<Vec<Row>>,
    /// The text of the assessment section, trimmed; `None` while it is absent.
    pub assessment_text: Option<String>,
}

/// Why a document yields no record: the mapping's error code and its line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappingError {
    /// The mapping error code (`missing`, `columns`, `row-arity`,
    /// `column-choices`, `min-rows`).
    pub code: &'static str,
    /// 1-based document line the error is reported at.
    pub line: usize,
}

fn error(code: &'static str, line: usize) -> MappingError {
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
fn cells(line: &str) -> Vec<String> {
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

/// The assessment section: its trimmed text, and the 1-based line of its
/// heading and the lines that follow it up to the next level-2 heading.
fn section(text: &str) -> Option<(usize, Vec<(usize, &str)>)> {
    let mut lines = text.lines().enumerate();
    let heading = lines.find(|(_, line)| line.trim_end() == SECTION)?.0 + 1;
    let body = lines
        .take_while(|(_, line)| !line.starts_with("## "))
        .map(|(index, line)| (index + 1, line))
        .collect();
    Some((heading, body))
}

fn rows(heading: usize, body: &[(usize, &str)]) -> Result<Vec<Row>, MappingError> {
    let mut table = body
        .iter()
        .skip_while(|(_, line)| !line.trim_start().starts_with('|'))
        .take_while(|(_, line)| line.trim_start().starts_with('|'));
    let Some(&(header_line, header)) = table.next() else {
        return Err(error("min-rows", heading));
    };
    if cells(header) != COLUMNS {
        return Err(error("columns", header_line));
    }
    let mut out = Vec::new();
    for &(line, raw) in table {
        let row = cells(raw);
        if is_separator(&row) {
            continue;
        }
        let [kind, target, impact, rationale] =
            <[String; 4]>::try_from(row).map_err(|_| error("row-arity", line))?;
        if !KINDS.contains(&kind.as_str()) || !IMPACTS.contains(&impact.as_str()) {
            return Err(error("column-choices", line));
        }
        out.push(Row {
            kind,
            target: (!target.is_empty() && target != "-").then_some(target),
            impact,
            rationale,
            line,
        });
    }
    if out.is_empty() {
        return Err(error("min-rows", header_line));
    }
    Ok(out)
}

/// The assessment-section text of a document, trimmed, when it has the section.
#[must_use]
pub fn assessment_text(text: &str) -> Option<String> {
    let (_, body) = section(text)?;
    let joined = body
        .iter()
        .map(|&(_, line)| line)
        .collect::<Vec<_>>()
        .join("\n");
    Some(joined.trim().to_owned())
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
    let (yaml, _) = split_frontmatter(text).ok_or_else(|| error("missing", 1))?;
    let front = quoin_yaml::from_str(&yaml).map_err(|_| error("missing", 1))?;
    let status_line = text
        .lines()
        .enumerate()
        .take_while(|&(index, line)| index == 0 || line != "---")
        .find(|(_, line)| line.starts_with("status:"))
        .map_or(1, |(index, _)| index + 1);
    let status = scalar(&front, "status").ok_or_else(|| error("missing", status_line))?;
    let safety_impact =
        scalar(&front, "safety_impact").ok_or_else(|| error("missing", status_line))?;
    let id = scalar(&front, "id").ok_or_else(|| error("missing", 1))?;
    let (assessment, assessment_text) = match section(text) {
        None => (None, None),
        Some((heading, body)) => (Some(rows(heading, &body)?), assessment_text(text)),
    };
    Ok(Record {
        id,
        status,
        status_line,
        resolution: scalar(&front, "resolution"),
        closed_by: scalar(&front, "closed_by"),
        safety_impact,
        assessment,
        assessment_text,
    })
}
