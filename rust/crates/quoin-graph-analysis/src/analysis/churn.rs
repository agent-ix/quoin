// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Which obligations were re-affirmed, by whom (`analysis.ts:197`).

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use quoin_store::json::order::cmp_utf16;

use crate::analysis::draft::{Draft, obligation_owners};
use crate::ids::{Author, Commit, ObligationId, SuiteId};
use crate::model::report::{ChurnAnalysis, ChurnEvent, ChurnRow, GraphGapKind, GraphView};
use crate::model::{Binding, GraphAnalysisInput};

/// What makes two recorded affirmations the same event (`analysis.ts:223`).
///
/// The retained key is the four parts joined with `U+0000`, and a missing note
/// joins as the empty string — so an affirmation with `note: ""` and one with
/// no note at all are one event, and the first one seen decides whether the
/// member is emitted. Ordering this key is `eventOrder` (`analysis.ts:693`)
/// exactly: who, then commit, then the same `note ?? ""`.
#[derive(PartialEq, Eq)]
struct EventKey {
    who: Author,
    commit: Commit,
    note: String,
}

impl Ord for EventKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.who
            .cmp(&other.who)
            .then_with(|| self.commit.cmp(&other.commit))
            .then_with(|| cmp_utf16(&self.note, &other.note))
    }
}

impl PartialOrd for EventKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// One event while its suites are still being collected.
struct Event {
    note: Option<String>,
    suites: BTreeSet<SuiteId>,
}

/// Which obligations were re-affirmed, by whom.
///
/// Every live obligation gets a row, including the ones with no affirmation
/// history at all; the rows are ordered busiest-first so that history is what
/// the reader sees.
#[must_use]
pub fn analyze_churn(input: &GraphAnalysisInput) -> ChurnAnalysis {
    let mut draft = Draft::new(input, GraphView::Churn);
    let Some(bindings) = input.bindings.available() else {
        return ChurnAnalysis {
            base: draft.not_computed(),
            rows: Vec::new(),
        };
    };
    let owners = obligation_owners(&input.assurance, false);
    let mut suites: BTreeMap<&ObligationId, BTreeSet<SuiteId>> = BTreeMap::new();
    let mut events: BTreeMap<&ObligationId, BTreeMap<EventKey, Event>> = BTreeMap::new();
    for binding in bindings {
        if !owners.contains_key(&binding.obligation) {
            report_orphan_history(&mut draft, binding);
            continue;
        }
        suites
            .entry(&binding.obligation)
            .or_default()
            .insert(binding.suite.clone());
        let by_key = events.entry(&binding.obligation).or_default();
        for affirmation in binding.affirmations.iter().flatten() {
            by_key
                .entry(EventKey {
                    who: affirmation.who.clone(),
                    commit: affirmation.commit.clone(),
                    note: affirmation.note.clone().unwrap_or_default(),
                })
                .or_insert_with(|| Event {
                    note: affirmation.note.clone(),
                    suites: BTreeSet::new(),
                })
                .suites
                .insert(binding.suite.clone());
        }
    }
    let mut rows: Vec<ChurnRow> = input
        .assurance
        .obligations
        .iter()
        .map(|obligation| ObligationId::new(obligation.id.clone()))
        .filter(|id| owners.contains_key(id))
        .map(|id| row(&id, &owners, &suites, &events))
        .collect();
    rows.sort_by(|left, right| {
        right
            .event_count
            .cmp(&left.event_count)
            .then_with(|| left.obligation.cmp(&right.obligation))
    });
    ChurnAnalysis {
        base: draft.finish(),
        rows,
    }
}

fn report_orphan_history(draft: &mut Draft, binding: &Binding) {
    if binding.affirmations.as_ref().is_none_or(Vec::is_empty) {
        return;
    }
    draft.gap(
        GraphGapKind::UnresolvedBinding,
        format!("{}:{}", binding.suite, binding.obligation),
        format!(
            "affirmation history belongs to absent obligation {}",
            binding.obligation
        ),
    );
}

fn row(
    id: &ObligationId,
    owners: &crate::analysis::draft::Owners,
    suites: &BTreeMap<&ObligationId, BTreeSet<SuiteId>>,
    events: &BTreeMap<&ObligationId, BTreeMap<EventKey, Event>>,
) -> ChurnRow {
    let row_events: Vec<ChurnEvent> = events
        .get(id)
        .into_iter()
        .flatten()
        .map(|(key, event)| ChurnEvent {
            who: key.who.clone(),
            commit: key.commit.clone(),
            note: event.note.clone(),
            suites: event.suites.iter().cloned().collect(),
        })
        .collect();
    ChurnRow {
        obligation: id.clone(),
        requirements: owners.get(id).cloned().unwrap_or_default(),
        suites: suites
            .get(id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect(),
        event_count: row_events.len(),
        events: row_events,
    }
}
