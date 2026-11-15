// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Systems records: parts, ports, connections and allocations (FR-193,
//! PLAT-1437).
//!
//! The records are the rows of the four systems tables of quire-rs FR-075,
//! recognised by their exact header rows, so no section heading is assumed.
//! Only the first table of each section is read, as [`crate::records`] reads
//! documents. A cell is carried verbatim; nothing here parses or resolves it.

use crate::coverage::Request;
use crate::error::Result;
use crate::records::{self, Document};

/// A `Owner | Declared Type | Multiplicity` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    /// The document that declares the row.
    pub document: String,
    /// The owning declaration.
    pub owner: String,
    /// The part's declared type.
    pub declared_type: String,
    /// How many of the part the owner holds.
    pub multiplicity: String,
}

/// A `Owner | Direction | Interface | Multiplicity` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Port {
    /// The document that declares the row.
    pub document: String,
    /// The owning declaration.
    pub owner: String,
    /// The port direction as written.
    pub direction: String,
    /// The interface type.
    pub interface: String,
    /// How many of the port the owner holds.
    pub multiplicity: String,
}

/// A `Source | Source Multiplicity | Target | Target Multiplicity | Direction`
/// row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    /// The document that declares the row.
    pub document: String,
    /// The source end.
    pub source: String,
    /// The source end's multiplicity.
    pub source_multiplicity: String,
    /// The target end.
    pub target: String,
    /// The target end's multiplicity.
    pub target_multiplicity: String,
    /// The flow direction as written.
    pub direction: String,
}

/// A `Source | Target` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Allocation {
    /// The document that declares the row.
    pub document: String,
    /// The allocated element.
    pub source: String,
    /// The element it is allocated to.
    pub target: String,
}

/// The systems records loaded from one scope.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Systems {
    /// Part records.
    pub parts: Vec<Part>,
    /// Port records.
    pub ports: Vec<Port>,
    /// Connection records.
    pub connections: Vec<Connection>,
    /// Allocation records.
    pub allocations: Vec<Allocation>,
}

const PART_HEADERS: [&str; 3] = ["Owner", "Declared Type", "Multiplicity"];
const PORT_HEADERS: [&str; 4] = ["Owner", "Direction", "Interface", "Multiplicity"];
const CONNECTION_HEADERS: [&str; 5] = [
    "Source",
    "Source Multiplicity",
    "Target",
    "Target Multiplicity",
    "Direction",
];
const ALLOCATION_HEADERS: [&str; 2] = ["Source", "Target"];

/// Load the systems records of every document below the request's scope, in
/// path order and then row order.
///
/// # Errors
/// The scope and module errors of [`records::read`].
pub fn load(request: &Request) -> Result<Systems> {
    let corpus = records::read(request.scope.as_path(), &request.modules, &[])?;
    Ok(extract(&corpus.documents))
}

fn extract(documents: &[Document]) -> Systems {
    let mut systems = Systems::default();
    for document in documents {
        for table in document.tables.values() {
            let headers: Vec<&str> = table.headers.iter().map(|header| header.trim()).collect();
            let document_id = &document.id;
            for row in &table.rows {
                let cells: Vec<String> = row.iter().map(|cell| cell.trim().to_owned()).collect();
                let cell = |index: usize| cells.get(index).cloned().unwrap_or_default();
                if headers == PART_HEADERS {
                    systems.parts.push(Part {
                        document: document_id.clone(),
                        owner: cell(0),
                        declared_type: cell(1),
                        multiplicity: cell(2),
                    });
                } else if headers == PORT_HEADERS {
                    systems.ports.push(Port {
                        document: document_id.clone(),
                        owner: cell(0),
                        direction: cell(1),
                        interface: cell(2),
                        multiplicity: cell(3),
                    });
                } else if headers == CONNECTION_HEADERS {
                    systems.connections.push(Connection {
                        document: document_id.clone(),
                        source: cell(0),
                        source_multiplicity: cell(1),
                        target: cell(2),
                        target_multiplicity: cell(3),
                        direction: cell(4),
                    });
                } else if headers == ALLOCATION_HEADERS {
                    systems.allocations.push(Allocation {
                        document: document_id.clone(),
                        source: cell(0),
                        target: cell(1),
                    });
                }
            }
        }
    }
    systems
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
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use serde_json::Map;

    use super::*;
    use crate::records::{Contract, Table};

    fn table(headers: &[&str], rows: &[&[&str]]) -> Table {
        Table {
            headers: headers.iter().map(|header| (*header).to_owned()).collect(),
            rows: rows
                .iter()
                .map(|row| row.iter().map(|cell| (*cell).to_owned()).collect())
                .collect(),
        }
    }

    fn document(id: &str, tables: Vec<(&str, Table)>) -> Document {
        Document {
            path: PathBuf::from(format!("{id}.md")),
            id: id.to_owned(),
            kind: "Component".to_owned(),
            frontmatter: Map::new(),
            edges: Vec::new(),
            tables: tables
                .into_iter()
                .map(|(name, table)| (name.to_owned(), table))
                .collect::<BTreeMap<_, _>>(),
            sections: BTreeMap::new(),
            raw: String::new(),
            contract: Contract::NotChecked,
        }
    }

    #[test]
    fn the_four_systems_tables_are_read_by_their_header_rows() {
        let documents = vec![document(
            "COMP-1",
            vec![
                ("Parts", table(&PART_HEADERS, &[&["COMP-1", "Engine", "1"]])),
                (
                    "Ports",
                    table(&PORT_HEADERS, &[&["COMP-1", "out", "Telemetry", "0..*"]]),
                ),
                (
                    "Connections",
                    table(
                        &CONNECTION_HEADERS,
                        &[&["COMP-1.out", "1", "COMP-2.in", "1", "forward"]],
                    ),
                ),
                (
                    "Allocations",
                    table(&ALLOCATION_HEADERS, &[&["FR-1", "COMP-1"]]),
                ),
                ("Other", table(&["Name", "Value"], &[&["a", "b"]])),
            ],
        )];
        let systems = extract(&documents);
        assert_eq!(systems.parts.len(), 1);
        assert_eq!(systems.parts[0].declared_type, "Engine");
        assert_eq!(systems.ports[0].interface, "Telemetry");
        assert_eq!(systems.connections[0].direction, "forward");
        assert_eq!(systems.connections[0].target, "COMP-2.in");
        assert_eq!(systems.allocations[0].source, "FR-1");
        assert_eq!(systems.allocations[0].target, "COMP-1");
    }

    #[test]
    fn a_corpus_without_systems_tables_yields_no_records() {
        let documents = vec![document("X", vec![("Other", table(&["Name"], &[&["a"]]))])];
        assert_eq!(extract(&documents), Systems::default());
    }
}
