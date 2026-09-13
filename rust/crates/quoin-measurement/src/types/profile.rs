// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The `AssuranceProfile` summary read out of an assurance document.
//!
//! Ports `src/measurement/profiles.ts:6-11`. The profile document itself is
//! not modelled here: `loadActiveAssuranceProfiles` reads four frontmatter
//! members and nothing else, and inventing a fuller model would be a
//! requirement this port was not given.

use crate::types::ids::NonEmptyText;
use crate::types::plan::LifecycleStatus;

/// One assurance profile, summarised from its frontmatter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssuranceProfileSummary {
    /// The profile's identity.
    pub id: NonEmptyText,
    /// Its human title.
    pub title: NonEmptyText,
    /// Where it is in its lifecycle.
    pub status: LifecycleStatus,
    /// The repository-relative, `/`-separated document it was read from.
    pub path: String,
}

impl AssuranceProfileSummary {
    /// The order `profiles.ts:23` sorts loaded profiles into: id, then path.
    #[must_use]
    pub fn sort_key(&self) -> (&str, &str) {
        (self.id.as_str(), self.path.as_str())
    }
}
