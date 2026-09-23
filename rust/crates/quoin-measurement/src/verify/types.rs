// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The checker's input and result types (quoin FR-108).
//!
//! Split from [`super`] so the checker stays under this crate's module-size
//! ceiling.

use std::collections::BTreeMap;

use quoin_store::JsonValue;

use crate::types::collection::MeasurementCollection;
use crate::verify::{Estimate, Reason, Verdict};

/// Where the intake order handed to the checker came from (FR-108-AC-8).
///
/// Only [`OrderSource::GitFirstParentAdd`] is an order the producer cannot
/// choose after the fact; a consumer granting credit reads this member
/// before trusting `orderAttested`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum OrderSource {
    /// The first-parent git commit that added each collection file.
    GitFirstParentAdd,
    /// The repository's git history is shallow, so no first-add commit can
    /// be trusted; every run is unpositioned and the result carries
    /// `order_unattested`.
    GitShallow,
    /// A position list the caller supplied without saying where it came
    /// from. The checker uses it, and attests nothing about it.
    CallerSupplied,
    /// No order at all.
    None,
}

impl OrderSource {
    /// Every source, in declaration order.
    pub const ALL: [Self; 4] = [
        Self::GitFirstParentAdd,
        Self::GitShallow,
        Self::CallerSupplied,
        Self::None,
    ];

    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GitFirstParentAdd => "git-first-parent-add",
            Self::GitShallow => "git-shallow",
            Self::CallerSupplied => "caller-supplied",
            Self::None => "none",
        }
    }

    /// Recover a source from its wire spelling.
    #[must_use]
    pub fn from_wire(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == value)
    }
}

/// One stored collection and its intake position.
#[derive(Clone, Copy, Debug)]
pub struct Ranked<'a> {
    /// The collection as stored.
    pub collection: &'a MeasurementCollection,
    /// Its position in an order the producer cannot choose — lower is
    /// earlier, equal is a tie — or `None` when nothing attests one.
    pub intake: Option<u64>,
    /// Whether this collection's recorded protected-apparatus digests for
    /// the plan being verified disagree with `git show
    /// <sourceRevision>:<path>` — the file's actual bytes at the commit the
    /// collection claims to be from. The caller alone has git, so it checks
    /// this and hands over the answer; `false` when nothing was checked, so
    /// a caller with no git access reports no forgery rather than a false
    /// one (PLAT-985).
    pub apparatus_forged: bool,
}

impl<'a> Ranked<'a> {
    /// A ranked collection with no tamper facts recorded — the common case
    /// for a caller with no git access to check them, and for a test that is
    /// not itself exercising PLAT-985's tamper checks.
    #[must_use]
    pub const fn new(collection: &'a MeasurementCollection, intake: Option<u64>) -> Self {
        Self {
            collection,
            intake,
            apparatus_forged: false,
        }
    }
}

/// Facts about the store's own git history that only the caller — which
/// alone runs git — can supply (PLAT-985). Each defaults to "nothing found",
/// so a caller with no git access reports no tampering rather than a false
/// one.
#[derive(Clone, Copy, Debug, Default)]
pub struct TamperFacts<'a> {
    /// Ids of collections that measured this plan under its metric and
    /// `definition_version`, were once added to the store's git history, and
    /// no longer exist there — the caller attributes a deleted id to a plan
    /// by reading its content at the commit before it was removed.
    pub deleted_collections: &'a [String],
    /// Ids of collections that named this plan — in the content intake first
    /// added or in their content now — whose stored file a later commit, or
    /// the uncommitted work tree, changed or re-added. Attributed by plan
    /// rather than read off the runs, so an edit that re-targets a run away
    /// from this plan still reports here rather than silently removing it.
    pub edited_collections: &'a [String],
    /// Whether the plan's objective, estimator, decision rule or protected
    /// apparatus changed between two committed revisions of its document
    /// that share a `definition_version` — engineering-assurance's
    /// `definition_change_without_version_bump`, applied by the caller over
    /// the plan document's git history, which this crate cannot read.
    pub definition_changed_without_version_bump: bool,
}

/// One reason, and where it was found.
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    /// Why.
    pub reason: Reason,
    /// The collection it was found in; `None` for a plan-level reason.
    pub collection_id: Option<String>,
    /// The observation's dimensions; `None` for a collection- or plan-level
    /// reason.
    pub dimensions: Option<BTreeMap<String, JsonValue>>,
}

/// The rule applied to one slice of the candidate collection.
#[derive(Clone, Debug, PartialEq)]
pub struct SliceDecision {
    /// The slice.
    pub dimensions: BTreeMap<String, JsonValue>,
    /// The estimate, and whether it was recomputed or asserted.
    pub estimate: Estimate,
    /// The baseline value, for a baseline rule that found one.
    pub baseline: Option<f64>,
    /// Whether the rule holds; `None` when it could not be evaluated.
    pub holds: Option<bool>,
}

/// What the checker counted.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Counts {
    /// Runs: collections holding the plan's metric under its definition.
    pub collections_considered: usize,
    /// Runs the rule does not hold for, against their own history.
    pub regressed_runs: usize,
    /// Observations whose estimate was recomputed from `matched` and
    /// `examined` — including those found to disagree with their `value`.
    pub observations_recomputed: usize,
    /// Observations whose stored `value` was used as stated.
    pub observations_asserted: usize,
    /// Runs with an intake position.
    pub order_attested: usize,
    /// Runs with none.
    pub order_unattested: usize,
}

/// The checker's typed result.
#[derive(Clone, Debug, PartialEq)]
pub struct MeasurementVerdict {
    /// The plan checked.
    pub plan_id: String,
    /// Its definition version.
    pub definition_version: String,
    /// The verdict.
    pub verdict: Verdict,
    /// Every distinct reason, sorted; empty exactly when accepted.
    pub reasons: Vec<Reason>,
    /// The verdict the caller claimed, when one was.
    pub claimed: Option<Verdict>,
    /// The candidate: the last run in intake order.
    pub candidate: Option<String>,
    /// The rule applied to each slice of the candidate.
    pub decisions: Vec<SliceDecision>,
    /// Every reason, and where.
    pub findings: Vec<Finding>,
    /// The runs that regressed, in intake order.
    pub regressed_runs: Vec<String>,
    /// Where the intake order came from.
    pub order_source: OrderSource,
    /// What was counted.
    pub counts: Counts,
}
