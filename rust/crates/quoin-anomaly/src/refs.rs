// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Reverse references: the anomalies whose assessment names an id.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::AnomalyError;
use crate::record::{cells, frontmatter, parse_record};

/// One anomaly that names the queried id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RefEntry {
    /// The anomaly's document id.
    pub anomaly: String,
    /// The row's `Kind`.
    pub kind: String,
    /// The row's `Impact`.
    pub impact: String,
}

fn markdown_files(root: &Path, out: &mut Vec<PathBuf>) -> Result<(), AnomalyError> {
    let entries = std::fs::read_dir(root).map_err(|error| AnomalyError::io(root, &error))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if matches!(name.to_str(), Some(".git" | "target" | "node_modules")) {
            continue;
        }
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            markdown_files(&path, out)?;
        } else if path.extension().is_some_and(|extension| extension == "md") {
            out.push(path);
        }
    }
    Ok(())
}

/// Every `Anomaly` document under `scope`, sorted by path, with its text.
///
/// # Errors
///
/// Returns an error when `scope` cannot be read.
pub fn anomaly_documents(scope: &Path) -> Result<Vec<(PathBuf, String)>, AnomalyError> {
    let mut files = Vec::new();
    markdown_files(scope, &mut files)?;
    files.sort();
    let mut out = Vec::new();
    for path in files {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let is_anomaly = frontmatter(&text)
            .and_then(|front| {
                front
                    .get("type")
                    .and_then(|t| t.as_str().map(str::to_owned))
            })
            .is_some_and(|kind| kind == "Anomaly");
        if is_anomaly {
            out.push((path, text));
        }
    }
    Ok(out)
}

/// Whether a document's frontmatter declares a hazard: `type` and `object` are
/// both `hazard`.
#[must_use]
pub fn is_hazard(frontmatter: &serde_json::Map<String, serde_json::Value>) -> bool {
    let word = |key: &str| frontmatter.get(key).and_then(serde_json::Value::as_str);
    word("type") == Some("hazard") && word("object") == Some("hazard")
}

/// The acceptance-criterion ids a requirement document declares: the first
/// cell of each table row that starts `<id>-AC-`.
#[must_use]
pub fn acceptance_criteria(text: &str, id: &str) -> Vec<String> {
    let prefix = format!("{id}-AC-");
    text.lines()
        .filter_map(|line| {
            let first = line
                .trim_start()
                .strip_prefix('|')?
                .split('|')
                .next()?
                .trim();
            first.starts_with(&prefix).then(|| first.to_owned())
        })
        .collect()
}

/// The test case ids a `TestMatrix` document declares: the first cell of each
/// row under a table whose first column is `Test ID`.
#[must_use]
pub fn test_case_ids(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_table = false;
    for line in text.lines() {
        if !line.trim_start().starts_with('|') {
            in_table = false;
            continue;
        }
        let row = cells(line);
        let first = row.first().map_or("", String::as_str);
        if first == "Test ID" {
            in_table = true;
        } else if in_table && !first.is_empty() && !first.chars().all(|c| matches!(c, '-' | ':')) {
            out.push(first.to_owned());
        }
    }
    out
}

/// The ids a `Target` can resolve to in the corpus under `scope`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CorpusIds {
    /// Requirement documents (`FR`, `NFR`) and hazard documents.
    pub documents: BTreeSet<String>,
    /// The acceptance criteria of the requirement documents.
    pub criteria: BTreeSet<String>,
    /// The test case ids of the `TestMatrix` documents.
    pub tests: BTreeSet<String>,
}

impl CorpusIds {
    /// Whether `id` resolves to anything in the corpus.
    #[must_use]
    pub fn contains(&self, id: &str) -> bool {
        self.documents.contains(id) || self.criteria.contains(id) || self.tests.contains(id)
    }
}

/// Read the resolvable ids of the corpus under `scope`: requirement documents,
/// their acceptance criteria, hazard documents and test case ids.
///
/// # Errors
///
/// Returns an error when `scope` cannot be read.
pub fn corpus_ids(scope: &Path) -> Result<CorpusIds, AnomalyError> {
    let mut files = Vec::new();
    markdown_files(scope, &mut files)?;
    let mut ids = CorpusIds::default();
    for path in files {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Some(front) = frontmatter(&text) else {
            continue;
        };
        let kind = front
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if kind == "TestMatrix" {
            ids.tests.extend(test_case_ids(&text));
        }
        let Some(id) = front.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        let requirement = matches!(kind, "FR" | "NFR");
        if requirement || front.as_object().is_some_and(is_hazard) {
            ids.documents.insert(id.to_owned());
        }
        if requirement {
            ids.criteria.extend(acceptance_criteria(&text, id));
        }
    }
    Ok(ids)
}

/// The anomalies under `scope` whose assessment names `id`, in anomaly id
/// order. A `None` row, and an id that resolves to nothing in the corpus,
/// yield no entry.
///
/// # Errors
///
/// Returns an error when `scope` cannot be read.
pub fn references_to(scope: &Path, id: &str) -> Result<Vec<RefEntry>, AnomalyError> {
    if !corpus_ids(scope)?.contains(id) {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for (_, text) in anomaly_documents(scope)? {
        let Ok(record) = parse_record(&text) else {
            continue;
        };
        for row in record.assessment.iter().flatten() {
            if row.target.as_deref() == Some(id) {
                out.push(RefEntry {
                    anomaly: record.id.clone(),
                    kind: row.kind.clone(),
                    impact: row.impact.clone(),
                });
            }
        }
    }
    out.sort_by(|a, b| a.anomaly.cmp(&b.anomaly));
    Ok(out)
}
