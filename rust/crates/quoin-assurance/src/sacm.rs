// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! SACM 2.3 XMI export and diagnostics for a validated GSN argument.

mod artifact;
mod xml;

use std::collections::{BTreeMap, BTreeSet};

use quoin_evidence::source::EvidenceSource;
use serde::Serialize;

use crate::gsn::{self, GsnCase, GsnEdge, GsnNode, Unmapped};

use artifact::Artifacts;
use xml::Xml;

const XMI_NAMESPACE: &str = "http://www.omg.org/spec/XMI/20131001";
const SACM_NAMESPACE: &str = "http://www.omg.org/spec/SACM/20220301";

/// Failure to produce a complete SACM document.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// The source case violates GSN well-formedness.
    #[error("{0}")]
    InvalidGsn(String),
    /// A model id is unsuitable for an XMI reference.
    #[error("invalid XMI id {id:?}")]
    InvalidId {
        /// The offending id.
        id: String,
    },
    /// Generated and source ids collide.
    #[error("duplicate XMI id {id:?}")]
    DuplicateId {
        /// The repeated id.
        id: String,
    },
    /// The source contains a character XML 1.0 cannot represent.
    #[error("SACM property {property} contains a character disallowed by XML 1.0")]
    InvalidXmlText {
        /// The affected XML property.
        property: String,
    },
    /// A solution carries a malformed run selection.
    #[error("invalid GSN evidence on {node}: {source}")]
    InvalidEvidence {
        /// The GSN id of the solution.
        node: String,
        /// The evidence parsing failure.
        #[source]
        source: serde_json::Error,
    },
    /// The suite cannot be used as a single path component.
    #[error("invalid retained-run suite id {suite:?}")]
    InvalidSuite {
        /// The offending suite id.
        suite: String,
    },
    /// The retained record could not be read.
    #[error(transparent)]
    Evidence(#[from] quoin_evidence::error::EvidenceError),
    /// An artifact was absent from the assembled package.
    #[error("missing indexed SACM evidence artifact")]
    MissingArtifact,
    /// The XML writer failed.
    #[error("could not write SACM XMI: {0}")]
    Xml(#[from] std::io::Error),
}

/// Source content omitted from the SACM package.
#[derive(Debug, Serialize)]
pub struct ExportReport {
    /// Unmapped source identifiers and the reasons they could not map.
    pub unmapped: Vec<Unmapped>,
    /// Unreachable requirement identifiers.
    pub unreachable: Vec<String>,
}

impl ExportReport {
    /// Diagnostics in the same order as the JSON report.
    pub fn diagnostics(&self) -> impl Iterator<Item = String> + '_ {
        self.unmapped
            .iter()
            .map(|item| {
                format!(
                    "unmapped: {}: {}",
                    diagnostic_line(&item.source),
                    diagnostic_line(&item.reason)
                )
            })
            .chain(
                self.unreachable
                    .iter()
                    .map(|id| format!("unreachable: {}", diagnostic_line(id))),
            )
    }
}

fn diagnostic_line(value: &str) -> String {
    value.replace('\r', "\\r").replace('\n', "\\n")
}

/// A complete in-memory XMI document and its omission report.
#[derive(Debug)]
pub struct Export {
    /// UTF-8 XMI 2.5.1 bytes.
    pub xmi: Vec<u8>,
    /// Source items omitted from the package.
    pub report: ExportReport,
}

/// Export the shared GSN model and hash the exact UTF-8 run files it selects.
///
/// `source` is the evidence store for the same repository that produced the
/// GSN model. No bytes are returned until the entire document is assembled.
///
/// # Errors
///
/// Returns an error for an invalid GSN model, invalid XMI data, or unreadable
/// selected evidence. A missing file is represented as undischarged evidence.
pub fn export(model: &GsnCase, source: &dyn EvidenceSource) -> Result<Export, ExportError> {
    gsn::validate(model).map_err(ExportError::InvalidGsn)?;
    let artifacts = Artifacts::collect(model, source)?;
    let graph = Graph::new(model);
    let mut report = ExportReport {
        unmapped: model.unmapped.clone(),
        unreachable: model.unreachable.clone(),
    };
    let relations = graph.relations(model, &artifacts, &mut report)?;
    let needs_support = graph.needs_support(model, &relations);
    let mut xml = Xml::new()?;
    xml.start(
        "sacm:AssuranceCasePackage",
        &[
            ("xmlns:xmi", XMI_NAMESPACE),
            ("xmlns:sacm", SACM_NAMESPACE),
            ("xmi:version", "2.5.1"),
            ("xmi:type", "sacm:AssuranceCasePackage"),
            ("xmi:id", "case"),
        ],
    )?;
    xml.name("Assurance case")?;
    xml.start(
        "argumentPackage",
        &[
            ("xmi:type", "sacm:ArgumentPackage"),
            ("xmi:id", "case-argument"),
        ],
    )?;
    xml.name("Argument")?;
    for node in &model.nodes {
        write_node(&mut xml, node, &artifacts, &needs_support, &mut report)?;
    }
    for relation in relations {
        relation.write(&mut xml)?;
    }
    xml.end("argumentPackage")?;
    xml.start(
        "artifactPackage",
        &[
            ("xmi:type", "sacm:ArtifactPackage"),
            ("xmi:id", "case-artifacts"),
        ],
    )?;
    xml.name("Evidence artifacts")?;
    artifacts.write(&mut xml)?;
    xml.end("artifactPackage")?;
    xml.end("sacm:AssuranceCasePackage")?;
    report
        .unmapped
        .sort_by(|left, right| (&left.source, &left.reason).cmp(&(&right.source, &right.reason)));
    report.unmapped.dedup();
    report.unreachable.sort();
    report.unreachable.dedup();
    Ok(Export {
        xmi: xml.finish(),
        report,
    })
}

fn write_node(
    xml: &mut Xml,
    node: &GsnNode,
    artifacts: &Artifacts<'_>,
    needs_support: &BTreeSet<&str>,
    report: &mut ExportReport,
) -> Result<(), ExportError> {
    let open = node.status == "open"
        || node.undeveloped == Some(true)
        || node.kind == "counter_claim"
        || needs_support.contains(node.id.as_str());
    let (sacm_type, declaration) = match node.kind.as_str() {
        "goal" | "counter_claim" => (
            "sacm:Claim",
            Some(if open { "needsSupport" } else { "asserted" }),
        ),
        "assumption" => ("sacm:Claim", Some("assumed")),
        "justification" => ("sacm:Claim", Some("asserted")),
        "strategy" => ("sacm:ArgumentReasoning", None),
        "solution" | "context" => ("sacm:ArtifactReference", None),
        kind => {
            report.unmapped.push(Unmapped {
                source: node.source.clone(),
                reason: format!("GSN node kind {kind} has no SACM mapping"),
            });
            return Ok(());
        }
    };
    let mut attributes = vec![("xmi:id", node.id.as_str()), ("xmi:type", sacm_type)];
    if let Some(declaration) = declaration {
        attributes.push(("assertionDeclaration", declaration));
    }
    if let Some(solution) = artifacts.solution(&node.id)
        && !solution.artifact_ids.is_empty()
    {
        attributes.push(("referencedArtifactElement", &solution.artifact_ids));
    }
    xml.start("argumentElement", &attributes)?;
    xml.name(&node.source)?;
    let description = match (open, node.open_reason.as_deref()) {
        (true, Some(reason)) => format!("open: {reason}\n{}", node.statement),
        (true, None) => format!("open: {}", node.statement),
        (false, _) => node.statement.clone(),
    };
    xml.description(&description)?;
    xml.end("argumentElement")
}

struct Graph<'a> {
    nodes: BTreeMap<&'a str, &'a GsnNode>,
    support_parents: BTreeMap<&'a str, Vec<&'a str>>,
    context_parents: BTreeMap<&'a str, Vec<&'a str>>,
    non_strategy_children: BTreeSet<&'a str>,
}

impl<'a> Graph<'a> {
    fn new(model: &'a GsnCase) -> Self {
        let mut graph = Self {
            nodes: model
                .nodes
                .iter()
                .map(|node| (node.id.as_str(), node))
                .collect(),
            support_parents: BTreeMap::new(),
            context_parents: BTreeMap::new(),
            non_strategy_children: BTreeSet::new(),
        };
        for edge in &model.edges {
            let parents = match edge.kind.as_str() {
                "supported_by" => {
                    if graph
                        .nodes
                        .get(edge.to.as_str())
                        .is_some_and(|child| child.kind != "strategy")
                    {
                        graph.non_strategy_children.insert(edge.from.as_str());
                    }
                    &mut graph.support_parents
                }
                "in_context_of" => &mut graph.context_parents,
                _ => continue,
            };
            parents.entry(&edge.to).or_default().push(&edge.from);
        }
        graph
    }

    fn node(&self, id: &str) -> Result<&'a GsnNode, ExportError> {
        self.nodes
            .get(id)
            .copied()
            .ok_or_else(|| ExportError::InvalidGsn(format!("WF-6 missing node {id}")))
    }

    fn claim_targets(&self, from: &'a GsnNode) -> Vec<&'a GsnNode> {
        if from.kind == "goal" {
            return vec![from];
        }

        // A reasoning element can describe an inference with a direct parent
        // Claim. Traversing a parent strategy would manufacture an inference
        // through an edge that the report says could not be mapped.
        let mut seen = BTreeSet::new();
        self.support_parents
            .get(from.id.as_str())
            .into_iter()
            .flatten()
            .filter_map(|parent| self.nodes.get(parent).copied())
            .filter(|parent| parent.kind == "goal" && seen.insert(parent.id.as_str()))
            .collect()
    }

    fn relations(
        &self,
        model: &'a GsnCase,
        artifacts: &Artifacts<'_>,
        report: &mut ExportReport,
    ) -> Result<Vec<Relationship<'a>>, ExportError> {
        let mut relationships = Vec::new();
        for (index, edge) in model.edges.iter().enumerate() {
            let from = self.node(&edge.from)?;
            let to = self.node(&edge.to)?;
            match edge.kind.as_str() {
                "in_context_of" => {
                    relationships.push(Relationship::new("AssertedContext", to, from, index))
                }
                "challenges" => {
                    let mut relationship = Relationship::new("AssertedInference", from, to, index);
                    relationship.is_counter = true;
                    relationships.push(relationship);
                }
                "supported_by" => {
                    self.support_relations(index, edge, artifacts, &mut relationships, report)?;
                }
                _ => {
                    return Err(ExportError::InvalidGsn(format!(
                        "unknown edge kind {}",
                        edge.kind
                    )));
                }
            }
        }
        Ok(relationships)
    }

    fn support_relations(
        &self,
        index: usize,
        edge: &'a GsnEdge,
        artifacts: &Artifacts<'_>,
        relationships: &mut Vec<Relationship<'a>>,
        report: &mut ExportReport,
    ) -> Result<(), ExportError> {
        let from = self.node(&edge.from)?;
        let to = self.node(&edge.to)?;
        if to.kind == "strategy" {
            let reason = if from.kind == "strategy" {
                Some("nested reasoning has no SACM relationship between strategies")
            } else if !self.non_strategy_children.contains(to.id.as_str()) {
                Some("strategy has no child for an asserted relationship")
            } else {
                None
            };
            if let Some(reason) = reason {
                unmapped_edge(from, to, reason, report);
            }
            return Ok(());
        }
        let targets = self.claim_targets(from);
        if targets.is_empty() {
            unmapped_edge(from, to, "strategy has no direct parent Claim", report);
        }
        let non_supporting = artifacts
            .solution(&to.id)
            .and_then(artifact::Solution::non_supporting);
        for target in targets {
            let relation_type = if to.kind == "solution" {
                "AssertedEvidence"
            } else {
                "AssertedInference"
            };
            let mut relationship = Relationship::new(relation_type, to, target, index);
            relationship.reasoning = (from.kind == "strategy").then_some(from.id.as_str());
            relationship.non_supporting = non_supporting.map(crate::matrix::EvidenceStatus::as_str);
            relationships.push(relationship);
        }
        Ok(())
    }

    fn needs_support(
        &self,
        model: &'a GsnCase,
        relations: &[Relationship<'a>],
    ) -> BTreeSet<&'a str> {
        let mut open = BTreeSet::new();
        let mut visited = BTreeSet::new();
        let mut challenged = Vec::new();
        for relation in relations {
            if relation.non_supporting.is_some() {
                open.insert(relation.target);
            }
            if relation.is_counter {
                challenged.push(relation.target);
            }
        }
        let roots: BTreeSet<_> = model.roots.iter().map(String::as_str).collect();
        while let Some(id) = challenged.pop() {
            if !visited.insert(id) {
                continue;
            }
            if roots.contains(id) {
                open.insert(id);
            }
            for parents in [&self.support_parents, &self.context_parents] {
                if let Some(parents) = parents.get(id) {
                    challenged.extend(parents.iter().copied());
                }
            }
        }
        open
    }
}

fn unmapped_edge(from: &GsnNode, to: &GsnNode, reason: &str, report: &mut ExportReport) {
    report.unmapped.push(Unmapped {
        source: from.source.clone(),
        reason: format!("supported_by {}: {reason}", to.source),
    });
}

struct Relationship<'a> {
    id: String,
    kind: String,
    source: &'a str,
    target: &'a str,
    reasoning: Option<&'a str>,
    is_counter: bool,
    non_supporting: Option<&'static str>,
}

impl<'a> Relationship<'a> {
    fn new(kind: &str, source: &'a GsnNode, target: &'a GsnNode, index: usize) -> Self {
        Self {
            id: format!("{}-{}-{kind}-{}", source.id, target.id, index + 1),
            kind: format!("sacm:{kind}"),
            source: &source.id,
            target: &target.id,
            reasoning: None,
            is_counter: false,
            non_supporting: None,
        }
    }

    fn write(&self, xml: &mut Xml) -> Result<(), ExportError> {
        let mut attrs = vec![
            ("xmi:type", self.kind.as_str()),
            ("xmi:id", self.id.as_str()),
            ("source", self.source),
            ("target", self.target),
            ("isCounter", if self.is_counter { "true" } else { "false" }),
            (
                "assertionDeclaration",
                if self.is_counter || self.non_supporting.is_some() {
                    "needsSupport"
                } else {
                    "asserted"
                },
            ),
        ];
        if let Some(reasoning) = self.reasoning {
            attrs.push(("reasoning", reasoning));
        }
        xml.start("argumentElement", &attrs)?;
        xml.name(&self.id)?;
        if let Some(verdict) = self.non_supporting {
            xml.description(&format!("not supporting: {verdict}"))?;
        }
        xml.end("argumentElement")
    }
}
