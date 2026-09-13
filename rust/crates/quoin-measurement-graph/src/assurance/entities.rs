// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The nodes an `assurance-v1` export carries: locators, artifacts,
//! obligations and symbols.
//!
//! Ports `locatorSchema`, `artifactSchema`, `obligationSchema` and
//! `symbolSchema` (`src/measurement/graph-adapters.ts:51-112`).

use std::collections::BTreeMap;
use std::num::NonZeroU64;

use serde::Deserialize;

use crate::scalars::{ArtifactUuid, BareDigest, LocatorPath, NonEmptyText};
use crate::wire::{NullableText, absent_or};

/// Where a fact was observed: a relative path, a 1-based line, and the digest
/// of the file it was read from.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Locator {
    /// The repository-relative path, with no `..` segment.
    pub path: LocatorPath,
    /// The 1-based line. `z.number().int().min(1)` is a [`NonZeroU64`].
    pub line: NonZeroU64,
    /// The digest of the file the fact was read from.
    pub digest: BareDigest,
}

/// One spec artifact the export declares.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    /// The artifact's identity.
    pub id: NonEmptyText,
    /// Its UUID, when it carries one.
    #[serde(default, deserialize_with = "absent_or")]
    pub uuid: Option<ArtifactUuid>,
    /// Which archetype it is.
    pub artifact_type: NonEmptyText,
    /// Where it was read from.
    pub locator: Locator,
}

/// One obligation the export declares, with the statement it binds.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Obligation {
    /// Which document family it came from.
    pub source: NonEmptyText,
    /// Its identity.
    pub id: NonEmptyText,
    /// The document it was stated in.
    pub document: NonEmptyText,
    /// The statement itself.
    pub statement: NonEmptyText,
    /// The digest of that statement.
    pub statement_hash: BareDigest,
    /// The verification method, when declared.
    #[serde(default, deserialize_with = "absent_or")]
    pub method: Option<NonEmptyText>,
    /// The method's parameters, when declared.
    #[serde(default, deserialize_with = "absent_or")]
    pub parameters: Option<BTreeMap<String, String>>,
    /// The criticality, when declared.
    #[serde(default, deserialize_with = "absent_or")]
    pub criticality: Option<NonEmptyText>,
    /// What it targets.
    pub target_ids: Vec<NonEmptyText>,
    /// Where it was read from.
    pub locator: Locator,
}

/// The three languages the extractor reads symbols from.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolLanguage {
    /// Rust.
    Rust,
    /// Python.
    Python,
    /// TypeScript.
    Typescript,
}

/// What kind of symbol was extracted.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    /// An ordinary function.
    Function,
    /// A function that is a test.
    TestFunction,
    /// A type, module or class holding others.
    Container,
    /// A benchmark.
    Benchmark,
    /// A fuzz target.
    FuzzTarget,
}

/// What a symbol is capable of binding.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolCapability {
    /// It can verify an obligation.
    Verifies,
    /// It can implement one.
    Implements,
}

/// One extracted symbol.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Symbol {
    /// Its identity, a bare digest.
    pub id: BareDigest,
    /// The language it was extracted from.
    pub language: SymbolLanguage,
    /// What kind of symbol it is.
    pub kind: SymbolKind,
    /// Its fully qualified name.
    pub qualified_name: NonEmptyText,
    /// The container it sits in, or null for a free symbol.
    ///
    /// `z.string().nullable()` — required, and allowed to be null. See
    /// [`NullableText`] for why that is not `Option<String>`.
    pub container: NullableText,
    /// What it can bind.
    pub capabilities: Vec<SymbolCapability>,
    /// Where it was read from.
    pub locator: Locator,
}
