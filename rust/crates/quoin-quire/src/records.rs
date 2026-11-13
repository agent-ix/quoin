// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The documents below a scope, read with the engine's own walker and parser.
//!
//! [`read`] loads every document with the engine's repository walker and
//! returns, per document, the identity and edges the engine harvests and the
//! first table of each level-2 section as the engine's parser reads it. For the
//! types a caller names it also validates the document against its type as
//! `quire validate` does, so a document that breaks its own type's contract (a
//! header that is not the declared one, a value outside the declared choices,
//! an unfilled placeholder) is reported as [`Contract::Violated`] with the
//! engine's reason and is never read as an empty record.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use quire_rs::{
    Diagnostic, Registry, WalkOptions, load_repo_with, parse_table, sections,
    validate_document_in_registry,
};
use serde_json::{Map, Value};

use crate::error::{Error, Result};
use crate::ids::ScopeRoot;
use crate::modules::ModuleSelection;

/// Whether a document passes the engine's validation for its type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Contract {
    /// The caller named no check for this document's type.
    NotChecked,
    /// The engine finds no error.
    Satisfied,
    /// The engine's first error.
    Violated(String),
}

/// A table as the engine's parser reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// The header cells.
    pub headers: Vec<String>,
    /// The data rows, one cell per header.
    pub rows: Vec<Vec<String>>,
}

/// One document below the scope.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    /// Where it was read.
    pub path: PathBuf,
    /// The frontmatter `id`.
    pub id: String,
    /// The frontmatter `type`.
    pub kind: String,
    /// The whole frontmatter mapping.
    pub frontmatter: Map<String, Value>,
    /// The edges the frontmatter `relationships` declare, as the engine reads
    /// them: target id and edge type, sorted and distinct. Links in the body
    /// are not edges here.
    pub edges: Vec<(String, String)>,
    /// The first table of each level-2 section, by section heading.
    pub tables: BTreeMap<String, Table>,
    /// The verbatim file text.
    pub raw: String,
    /// The result of the declared-contract check.
    pub contract: Contract,
}

/// A file the engine's walker could not load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unreadable {
    /// The file.
    pub path: PathBuf,
    /// The engine's reason.
    pub reason: String,
}

/// What [`read`] found below the scope.
#[derive(Debug, Clone, PartialEq)]
pub struct Corpus {
    /// The documents, ordered by path.
    pub documents: Vec<Document>,
    /// The files the walker could not load, ordered by path.
    pub unreadable: Vec<Unreadable>,
}

/// Every document below `scope` that carries an `id` and a `type`, ordered by
/// path. Documents whose `type` is in `checked_types` are also checked against
/// validation for their type.
///
/// # Errors
/// [`Error::ScopeNotADirectory`] when `scope` is not a directory,
/// [`Error::ModuleLoad`] or [`Error::ModuleSetEmpty`] when the modules do not
/// load, and [`Error::ArchetypeUnknown`] when a checked type has no registered
/// archetype.
pub fn read(scope: &Path, selection: &ModuleSelection, checked_types: &[&str]) -> Result<Corpus> {
    let root = ScopeRoot::open(scope)?;
    let registry = selection.resolve(&root, &mut Vec::new())?;
    let loaded = load_repo_with(root.as_path(), &WalkOptions::default());
    let mut unreadable: Vec<Unreadable> = loaded
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match diagnostic {
            Diagnostic::DocumentUnreadable { path, reason } => Some(Unreadable {
                path: path.clone(),
                reason: reason.clone(),
            }),
            _ => None,
        })
        .collect();
    unreadable.sort_by(|a, b| a.path.cmp(&b.path));
    let mut documents = Vec::new();
    for found in &loaded.documents {
        let Some(frontmatter) = found.frontmatter() else {
            continue;
        };
        let Some(kind) = frontmatter.get("type").and_then(Value::as_str) else {
            continue;
        };
        if found.id.is_empty() {
            continue;
        }
        let contract = if checked_types.contains(&kind) {
            check(&registry, found, kind)?
        } else {
            Contract::NotChecked
        };
        let body = found.body();
        let tables = sections(body, Some(2))
            .into_iter()
            .filter_map(|section| {
                let table = parse_table(&section.content)?;
                Some((
                    section.heading.clone(),
                    Table {
                        headers: table.headers,
                        rows: table.rows,
                    },
                ))
            })
            .collect();
        documents.push(Document {
            path: found.path.clone(),
            id: found.id.clone(),
            kind: kind.to_owned(),
            frontmatter: frontmatter.clone(),
            edges: relationships(frontmatter),
            tables,
            raw: found.raw().to_owned(),
            contract,
        });
    }
    documents.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(Corpus {
        documents,
        unreadable,
    })
}

/// The `(target id, edge type)` pairs of the frontmatter `relationships`, read
/// as the engine reads them: the target is the last `/`-segment of an `ix://`
/// URI or a bare id, and an entry with no `type` is a `references` edge.
fn relationships(frontmatter: &Map<String, Value>) -> Vec<(String, String)> {
    let mut edges: Vec<(String, String)> = frontmatter
        .get("relationships")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let target = entry.get("target")?.as_str()?;
            let target = target.strip_prefix("ix://").unwrap_or(target);
            let id = target.rsplit('/').next().unwrap_or(target);
            let kind = entry
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("references");
            Some((id.to_owned(), kind.to_owned()))
        })
        .collect();
    edges.sort();
    edges.dedup();
    edges
}

/// Validate the document against its type as `quire validate` does, so the
/// type's declared `body_extraction` assertions, frontmatter and data schema
/// all apply.
fn check(registry: &Registry, found: &quire_rs::LoadedDocument, kind: &str) -> Result<Contract> {
    let archetype = registry
        .archetype(kind)
        .ok_or_else(|| Error::ArchetypeUnknown {
            document: found.path.display().to_string(),
            archetype: kind.to_owned(),
        })?;
    let result = validate_document_in_registry(registry, archetype, found.raw());
    Ok(match result.errors.first() {
        None => Contract::Satisfied,
        Some(error) => Contract::Violated(error.message.clone()),
    })
}
