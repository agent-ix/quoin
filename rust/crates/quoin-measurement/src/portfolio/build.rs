// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Reading a portfolio of repositories into one report.
//!
//! Ports `buildPortfolioReport`, `buildPortfolioReportFromCollections`,
//! `finalizePortfolioReport`, `loadRepository`, `markUnreadable`,
//! `emptyRepository` and `collectionRef` (`portfolio.ts:77-140,223-331`).
//!
//! # Nothing here returns an error
//!
//! `portfolio.ts:235-279` wraps every read of a repository in one `try`, and a
//! throw becomes that repository's `status: "unreadable"` rather than the
//! walk's failure — which is the whole point of a portfolio view: one
//! unreadable repository must not hide the other eleven. So the refusals this
//! crate raises are caught here in exactly one place, [`loaded`], and become
//! that repository's stated error.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use quoin_store::json::order::cmp_utf16;

use crate::date_time::Rfc3339DateTime;
use crate::error::MeasurementError;
use crate::intervention::intake::read_intervention_records;
use crate::operational::read::read_operational_records;
use crate::plans::{PlanLoadOptions, load_measurement_plans};
use crate::portfolio::location::{LocationState, display_name, probe, resolve};
use crate::portfolio::types::{
    DAY_MS, PortfolioCollectionRef, PortfolioCollectionSnapshot, PortfolioComparison,
    PortfolioReport, PortfolioRepositoryReport, RepositoryStatus, Staleness, StoreState,
};
use crate::profiles::load_active_assurance_profiles;
use crate::report::build::build_measurement_report_from;
use crate::source::DiskMeasurement;
use crate::store::paths::{measurement_path, measurements_root};
use crate::store::read::read_measurement_collections;
use crate::types::collection::MeasurementCollection;
use crate::types::ids::CollectionId;
use crate::types::plan::LifecycleStatus;

/// Read every named location and build the portfolio report.
///
/// `buildPortfolioReport` (`portfolio.ts:78-83`). Locations are resolved,
/// de-duplicated and ordered before anything is read, so two spellings of one
/// repository are one entry.
#[must_use]
pub fn build_portfolio_report(locations: &[PathBuf]) -> PortfolioReport {
    let mut roots: Vec<PathBuf> = Vec::new();
    for location in locations {
        let root = resolve(location);
        if !roots.contains(&root) {
            roots.push(root);
        }
    }
    sort_roots(&mut roots);
    finalize(
        roots
            .iter()
            .map(|root| load_repository(root, None))
            .collect(),
    )
}

/// Build the same view from a caller's already-read snapshots.
///
/// `buildPortfolioReportFromCollections` (`portfolio.ts:86-100`). The retained
/// function keys a `Map` by resolved root, so two snapshots of one repository
/// collapse to the last one given.
#[must_use]
pub fn build_portfolio_report_from_collections(
    snapshots: &[PortfolioCollectionSnapshot],
) -> PortfolioReport {
    let mut by_root: BTreeMap<PathBuf, &[MeasurementCollection]> = BTreeMap::new();
    for snapshot in snapshots {
        by_root.insert(resolve(&snapshot.root), &snapshot.collections);
    }
    let mut roots: Vec<PathBuf> = by_root.keys().cloned().collect();
    sort_roots(&mut roots);
    finalize(
        roots
            .iter()
            .map(|root| load_repository(root, by_root.get(root).copied()))
            .collect(),
    )
}

/// `[...].sort(compare)` (`portfolio.ts:369-371`) over resolved roots.
///
/// `compare` is JavaScript's `<` on strings, which orders by UTF-16 code unit;
/// [`cmp_utf16`] is that order, and is the store's rather than a second one.
fn sort_roots(roots: &mut [PathBuf]) {
    roots.sort_by(|left, right| cmp_utf16(&left.to_string_lossy(), &right.to_string_lossy()));
}

/// One repository, read or refused. `loadRepository` (`portfolio.ts:223-280`).
fn load_repository(
    root: &Path,
    provided: Option<&[MeasurementCollection]>,
) -> PortfolioRepositoryReport {
    let empty = |status: RepositoryStatus, store: StoreState| PortfolioRepositoryReport {
        name: display_name(root),
        root: root.to_string_lossy().into_owned(),
        status,
        store,
        profiles: Vec::new(),
        plans: Vec::new(),
        measurements: None,
        latest_collection: None,
        comparison: None,
        staleness: Staleness::NotComputed,
    };
    match probe(root) {
        LocationState::Missing => empty(
            RepositoryStatus::Missing(format!("{}: repository does not exist", root.display())),
            StoreState::Missing,
        ),
        LocationState::NotADirectory => empty(
            RepositoryStatus::Unreadable(format!(
                "{}: repository location is not a directory",
                root.display()
            )),
            StoreState::Unreadable,
        ),
        LocationState::Unreadable(error) => {
            empty(RepositoryStatus::Unreadable(error), StoreState::Unreadable)
        }
        LocationState::Directory => loaded(root, provided).unwrap_or_else(|error| {
            empty(
                RepositoryStatus::Unreadable(error.to_string()),
                StoreState::Unreadable,
            )
        }),
    }
}

/// The body of the retained `try` (`portfolio.ts:244-270`).
fn loaded(
    root: &Path,
    provided: Option<&[MeasurementCollection]>,
) -> Result<PortfolioRepositoryReport, MeasurementError> {
    let source = DiskMeasurement::new(root);
    let profiles = load_active_assurance_profiles(&source)?;
    let plans = load_measurement_plans(&source, PlanLoadOptions::default())?
        .into_iter()
        .filter(|plan| plan.status == LifecycleStatus::Active)
        .collect::<Vec<_>>();
    let collections = match provided {
        Some(given) => given.to_vec(),
        None => read_measurement_collections(&source)?,
    };
    let interventions = read_intervention_records(&source)?;
    let operational = read_operational_records(root)?
        .into_iter()
        .map(|valid| valid.record().clone())
        .collect::<Vec<_>>();
    let measurements =
        build_measurement_report_from(root, &plans, &collections, &interventions, &operational)?;

    let latest = collections.last();
    let previous = collections
        .len()
        .checked_sub(2)
        .and_then(|index| collections.get(index));
    Ok(PortfolioRepositoryReport {
        name: display_name(root),
        root: root.to_string_lossy().into_owned(),
        status: RepositoryStatus::Readable,
        store: if measurements_root(root).exists() {
            if collections.is_empty() {
                StoreState::Empty
            } else {
                StoreState::Present
            }
        } else {
            StoreState::Missing
        },
        profiles,
        plans,
        measurements: Some(measurements),
        latest_collection: latest
            .map(|found| collection_ref(root, found))
            .transpose()?,
        comparison: match (previous, latest) {
            (Some(before), Some(after)) => Some(PortfolioComparison {
                before: collection_ref(root, before)?,
                after: collection_ref(root, after)?,
                observations: crate::compare::compare_measurement_collections(before, after)?,
            }),
            _ => None,
        },
        staleness: Staleness::NotComputed,
    })
}

/// `collectionRef` (`portfolio.ts:315-331`).
fn collection_ref(
    repo: &Path,
    collection: &MeasurementCollection,
) -> Result<PortfolioCollectionRef, MeasurementError> {
    let id = CollectionId::parse(collection.collection_id.as_str())?;
    Ok(PortfolioCollectionRef {
        id: collection.collection_id.as_str().to_owned(),
        path: measurement_path(repo, &id).to_string_lossy().into_owned(),
        timestamp: collection.timestamp.as_str().to_owned(),
        source_revision: collection.source_revision.as_str().to_owned(),
        tool_identity: collection.tool_identity.as_str().to_owned(),
        tool_version: collection.tool_version.as_str().to_owned(),
        config_digest: collection.config_digest.as_str().to_owned(),
        // `? { corpusRevision } : {}` — a stated empty string is as absent as
        // an absent member, because both are falsy.
        corpus_revision: collection
            .corpus_revision
            .clone()
            .filter(|revision| !revision.is_empty()),
    })
}

/// `finalizePortfolioReport` (`portfolio.ts:102-140`).
fn finalize(mut repositories: Vec<PortfolioRepositoryReport>) -> PortfolioReport {
    for repository in &mut repositories {
        let unreadable = repository
            .latest_collection
            .as_ref()
            .filter(|latest| epoch_millis(&latest.timestamp).is_none())
            .map(|latest| format!("{}: collection timestamp is not a valid date", latest.path));
        if let Some(error) = unreadable {
            mark_unreadable(repository, error);
        }
    }

    let mut timestamps: Vec<&str> = repositories
        .iter()
        .filter_map(|repository| {
            repository
                .latest_collection
                .as_ref()
                .map(|latest| latest.timestamp.as_str())
        })
        .collect();
    timestamps.sort_by(|left, right| {
        epoch_millis(left)
            .cmp(&epoch_millis(right))
            .then_with(|| cmp_utf16(left, right))
    });
    let newest_collection_timestamp = timestamps.last().map(|&found| found.to_owned());
    let newest = newest_collection_timestamp
        .as_deref()
        .and_then(epoch_millis);

    for repository in &mut repositories {
        let age = repository
            .latest_collection
            .as_ref()
            .and_then(|latest| epoch_millis(&latest.timestamp))
            .zip(newest)
            .map(|(timestamp, newest)| (newest - timestamp).max(0) / DAY_MS);
        if let Some((age_days, relative_to)) = age.zip(newest_collection_timestamp.clone()) {
            repository.staleness = Staleness::of(age_days, relative_to);
        }
    }

    PortfolioReport {
        newest_collection_timestamp,
        repositories,
    }
}

/// `markUnreadable` (`portfolio.ts:282-292`).
fn mark_unreadable(repository: &mut PortfolioRepositoryReport, error: String) {
    repository.status = RepositoryStatus::Unreadable(error);
    repository.store = StoreState::Unreadable;
    repository.measurements = None;
    repository.comparison = None;
    repository.latest_collection = None;
}

/// `Date.parse(timestamp)`, read by this crate's one instant reader.
///
/// `date_time.rs` is the crate's single RFC 3339 grammar and the Stage 6 plan
/// §5 forbids a seventh; `Date.parse` accepts more spellings than RFC 3339 and
/// the divergence is declared in `DIVERGENCE.md` §13.
fn epoch_millis(timestamp: &str) -> Option<i64> {
    Rfc3339DateTime::parse(timestamp)
        .ok()
        .map(|instant| instant.epoch_millis())
}
