// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Which suites carry which live obligations (`analysis.ts:154`).

use std::collections::{BTreeMap, BTreeSet};

use crate::analysis::draft::{Draft, absent_from_export, obligation_owners};
use crate::ids::{ObligationId, SuiteId};
use crate::model::GraphAnalysisInput;
use crate::model::report::{FanOutAnalysis, FanOutRow, GraphGapKind, GraphView, OwnedObligation};

/// One suite's two buckets while the rows are being accumulated.
#[derive(Default)]
struct Row {
    live: BTreeSet<ObligationId>,
    unresolved: BTreeSet<ObligationId>,
}

/// Which suites carry which live obligations.
///
/// A suite appears when any binding names it, whether or not the export knows
/// the obligation — so a row can have an empty `obligations` and a non-empty
/// `unresolvedBindings`.
#[must_use]
pub fn analyze_fan_out(input: &GraphAnalysisInput) -> FanOutAnalysis {
    let mut draft = Draft::new(input, GraphView::FanOut);
    let Some(bindings) = input.bindings.available() else {
        return FanOutAnalysis {
            base: draft.not_computed(),
            rows: Vec::new(),
        };
    };
    let owners = obligation_owners(&input.assurance, false);
    let mut rows: BTreeMap<SuiteId, Row> = BTreeMap::new();
    for binding in bindings {
        let row = rows.entry(binding.suite.clone()).or_default();
        if owners.contains_key(&binding.obligation) {
            row.live.insert(binding.obligation.clone());
        } else {
            row.unresolved.insert(binding.obligation.clone());
            draft.gap(
                GraphGapKind::UnresolvedBinding,
                format!("{}:{}", binding.suite, binding.obligation),
                absent_from_export(binding.obligation.as_str()),
            );
        }
    }
    let rows = rows
        .into_iter()
        .map(|(suite, row)| FanOutRow {
            suite,
            obligations: row
                .live
                .iter()
                .map(|obligation| OwnedObligation {
                    obligation: obligation.clone(),
                    requirements: owners.get(obligation).cloned().unwrap_or_default(),
                })
                .collect(),
            obligation_count: row.live.len(),
            unresolved_bindings: row.unresolved.into_iter().collect(),
        })
        .collect();
    FanOutAnalysis {
        base: draft.finish(),
        rows,
    }
}
