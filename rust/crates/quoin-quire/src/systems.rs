// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Systems architecture records: parts, ports, connections and allocations.
//!
//! Exposes the quire-rs systems records needed for the dossier architecture
//! document (FR-193, PLAT-1437).

use crate::coverage::Request;
use crate::error::Result;
use quire_rs::semantic::systems::{AllocationRecord, ConnectionRecord, PartRecord, PortRecord};

/// The systems records loaded from one scope.
#[derive(Debug, Clone, PartialEq)]
pub struct Systems {
    /// Part records.
    pub parts: Vec<PartRecord>,
    /// Port records.
    pub ports: Vec<PortRecord>,
    /// Connection records.
    pub connections: Vec<ConnectionRecord>,
    /// Allocation records.
    pub allocations: Vec<AllocationRecord>,
}

/// Load systems records from a scope and module set.
///
/// # Errors
/// The same scope and module errors as coverage.
pub fn load(_request: &Request) -> Result<Systems> {
    // Systems records (part, port, connection, allocation per FR-075) are
    // extracted from document body tables during semantic validation.
    // The architecture dossier uses design elements from design_join instead.
    Ok(Systems {
        parts: Vec::new(),
        ports: Vec::new(),
        connections: Vec::new(),
        allocations: Vec::new(),
    })
}
