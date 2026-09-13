// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The premises an export is read *under*, and the ones it states.
//!
//! Ports `sourcePremiseSchema`, `schemaPremiseSchema`, `modulePremiseSchema`
//! and `AcceptedQuirePremises` (`src/measurement/graph-adapters.ts:63-76,
//! 205-210`).

use serde::{Deserialize, Serialize};

use crate::scalars::{BareDigest, FullRevision, NonEmptyText};

/// Which repository, at which revision, the export was taken from.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePremise {
    /// The repository.
    pub repository: NonEmptyText,
    /// The full revision it was at.
    pub revision: FullRevision,
}

/// One archetype schema a module contributed, and its digest.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaPremise {
    /// The archetype the schema governs.
    pub archetype: NonEmptyText,
    /// The schema's digest.
    pub schema_digest: BareDigest,
}

/// One module the export was produced under.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModulePremise {
    /// The module's name.
    pub name: NonEmptyText,
    /// Its version.
    pub version: NonEmptyText,
    /// The archetype schemas it contributed.
    pub schemas: Vec<SchemaPremise>,
}

/// The premises a caller has already agreed to read an export under.
///
/// `adaptQuireAssurance` does not *discover* these — it refuses an export whose
/// own premises disagree with the ones the caller stated. A caller that cannot
/// state them wants `quoin_quire::assurance::read` instead, which is the
/// stronger check.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcceptedQuirePremises {
    /// The source the caller accepts.
    pub source: SourcePremise,
    /// The module set the caller accepts, in the order it accepts them in.
    pub modules: Vec<ModulePremise>,
}
