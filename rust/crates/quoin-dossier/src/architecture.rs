// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Architecture document generator (FR-193, PLAT-1437).
//!
//! TODO(PLAT-1437): This is a minimal stub implementation pending the correct
//! quire-rs API for accessing systems records and design join. The generator
//! compiles and has test fixtures but needs proper integration with quire-rs.

use crate::error::SourceFailure;
use crate::gap::{Gap, GapKind};
use crate::registry::{DocumentSpec, Generated, Generator, Inputs};

/// The Architecture generator.
pub struct ArchitectureGenerator;

impl Generator for ArchitectureGenerator {
    fn spec(&self) -> DocumentSpec {
        DocumentSpec {
            kind: "architecture",
            id: "DOS-ARCH",
            title: "Architecture",
            needs_as_of: false,
        }
    }

    fn generate(&self, _inputs: &Inputs) -> Result<Generated, SourceFailure> {
        // TODO(PLAT-1437): Implement actual architecture generation using quire-rs API.
        // This is a stub implementation to allow compilation and testing of the command frame.

        let mut gaps = Vec::new();
        let mut body = String::from("# Architecture\n\n");

        body.push_str("## Parts\n\nTODO: Load from design_relations\n\n");
        body.push_str("## Connections\n\nTODO: Load from design_relations\n\n");
        body.push_str("## Decisions\n\nTODO: Load ADRs\n\n");
        body.push_str(
            "## Requirements without a design element\n\nTODO: Compute from design join\n\n",
        );

        gaps.push(Gap::new(
            GapKind::DesignRelationsUnavailable,
            "this document",
            "architecture generation not yet implemented",
        ));

        Ok(Generated { body, gaps })
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    #[trace("FR-193")]
    #[test]
    fn architecture_generator_has_correct_spec() {
        let generator = ArchitectureGenerator;
        let spec = generator.spec();
        assert_eq!(spec.kind, "architecture");
        assert_eq!(spec.id, "DOS-ARCH");
        assert_eq!(spec.title, "Architecture");
        assert!(!spec.needs_as_of);
    }
}
