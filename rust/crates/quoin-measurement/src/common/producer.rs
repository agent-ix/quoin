// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Who was measured, and what measured it.
//!
//! `intervention-types.ts:22-31` and `operational-types.ts:27-38` declare
//! `subject` and `producer` inline, twice, with identical fields. They are one
//! shape here. The plan's §13.2 makes that a review criterion rather than a
//! preference: porting a known duplicate twice carries it into new code.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::common::identity::{Digest, Revision, SubjectId};
use crate::common::scalar::EnvironmentValue;

/// What a record is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subject {
    /// The subject's identity.
    pub id: SubjectId,
    /// The revision of the subject that was measured.
    pub revision: Revision,
}

/// What produced a record, and under what configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Producer {
    /// The producing tool.
    pub tool_identity: String,
    /// The producing tool's version.
    pub tool_version: String,
    /// A digest of the producer's configuration.
    pub configuration_digest: Digest,
    /// The revision of the producer's own source.
    pub source_revision: Revision,
    /// The environment the production ran in.
    ///
    /// `BTreeMap` rather than a hash map: the serialised form must be
    /// byte-stable, which is the same reason the error envelopes in
    /// `quoin-core` use one.
    pub environment: BTreeMap<String, EnvironmentValue>,
    /// The version of the producer definition this record was produced from.
    pub definition_version: String,
}
