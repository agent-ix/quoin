// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Resolving every `<repository>=<value>` mapping **before** anything is read.
//!
//! Ports `parseGraphPortfolioMappings`, `resolvePathMappings`,
//! `resolveChangedMappings` and `splitMapping`
//! (`graph-portfolio.ts:209-268,738-838`).
//!
//! # Why this happens first
//!
//! `graph-portfolio-load.ts:22` calls this before its first read and says why:
//! *"Mapping validation is deliberately first: conflicts cannot trigger
//! reads."* A caller who mapped one repository to two different graph exports
//! has said something contradictory, and discovering that after half the
//! repositories have been walked would mean a refusal whose side effects had
//! already happened.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use quoin_measurement::portfolio::location::{resolve, resolve_against};
use quoin_store::JsonValue;

use crate::availability::MappingRefusal;
use crate::error::{GraphAdapterError, GraphAdapterErrorCode};
use crate::order::compare_text;

/// What a caller may map onto the repositories it named.
/// `graph-portfolio.ts:84-90`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GraphPortfolioMappingOptions {
    /// `<repository>=<graph export>` mappings.
    pub graph_exports: Vec<String>,
    /// `<repository>=<premises document>` mappings.
    pub graph_premises: Vec<String>,
    /// `<repository>=<audit document>` mappings.
    pub graph_audits: Vec<String>,
    /// `<repository>=<changed requirement>` seeds; repeatable per repository.
    pub changed: Vec<String>,
    /// What relative paths resolve against. The process's working directory
    /// when absent, which is `process.cwd()` (`graph-portfolio.ts:211`).
    pub cwd: Option<PathBuf>,
}

/// One repository's mappings, resolved. `graph-portfolio.ts:92-107`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedMapping {
    /// All three documents were mapped.
    Ready {
        /// The resolved repository root.
        root: PathBuf,
        /// The resolved graph export.
        export_path: PathBuf,
        /// The resolved premises document.
        premises_path: PathBuf,
        /// The resolved audit document.
        audit_path: PathBuf,
        /// The changed requirement seeds, ordered.
        changed: Vec<String>,
    },
    /// They were not, and structural reporting is refused for this repository.
    Refused {
        /// The resolved repository root.
        root: PathBuf,
        /// Whether nothing was mapped, or only some of the three.
        status: MappingRefusal,
        /// The changed requirement seeds, ordered.
        changed: Vec<String>,
    },
}

impl ResolvedMapping {
    /// `graph-portfolio.ts:249`.
    const NOTHING_MAPPED: &'static str = "no graph export, premises, or audit mapping was supplied";

    /// `graph-portfolio.ts:255-256`.
    const PARTLY_MAPPED: &'static str =
        "graph structural reporting requires export, premises, and audit mappings";

    /// The resolved repository root.
    #[must_use]
    pub fn root(&self) -> &Path {
        match *self {
            Self::Ready { ref root, .. } | Self::Refused { ref root, .. } => root,
        }
    }

    /// The changed requirement seeds.
    #[must_use]
    pub fn changed(&self) -> &[String] {
        match *self {
            Self::Ready { ref changed, .. } | Self::Refused { ref changed, .. } => changed,
        }
    }

    /// The stated reason a refused mapping is refused.
    #[must_use]
    pub const fn reason(&self) -> Option<&'static str> {
        match *self {
            Self::Ready { .. } => None,
            Self::Refused { status, .. } => Some(match status {
                MappingRefusal::Missing => Self::NOTHING_MAPPED,
                MappingRefusal::Incompatible => Self::PARTLY_MAPPED,
            }),
        }
    }
}

/// Resolve every mapping a caller supplied, or refuse the whole set.
///
/// `parseGraphPortfolioMappings` (`graph-portfolio.ts:209-268`). Locations are
/// resolved, de-duplicated and ordered first, so two spellings of one
/// repository are one entry and a mapping may name either spelling.
///
/// # Errors
///
/// [`GraphAdapterErrorCode::InvalidRepositoryMapping`] for a mapping that is not
/// `<repository>=<value>` with both halves non-empty, or that names a
/// repository absent from `locations`; and the three `DuplicateGraph…` codes
/// when one repository is mapped to two different documents of one kind.
pub fn parse_graph_portfolio_mappings(
    locations: &[PathBuf],
    options: &GraphPortfolioMappingOptions,
) -> Result<Vec<ResolvedMapping>, GraphAdapterError> {
    let cwd = options
        .cwd
        .clone()
        .unwrap_or_else(|| resolve(Path::new(".")));
    let mut roots: Vec<PathBuf> = Vec::new();
    for location in locations {
        let root = resolve_against(&cwd, location);
        if !roots.contains(&root) {
            roots.push(root);
        }
    }
    roots.sort_by(|left, right| compare_text(&left.to_string_lossy(), &right.to_string_lossy()));
    let known: BTreeSet<&PathBuf> = roots.iter().collect();

    let exports = path_mappings(
        &options.graph_exports,
        GraphAdapterErrorCode::DuplicateGraphExport,
        &known,
        &cwd,
    )?;
    let premises = path_mappings(
        &options.graph_premises,
        GraphAdapterErrorCode::DuplicateGraphPremises,
        &known,
        &cwd,
    )?;
    let audits = path_mappings(
        &options.graph_audits,
        GraphAdapterErrorCode::DuplicateGraphAudit,
        &known,
        &cwd,
    )?;
    let changed = changed_mappings(&options.changed, &known, &cwd)?;

    Ok(roots
        .into_iter()
        .map(|root| {
            let seeds = changed.get(&root).cloned().unwrap_or_default();
            match (exports.get(&root), premises.get(&root), audits.get(&root)) {
                (Some(export_path), Some(premises_path), Some(audit_path)) => {
                    ResolvedMapping::Ready {
                        root,
                        export_path: export_path.clone(),
                        premises_path: premises_path.clone(),
                        audit_path: audit_path.clone(),
                        changed: seeds,
                    }
                }
                (None, None, None) => ResolvedMapping::Refused {
                    root,
                    status: MappingRefusal::Missing,
                    changed: seeds,
                },
                _ => ResolvedMapping::Refused {
                    root,
                    status: MappingRefusal::Incompatible,
                    changed: seeds,
                },
            }
        })
        .collect())
}

/// `resolvePathMappings` (`graph-portfolio.ts:772-800`).
fn path_mappings(
    values: &[String],
    conflict: GraphAdapterErrorCode,
    known: &BTreeSet<&PathBuf>,
    cwd: &Path,
) -> Result<BTreeMap<PathBuf, PathBuf>, GraphAdapterError> {
    let mut resolved: BTreeMap<PathBuf, PathBuf> = BTreeMap::new();
    for value in values {
        let (repository, path) = split_mapping(value)?;
        let root = known_root(repository, known, cwd)?;
        let target = resolve_against(cwd, Path::new(path));
        if let Some(prior) = resolved.get(&root)
            && *prior != target
        {
            return Err(GraphAdapterError::new(
                conflict,
                format!(
                    "{} maps to both {} and {}",
                    root.display(),
                    prior.display(),
                    target.display()
                ),
            ));
        }
        resolved.insert(root, target);
    }
    Ok(resolved)
}

/// `resolveChangedMappings` (`graph-portfolio.ts:802-822`).
///
/// The seeds are de-duplicated as the retained `Set` does and then sorted with
/// [`compare_text`] as `graph-portfolio.ts:243` does — not with `Ord for
/// String`, which is UTF-8 byte order and disagrees with JavaScript's `<` on
/// any pair that straddles the surrogate range.
fn changed_mappings(
    values: &[String],
    known: &BTreeSet<&PathBuf>,
    cwd: &Path,
) -> Result<BTreeMap<PathBuf, Vec<String>>, GraphAdapterError> {
    let mut resolved: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();
    for value in values {
        let (repository, seed) = split_mapping(value)?;
        let root = known_root(repository, known, cwd)?;
        let seeds = resolved.entry(root).or_default();
        if !seeds.iter().any(|held| held == seed) {
            seeds.push(seed.to_owned());
        }
    }
    for seeds in resolved.values_mut() {
        seeds.sort_by(|left, right| compare_text(left, right));
    }
    Ok(resolved)
}

/// The resolved root a mapping names, refusing one the portfolio does not hold.
fn known_root(
    repository: &str,
    known: &BTreeSet<&PathBuf>,
    cwd: &Path,
) -> Result<PathBuf, GraphAdapterError> {
    let root = resolve_against(cwd, Path::new(repository));
    if known.contains(&root) {
        Ok(root)
    } else {
        Err(GraphAdapterError::new(
            GraphAdapterErrorCode::InvalidRepositoryMapping,
            format!(
                "repository {} is not present in --portfolio",
                root.display()
            ),
        ))
    }
}

/// `splitMapping` (`graph-portfolio.ts:824-838`).
///
/// Splits on the FIRST `=`, so a value may contain one. The retained guard
/// `separator < 1 || separator === value.length - 1` is exactly "neither half
/// is empty", which is what is written here.
fn split_mapping(value: &str) -> Result<(&str, &str), GraphAdapterError> {
    let Some((repository, target)) = value.split_once('=') else {
        return Err(malformed("expected <repository>=<value>; observed", value));
    };
    if repository.is_empty() || target.is_empty() {
        return Err(malformed("expected <repository>=<value>; observed", value));
    }
    let repository = repository.trim();
    let target = target.trim();
    if repository.is_empty() || target.is_empty() {
        return Err(malformed(
            "expected non-empty repository and value; observed",
            value,
        ));
    }
    Ok((repository, target))
}

/// `${sentence} ${JSON.stringify(value)}`.
///
/// The quoting is [`quoin_store::canonical_json`]'s, not a second one: a
/// string is a leaf, so the canonical writer's bytes for it are exactly
/// `JSON.stringify`'s, and the only edit is dropping the trailing newline the
/// store's writer appends.
fn malformed(sentence: &str, value: &str) -> GraphAdapterError {
    let quoted = crate::canonical::stored_pretty_text_trimmed(&JsonValue::string(value))
        .unwrap_or_else(|error| error.to_string());
    GraphAdapterError::new(
        GraphAdapterErrorCode::InvalidRepositoryMapping,
        format!("{sentence} {quoted}"),
    )
}
