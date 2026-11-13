// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The documents below a scope: frontmatter, relationships and body tables.

use std::path::{Path, PathBuf};

use quoin_anomaly::record::frontmatter;
use serde_json::Value;

use crate::error::AdviceError;

/// A document larger than this is not read as a record.
const MAX_DOCUMENT_BYTES: u64 = 1024 * 1024;

/// One markdown document with an `id` and a `type` in its frontmatter.
pub(crate) struct Doc {
    pub(crate) id: String,
    pub(crate) kind: String,
    front: Value,
    pub(crate) text: String,
}

impl Doc {
    fn field(&self, key: &str) -> Option<&str> {
        self.front.get(key)?.as_str()
    }

    /// The frontmatter string at `key`, when present.
    pub(crate) fn front_str(&self, key: &str) -> Option<&str> {
        self.field(key)
    }

    /// The local ids this document's relationships of `verb` point at, sorted.
    pub(crate) fn edges(&self, verb: &str) -> Vec<String> {
        let mut targets: Vec<String> = self
            .front
            .get("relationships")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|edge| edge.get("type").and_then(Value::as_str) == Some(verb))
            .filter_map(|edge| edge.get("target")?.as_str())
            .map(local_id)
            .collect();
        targets.sort();
        targets.dedup();
        targets
    }

    /// The rows under the `## heading` table whose header is exactly `columns`.
    pub(crate) fn table(&self, heading: &str, columns: &[&str]) -> Vec<Vec<String>> {
        let marker = format!("## {heading}");
        let mut lines = self
            .text
            .lines()
            .skip_while(|line| line.trim_end() != marker)
            .skip(1)
            .take_while(|line| !line.starts_with("## "))
            .skip_while(|line| !line.trim_start().starts_with('|'))
            .take_while(|line| line.trim_start().starts_with('|'));
        let Some(header) = lines.next() else {
            return Vec::new();
        };
        if cells(header) != columns {
            return Vec::new();
        }
        lines
            .map(cells)
            .filter(|row| !is_separator(row) && row.len() == columns.len())
            .collect()
    }
}

/// The id at the end of an `ix://` target, or the target itself.
fn local_id(target: &str) -> String {
    target
        .rsplit('/')
        .next()
        .unwrap_or(target)
        .trim()
        .to_owned()
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
    row.iter()
        .all(|cell| !cell.is_empty() && cell.chars().all(|ch| matches!(ch, '-' | ':' | ' ')))
}

fn markdown_files(root: &Path, out: &mut Vec<PathBuf>) -> Result<(), AdviceError> {
    let entries = std::fs::read_dir(root).map_err(|error| AdviceError::io(root, &error))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if matches!(
            entry.file_name().to_str(),
            Some(".git" | "target" | "node_modules")
        ) {
            continue;
        }
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            markdown_files(&path, out)?;
        } else if kind.is_file() && path.extension().is_some_and(|ext| ext == "md") {
            out.push(path);
        }
    }
    Ok(())
}

/// Every document below `scope` that carries an `id` and a `type`, by path.
pub(crate) fn load(scope: &Path) -> Result<Vec<Doc>, AdviceError> {
    let mut files = Vec::new();
    markdown_files(scope, &mut files)?;
    files.sort();
    let mut docs = Vec::new();
    for path in files {
        let small = std::fs::metadata(&path).is_ok_and(|meta| meta.len() <= MAX_DOCUMENT_BYTES);
        let Some(text) = small.then(|| std::fs::read_to_string(&path).ok()).flatten() else {
            continue;
        };
        let Some(front) = frontmatter(&text) else {
            continue;
        };
        let id = front.get("id").and_then(Value::as_str).map(str::to_owned);
        let kind = front.get("type").and_then(Value::as_str).map(str::to_owned);
        if let (Some(id), Some(kind)) = (id, kind) {
            docs.push(Doc {
                id,
                kind,
                front,
                text,
            });
        }
    }
    Ok(docs)
}
