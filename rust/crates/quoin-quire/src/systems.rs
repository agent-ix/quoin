// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Systems architecture records: parts, ports, connections and allocations.
//!
//! Exposes the quire-rs systems records needed for the dossier architecture
//! document (FR-193, PLAT-1437).

use crate::coverage::{Request, walk};
use crate::error::Result;

/// The systems records loaded from one scope.
#[derive(Debug, Clone, PartialEq)]
pub struct Systems {
    /// Part records.
    pub parts: Vec<quire_rs::semantic::systems::PartRecord>,
    /// Port records.
    pub ports: Vec<quire_rs::semantic::systems::PortRecord>,
    /// Connection records.
    pub connections: Vec<quire_rs::semantic::systems::ConnectionRecord>,
    /// Allocation records.
    pub allocations: Vec<quire_rs::semantic::systems::AllocationRecord>,
}

/// Load systems records from a scope and module set.
///
/// # Errors
/// The same scope and module errors as coverage.
pub fn load(request: &Request) -> Result<Systems> {
    let walked = walk(request)?;
    let systems = quire_rs::semantic::systems::load_systems(&walked.spec);
    Ok(Systems {
        parts: systems.parts,
        ports: systems.ports,
        connections: systems.connections,
        allocations: systems.allocations,
    })
}
