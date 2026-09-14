// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The filesystem boundary for the governed graph portfolio.
//!
//! Ports `buildGovernedGraphPortfolio` (`src/measurement/graph-portfolio-load.ts`),
//! quoin#477. [`crate::build`] is the pure projection; this is the only module
//! that decides *which bytes* it is given.
//!
//! # Everything read here is read by something that already owns it
//!
//! Four reads happen for one repository and this module performs none of them:
//! the collections are [`quoin_measurement::read_measurement_collection_results`]'s,
//! the plans are [`quoin_measurement::load_measurement_plans`]'s, the
//! ungoverned portfolio entry is
//! [`quoin_measurement::portfolio::build_portfolio_report_from_collections`]'s,
//! and the structural graph is [`quoin_graph_analysis`]'s behind
//! [`crate::structural`]. What is written here is the *order* they happen in
//! and the three refusals the retained loader synthesises itself.
//!
//! # Mapping validation is deliberately first
//!
//! `graph-portfolio-load.ts:22` says so, and [`crate::mapping`]'s own module
//! documentation says why: a caller who mapped one repository to two different
//! graph exports has said something contradictory, and discovering that after
//! half the repositories have been walked would mean a refusal whose side
//! effects had already happened.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use quoin_measurement::portfolio::{
    PortfolioCollectionSnapshot, PortfolioRepositoryReport, RepositoryStatus,
    build_portfolio_report_from_collections,
};
use quoin_measurement::source::DiskMeasurement;
use quoin_measurement::store::{collection_order, measurements_root};
use quoin_measurement::types::collection::MeasurementCollection;
use quoin_measurement::{PlanLoadOptions, Rfc3339DateTime, load_measurement_plans};

use crate::availability::CollectionUnavailability;
use crate::build::build_governed_graph_portfolio_from;
use crate::error::{GraphAdapterError, Result};
use crate::input::{GraphCollectionRead, GraphPortfolioRepositoryInput};
use crate::mapping::{
    GraphPortfolioMappingOptions, ResolvedMapping, parse_graph_portfolio_mappings,
};
use crate::report::GovernedGraphPortfolioReport;
use crate::structural::load_structural_graph;

/// The sentence a collection whose timestamp no instant grammar reads is
/// refused with. `graph-portfolio-load.ts:36`.
const NOT_AN_INSTANT: &str = "collection timestamp is not a valid instant";

/// Read every named repository and build the governed graph portfolio.
///
/// `buildGovernedGraphPortfolio` (`graph-portfolio-load.ts:20-93`).
///
/// # Errors
///
/// The four mapping refusals [`parse_graph_portfolio_mappings`] raises, and
/// [`crate::GraphAdapterErrorCode::Measurement`] when a readable repository's
/// measurement plans cannot be loaded — which is the one read the retained
/// loader also lets throw rather than reporting as a gap
/// (`graph-portfolio-load.ts:80`).
pub fn build_governed_graph_portfolio(
    locations: &[PathBuf],
    options: &GraphPortfolioMappingOptions,
) -> Result<GovernedGraphPortfolioReport> {
    // Mapping validation is deliberately first: conflicts cannot trigger reads.
    let mappings = parse_graph_portfolio_mappings(locations, options)?;

    let reads: BTreeMap<PathBuf, Vec<GraphCollectionRead>> = mappings
        .iter()
        .map(|mapping| {
            (
                mapping.root().to_path_buf(),
                collection_reads(mapping.root()),
            )
        })
        .collect();

    let portfolio = build_portfolio_report_from_collections(
        &mappings
            .iter()
            .map(|mapping| PortfolioCollectionSnapshot {
                root: mapping.root().to_path_buf(),
                collections: readable(reads.get(mapping.root())),
            })
            .collect::<Vec<_>>(),
    );
    let by_root: BTreeMap<&str, &PortfolioRepositoryReport> = portfolio
        .repositories
        .iter()
        .map(|repository| (repository.root.as_str(), repository))
        .collect();

    let inputs = mappings
        .iter()
        .map(|mapping| repository_input(mapping, &by_root, reads.get(mapping.root())))
        .collect::<Result<Vec<_>>>()?;

    let report = build_governed_graph_portfolio_from(&inputs)?;
    Ok(GovernedGraphPortfolioReport {
        // `graph-portfolio-load.ts:89-92` overwrites the projection's own
        // newest timestamp with the ungoverned portfolio's, which is computed
        // over the same latest collections but before the governed view
        // sanitises any of them.
        newest_collection_timestamp: portfolio.newest_collection_timestamp,
        ..report
    })
}

/// One repository's inputs to the governed view (`graph-portfolio-load.ts:71-88`).
fn repository_input(
    mapping: &ResolvedMapping,
    by_root: &BTreeMap<&str, &PortfolioRepositoryReport>,
    reads: Option<&Vec<GraphCollectionRead>>,
) -> Result<GraphPortfolioRepositoryInput> {
    let root = mapping.root();
    // `graph-portfolio-load.ts:74-77` throws "portfolio omitted resolved
    // repository" here. It cannot fire: every root handed to the portfolio
    // came from this same list and both sides resolve it with
    // `quoin_measurement::portfolio::location::resolve_against`. The refusal
    // is kept rather than unwrapped because a missing entry would otherwise be
    // a panic in a library. See `DIVERGENCE.md` §10.
    let base = by_root
        .get(root.to_string_lossy().as_ref())
        .ok_or_else(|| {
            GraphAdapterError::new(
                crate::error::GraphAdapterErrorCode::Measurement,
                format!("portfolio omitted resolved repository {}", root.display()),
            )
        })?;
    Ok(GraphPortfolioRepositoryInput {
        portfolio: (*base).clone(),
        // Governance fields are loaded again for a readable repository: the
        // portfolio entry holds only ACTIVE plans and drops `owner` and
        // `action`, and the governed view reports both.
        plans: if base.status == RepositoryStatus::Readable {
            load_measurement_plans(
                &DiskMeasurement::new(root),
                PlanLoadOptions {
                    include_governance: true,
                },
            )?
        } else {
            base.plans.clone()
        },
        collections: reads.cloned().unwrap_or_default(),
        graph: load_structural_graph(mapping),
    })
}

/// Every attempt to read one repository's collections
/// (`graph-portfolio-load.ts:24-55`).
///
/// The retained `try` is around the whole directory read, so a repository
/// whose store cannot be listed is **one** refusal naming the repository root,
/// not one per file.
fn collection_reads(root: &Path) -> Vec<GraphCollectionRead> {
    let results =
        match quoin_measurement::read_measurement_collection_results(&DiskMeasurement::new(root)) {
            Ok(results) => results,
            Err(error) => {
                return vec![GraphCollectionRead::refused(
                    root.to_string_lossy().into_owned(),
                    Some(CollectionUnavailability::Unreadable),
                    Some(error.to_string()),
                )];
            }
        };
    results
        .into_iter()
        .map(|result| {
            // `readMeasurementCollectionResults` names each read by its whole
            // path (`store.ts:84`), and this projection renders that name.
            let path = measurements_root(root)
                .join(&result.path)
                .to_string_lossy()
                .into_owned();
            match result.collection {
                Err(error) => GraphCollectionRead::refused(
                    path,
                    Some(CollectionUnavailability::Unreadable),
                    Some(error.to_string()),
                ),
                Ok(collection) => readable_or_undated(path, collection),
            }
        })
        .collect()
}

/// A read collection, unless its timestamp is not an instant
/// (`graph-portfolio-load.ts:30-41`).
///
/// The retained check is `!Number.isFinite(Date.parse(timestamp))`, read here
/// by [`Rfc3339DateTime`] — this workspace's one instant grammar. The
/// difference between the two grammars is `DIVERGENCE.md` §4, which this call
/// site widens rather than re-declaring.
fn readable_or_undated(path: String, collection: MeasurementCollection) -> GraphCollectionRead {
    if Rfc3339DateTime::parse(collection.timestamp.as_str()).is_ok() {
        GraphCollectionRead::read(path, collection)
    } else {
        let reason = format!("{path}: {NOT_AN_INSTANT}");
        GraphCollectionRead::refused(
            path,
            Some(CollectionUnavailability::Unreadable),
            Some(reason),
        )
    }
}

/// The collections that were read, in the order the portfolio wants them
/// (`graph-portfolio-load.ts:59-66`).
///
/// The comparison is `compareInstants(timestamp) || compare(collectionId)`,
/// which is [`collection_order`] — `store.ts:96-102`'s own, reached rather
/// than re-spelled.
fn readable(reads: Option<&Vec<GraphCollectionRead>>) -> Vec<MeasurementCollection> {
    let mut collections: Vec<MeasurementCollection> = reads
        .into_iter()
        .flatten()
        .filter_map(GraphCollectionRead::collection)
        .cloned()
        .collect();
    collections.sort_by(collection_order);
    collections
}
