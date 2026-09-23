// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! What the intake order can and cannot attest (quoin FR-108-AC-2).
//!
//! Split from [`super`] so the checker stays under this crate's module-size
//! ceiling. A position is attested only when it is the run's alone and the
//! producer's own stated timestamps do not contradict it: a tie leaves the
//! order open, and a disagreement means one of the two orders was chosen.

use std::cmp::Ordering;

use crate::date_time::Rfc3339DateTime;
use crate::store::read::collection_order;
use crate::types::collection::MeasurementCollection;

use super::Run;

/// The sort key of an intake position: positioned runs first, by position.
const fn key(intake: Option<u64>) -> (bool, Option<u64>) {
    (intake.is_none(), intake)
}

/// Sort `runs` into intake order. The stated timestamp only makes a tie's
/// listing deterministic.
pub(super) fn sort(runs: &mut [Run<'_>]) {
    runs.sort_by(|a, b| {
        key(a.intake)
            .cmp(&key(b.intake))
            .then_with(|| collection_order(a.collection, b.collection))
    });
}

/// Whether `later` is not before `earlier` in intake order: after it, or
/// tied with it so nothing attests which came first.
pub(super) fn not_before(later: Option<u64>, earlier: Option<u64>) -> bool {
    key(later) >= key(earlier)
}

/// Whether the run at `index` has no attested place in `runs`: it shares
/// its intake position with another run, or a run on one side of it states a
/// timestamp on the other side.
pub(super) fn unattested(runs: &[Run<'_>], index: usize) -> bool {
    let Some(run) = runs.get(index) else {
        return false;
    };
    runs.iter().enumerate().any(|(other, candidate)| {
        other != index
            && (candidate.intake == run.intake
                || stated_disagrees(other.cmp(&index), candidate.collection, run.collection))
    })
}

/// Whether the stated timestamps of `other` and `run` contradict `other`'s
/// intake position relative to `run`.
fn stated_disagrees(
    position: Ordering,
    other: &MeasurementCollection,
    run: &MeasurementCollection,
) -> bool {
    let stated = stated_cmp(other, run);
    matches!(
        (position, stated),
        (Ordering::Less, Ordering::Greater) | (Ordering::Greater, Ordering::Less)
    )
}

/// The stated timestamps compared: as instants when both parse, otherwise as
/// text, the same fallback `collection_order` uses.
fn stated_cmp(a: &MeasurementCollection, b: &MeasurementCollection) -> Ordering {
    Rfc3339DateTime::parse(a.timestamp.as_str())
        .ok()
        .zip(Rfc3339DateTime::parse(b.timestamp.as_str()).ok())
        .map_or_else(
            || a.timestamp.as_str().cmp(b.timestamp.as_str()),
            |(left, right)| left.epoch_millis().cmp(&right.epoch_millis()),
        )
}
