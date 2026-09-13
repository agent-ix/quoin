// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The reverse walk: from each seed, the shortest route to every dependent
//! (`analysis.ts:558`).
//!
//! Dijkstra with a unit cost, which is a breadth-first search with a
//! tie-break. The tie-break is the whole reason the retained code carries a
//! heap rather than a queue: two routes of equal length must resolve the same
//! way every run, so the frontier is ordered by length, then by the route's
//! own text, then by the node. That order is total — two queued items compare
//! equal only if they are the same item — so the heap's own instability is not
//! observable.

use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeMap, BinaryHeap};

use quoin_store::json::order::cmp_utf16;

use crate::ids::{ArtifactId, RelationKind};
use crate::model::report::{ImpactPath, ImpactPathEdge};

/// One selected corpus edge, reduced to the three parts the walk reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CorpusEdge {
    /// The dependent.
    pub(crate) source: ArtifactId,
    /// What it depends on.
    pub(crate) target: ArtifactId,
    /// How.
    pub(crate) edge_type: RelationKind,
}

impl CorpusEdge {
    /// `corpusEdgeOrder` (`analysis.ts:682`).
    pub(crate) fn order(&self, other: &Self) -> Ordering {
        self.source
            .cmp(&other.source)
            .then_with(|| self.target.cmp(&other.target))
            .then_with(|| self.edge_type.cmp(&other.edge_type))
    }
}

/// A route, with the sort key the tie-break is decided on.
///
/// The key is carried rather than recomputed: the retained `pathKey` is called
/// from inside two comparators and so runs once per comparison, which is an
/// implementation detail and not an observable one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeyedPath {
    pub(crate) path: ImpactPath,
    key: String,
}

impl KeyedPath {
    /// `pathKey` (`analysis.ts:663`): the seed and every edge's three parts,
    /// joined with `U+0000`.
    fn new(path: ImpactPath) -> Self {
        let mut key = path.seed.as_str().to_owned();
        for edge in &path.edges {
            key.push('\u{0}');
            key.push_str(edge.source.as_str());
            key.push('\u{0}');
            key.push_str(edge.target.as_str());
            key.push('\u{0}');
            key.push_str(edge.relationship.as_str());
        }
        Self { path, key }
    }

    /// `betterPath` (`analysis.ts:652`): shorter wins, then lower key.
    fn better_than(&self, current: Option<&Self>) -> bool {
        let Some(current) = current else {
            return true;
        };
        match self.path.edges.len().cmp(&current.path.edges.len()) {
            Ordering::Equal => cmp_utf16(&self.key, &current.key).is_lt(),
            other => other.is_lt(),
        }
    }
}

/// One frontier entry.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Queued {
    node: ArtifactId,
    path: KeyedPath,
}

impl Ord for Queued {
    /// `queueOrder` (`analysis.ts:674`).
    fn cmp(&self, other: &Self) -> Ordering {
        self.path
            .path
            .edges
            .len()
            .cmp(&other.path.path.edges.len())
            .then_with(|| cmp_utf16(&self.path.key, &other.path.key))
            .then_with(|| self.node.cmp(&other.node))
    }
}

impl PartialOrd for Queued {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The shortest route from some seed to every requirement that reaches one.
///
/// "Reverse": the walk follows each edge from its target back to its source,
/// because the question is *what depends on this*, not what this depends on.
pub(crate) fn shortest_reverse_paths(
    seeds: &[ArtifactId],
    edges: &[CorpusEdge],
) -> BTreeMap<ArtifactId, KeyedPath> {
    let mut dependents: BTreeMap<&ArtifactId, Vec<&CorpusEdge>> = BTreeMap::new();
    for edge in edges {
        dependents.entry(&edge.target).or_default().push(edge);
    }
    for group in dependents.values_mut() {
        group.sort_by(|left, right| left.order(right));
    }

    let mut paths: BTreeMap<ArtifactId, KeyedPath> = BTreeMap::new();
    let mut queue: BinaryHeap<Reverse<Queued>> = BinaryHeap::new();
    let mut sorted_seeds = seeds.to_vec();
    sorted_seeds.sort();
    for seed in sorted_seeds {
        let candidate = KeyedPath::new(ImpactPath {
            seed: seed.clone(),
            edges: Vec::new(),
        });
        if candidate.better_than(paths.get(&seed)) {
            paths.insert(seed.clone(), candidate.clone());
            queue.push(Reverse(Queued {
                node: seed,
                path: candidate,
            }));
        }
    }

    while let Some(Reverse(current)) = queue.pop() {
        // `samePath` (`analysis.ts:659`): this entry was superseded by a
        // better route to the same node before it came off the frontier.
        if paths.get(&current.node).map(|held| held.key.as_str()) != Some(current.path.key.as_str())
        {
            continue;
        }
        for edge in dependents.get(&current.node).into_iter().flatten() {
            let mut extended = current.path.path.edges.clone();
            extended.push(ImpactPathEdge {
                source: edge.source.clone(),
                target: edge.target.clone(),
                relationship: edge.edge_type.clone(),
            });
            let candidate = KeyedPath::new(ImpactPath {
                seed: current.path.path.seed.clone(),
                edges: extended,
            });
            if candidate.better_than(paths.get(&edge.source)) {
                paths.insert(edge.source.clone(), candidate.clone());
                queue.push(Reverse(Queued {
                    node: edge.source.clone(),
                    path: candidate,
                }));
            }
        }
    }
    paths
}
