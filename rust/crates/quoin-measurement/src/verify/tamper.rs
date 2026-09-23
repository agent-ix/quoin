// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The checker's tamper findings from the caller's git-derived facts
//! (PLAT-985).
//!
//! Split from [`super`] so the checker stays under this crate's module-size
//! ceiling.

use super::{Finding, Reason, Run, TamperFacts, id_of, plan_level};

/// The plan-level and per-run findings the caller's git-derived
/// [`TamperFacts`] contribute, independent of whether the plan has a rule to
/// evaluate at all.
///
/// A forged run is evidence of tampering in its own data, so it is reported
/// whether the run is the candidate or history, the same as
/// [`Reason::carries_from_history`]'s existing pair. Deleted and edited
/// collections come attributed to the plan by the caller, not read off the
/// runs, since tampering can remove a run from this plan's runs entirely.
pub(super) fn findings(runs: &[Run<'_>], tamper: TamperFacts<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    for id in tamper.deleted_collections {
        findings.push(Finding {
            reason: Reason::CollectionDeleted,
            collection_id: Some(id.clone()),
            dimensions: None,
        });
    }
    for id in tamper.edited_collections {
        findings.push(Finding {
            reason: Reason::CollectionEdited,
            collection_id: Some(id.clone()),
            dimensions: None,
        });
    }
    if tamper.definition_changed_without_version_bump {
        findings.push(plan_level(Reason::DefinitionChangedWithoutVersionBump));
    }
    for run in runs {
        if run.apparatus_forged {
            findings.push(Finding {
                reason: Reason::ApparatusForged,
                collection_id: Some(id_of(run.collection)),
                dimensions: None,
            });
        }
    }
    findings
}
