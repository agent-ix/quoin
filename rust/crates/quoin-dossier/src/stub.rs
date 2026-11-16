// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Stub generators, compiled only under the `test-stubs` feature (PLAT-1614).
//!
//! They let the command be driven end to end before any real document exists.
//! They read no record and never ship.

use crate::error::SourceFailure;
use crate::gap::{Gap, GapKind};
use crate::registry::{DocumentSpec, Generated, Generator, Inputs};

/// Which behaviour a stub exhibits.
#[derive(Debug, Clone, Copy)]
enum Behaviour {
    Complete,
    OneGap,
    TwoGapsOutOfOrder,
    SourceFails,
}

struct Stub {
    spec: DocumentSpec,
    behaviour: Behaviour,
}

impl Generator for Stub {
    fn spec(&self) -> DocumentSpec {
        self.spec
    }

    fn generate(&self, inputs: &Inputs) -> Result<Generated, SourceFailure> {
        let gaps = match self.behaviour {
            Behaviour::Complete => Vec::new(),
            Behaviour::OneGap => vec![
                Gap::new(
                    GapKind::NoAcceptanceCriteria,
                    "FR-1",
                    "has no acceptance criterion",
                )
                .for_requirement("FR-1"),
            ],
            // Deliberately not in gap order: the writer must sort them.
            Behaviour::TwoGapsOutOfOrder => vec![
                Gap::new(GapKind::NoEvidence, "FR-100-AC-10", "no evidence")
                    .for_requirement("FR-100")
                    .for_criterion("FR-100-AC-10"),
                Gap::new(GapKind::NoEvidence, "FR-100-AC-2", "no evidence")
                    .for_requirement("FR-100")
                    .for_criterion("FR-100-AC-2"),
                Gap::new(
                    GapKind::NoAcceptanceCriteria,
                    "FR-99",
                    "has no acceptance criterion",
                )
                .for_requirement("FR-99"),
                Gap::new(
                    GapKind::NoClauseBinding,
                    "this document",
                    "no clause binding given",
                ),
            ],
            Behaviour::SourceFails => {
                return Err(SourceFailure {
                    source: "requirements".to_owned(),
                    detail: "stub source is unreadable".to_owned(),
                });
            }
        };
        let as_of = inputs
            .as_of
            .as_deref()
            .map_or_else(String::new, |instant| format!("\nAs of {instant}.\n"));
        Ok(Generated {
            body: format!("# {}\n{as_of}", self.spec.title),
            document_version: None,
            gaps,
        })
    }
}

/// The stub generators, in registry order.
pub(crate) fn stubs() -> Vec<Box<dyn Generator>> {
    let stub = |kind, id, title, needs_as_of, behaviour| -> Box<dyn Generator> {
        Box::new(Stub {
            spec: DocumentSpec {
                kind,
                id,
                title,
                needs_as_of,
            },
            behaviour,
        })
    };
    vec![
        stub("stub-a", "DOS-STUB-A", "Stub A", false, Behaviour::Complete),
        stub("stub-b", "DOS-STUB-B", "Stub B", false, Behaviour::OneGap),
        stub(
            "stub-gaps",
            "DOS-STUB-GAPS",
            "Stub gaps",
            false,
            Behaviour::TwoGapsOutOfOrder,
        ),
        stub(
            "stub-fail",
            "DOS-STUB-FAIL",
            "Stub fail",
            false,
            Behaviour::SourceFails,
        ),
        stub(
            "stub-time",
            "DOS-STUB-TIME",
            "Stub time",
            true,
            Behaviour::Complete,
        ),
    ]
}
