// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! MP-219: every frozen span-breadth label is replayed through the property
//! classifier, and the producer states the ids it examined and the ids whose
//! reviewed outcome the engine reproduced.
//!
//! The population is the label file itself, read at run time. A label whose
//! repository checkout is not supplied cannot be examined, so the replay
//! refuses rather than reporting a smaller denominator.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use quoin_quire::ids::ScopeRoot;
use quoin_quire::modules::ModuleSelection;
use quoin_quire::properties::{self, Criterion, Span};
use serde::Deserialize;

use super::{Probed, repository};

/// The frozen label file, protected apparatus of MP-219.
pub(crate) const LABELS: &str = "bench/span-breadth-v1-labels.json";

#[derive(Deserialize)]
struct LabelFile {
    labels: Vec<Label>,
}

#[derive(Deserialize)]
struct Label {
    id: String,
    repository: String,
    document: String,
    #[serde(rename = "rowId")]
    row_id: String,
    #[serde(rename = "normalizedStatement")]
    normalized_statement: String,
    #[serde(rename = "expectedSpans")]
    expected_spans: Option<ExpectedSpans>,
    #[serde(rename = "expectedRefusal")]
    expected_refusal: Option<String>,
}

#[derive(Deserialize)]
struct ExpectedSpans {
    domain: Option<LabelSpan>,
    precondition: Option<LabelSpan>,
    oracle: Option<LabelSpan>,
}

#[derive(Deserialize)]
struct LabelSpan {
    start: usize,
    end: usize,
    text: String,
}

/// Whether the classified `span` is the reviewed one; an unlabelled boundary
/// must be absent.
fn same(expected: Option<&LabelSpan>, span: Option<&Span>) -> bool {
    match (expected, span) {
        (None, None) => true,
        (Some(expected), Some(span)) => {
            (span.start, span.end, span.text.as_str())
                == (expected.start, expected.end, expected.text.as_str())
        }
        _ => false,
    }
}

/// The repositories the label file names that no checkout was supplied for.
#[derive(Debug)]
pub(crate) struct Unavailable(pub(crate) BTreeSet<String>);

fn normalized(text: &str) -> String {
    text.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

impl Label {
    /// Whether the classified criterion is the labelled statement and carries
    /// the reviewed outcome: the exact reviewed spans, or the justified refusal
    /// with no span at all.
    fn reproduced(&self, criterion: &Criterion) -> bool {
        if normalized(&criterion.statement) != normalized(&self.normalized_statement) {
            return false;
        }
        match (&self.expected_spans, &self.expected_refusal) {
            (Some(expected), None) => {
                same(expected.domain.as_ref(), criterion.domain.as_ref())
                    && same(
                        expected.precondition.as_ref(),
                        criterion.precondition.as_ref(),
                    )
                    && same(expected.oracle.as_ref(), criterion.oracle.as_ref())
            }
            (None, Some(refusal)) => {
                criterion.domain.is_none()
                    && criterion.precondition.is_none()
                    && criterion.oracle.is_none()
                    && criterion.signals.iter().any(|signal| signal == refusal)
            }
            _ => false,
        }
    }
}

/// Replay every label in `labels_json`. `roots` maps a label's `repository` to
/// its checkout. A label whose document or row is missing, whose statement
/// drifted, or whose outcome differs is examined and not matched.
pub(crate) fn replay(
    labels_json: &str,
    roots: &BTreeMap<String, PathBuf>,
) -> Result<Probed, Unavailable> {
    let file: LabelFile = serde_json::from_str(labels_json).expect("the label file parses");
    let missing: BTreeSet<String> = file
        .labels
        .iter()
        .filter(|label| !roots.contains_key(&label.repository))
        .map(|label| label.repository.clone())
        .collect();
    if !missing.is_empty() {
        return Err(Unavailable(missing));
    }

    let mut classified: BTreeMap<(String, String), Vec<Criterion>> = BTreeMap::new();
    let mut probed = Probed {
        examined: BTreeSet::new(),
        matched: BTreeSet::new(),
    };
    for label in &file.labels {
        let key = (label.repository.clone(), label.document.clone());
        let criteria = classified
            .entry(key)
            .or_insert_with(|| classify(&roots[&label.repository], &label.document));
        probed.examined.insert(label.id.clone());
        let found = criteria
            .iter()
            .find(|criterion| criterion.row_id.as_deref() == Some(label.row_id.as_str()));
        if found.is_some_and(|criterion| label.reproduced(criterion)) {
            probed.matched.insert(label.id.clone());
        }
    }
    Ok(probed)
}

/// The criteria `document` classifies to under `root`'s own modules; none when
/// the document no longer classifies.
fn classify(root: &Path, document: &str) -> Vec<Criterion> {
    let scope = ScopeRoot::open(root).expect("the checkout opens as a scope");
    let outcome = properties::classify(&properties::Request {
        scope,
        modules: ModuleSelection::ScopeOrAmbient,
        documents: vec![PathBuf::from(document)],
        archetype: None,
    })
    .expect("classification does not fail over one document");
    outcome
        .report
        .documents
        .into_iter()
        .flat_map(|document| document.criteria)
        .collect()
}

/// The label file of this checkout.
pub(crate) fn labels_json() -> String {
    std::fs::read_to_string(repository().join(LABELS)).expect("the label file is readable")
}
