// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Deterministic GSN case export and its well-formedness checks.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{AssuranceCase, CaseNode, NodeKind, NodeStatus};

/// GSN export wire format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GsnCase {
    /// Format discriminator.
    pub format: String,
    /// Nodes in deterministic depth-first order.
    pub nodes: Vec<GsnNode>,
    /// Typed relationships.
    pub edges: Vec<GsnEdge>,
    /// Root node ids.
    pub roots: Vec<String>,
    /// Unreachable requirement ids.
    pub unreachable: Vec<String>,
    /// Source facts that this format does not represent.
    pub unmapped: Vec<Unmapped>,
}

/// A typed GSN node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct GsnNode {
    /// Generated deterministic id.
    pub id: String,
    /// GSN dialect node kind.
    pub kind: String,
    /// Source Quoin id.
    pub source: String,
    /// Source statement.
    pub statement: String,
    /// Supported or open.
    pub status: String,
    /// Reason the node remains open.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub open_reason: Option<String>,
    /// True for an undeveloped goal.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub undeveloped: Option<bool>,
    /// Evidence for a solution node.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub evidence: Option<serde_json::Value>,
    /// Additional source-authored fields retained by the dialect.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source_data: Option<serde_json::Value>,
}

/// A directed GSN relationship.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct GsnEdge {
    /// Relationship kind.
    pub kind: String,
    /// Source node id.
    pub from: String,
    /// Target node id.
    pub to: String,
}

/// A source item that cannot be represented.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unmapped {
    /// Source identifier.
    pub source: String,
    /// Why it is unmapped.
    pub reason: String,
}

/// Convert a derived assurance case to deterministic GSN nodes and edges.
pub fn from_case(case: &AssuranceCase) -> Result<GsnCase, String> {
    let mut builder = Builder {
        nodes: Vec::new(),
        edges: Vec::new(),
        ids: BTreeMap::new(),
        counters: BTreeMap::new(),
    };
    let roots = case
        .claims
        .iter()
        .map(|node| builder.add(node))
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(root) = roots.first() {
        let trust =
            serde_json::to_value(&case.producer_trust).map_err(|error| error.to_string())?;
        append_global_context(&mut builder, root, trust, "producer trust")?;
        let independence =
            serde_json::to_value(&case.evidence_independence).map_err(|error| error.to_string())?;
        append_global_context(&mut builder, root, independence, "evidence independence")?;
    }
    let model = GsnCase {
        format: "quoin.gsn-case/v1".to_owned(),
        nodes: builder.nodes,
        edges: builder.edges,
        roots,
        unreachable: case.unreachable.clone(),
        unmapped: case
            .unreadable
            .iter()
            .map(|item| Unmapped {
                source: item.path.clone(),
                reason: format!("source document could not be read: {}", item.reason),
            })
            .chain(case.formalization_records.iter().map(|item| Unmapped {
                source: item.obligation_id.clone(),
                reason: "formalization record is outside the declared GSN mapping".to_owned(),
            }))
            .collect(),
    };
    validate(&model)?;
    Ok(model)
}

fn append_global_context(
    builder: &mut Builder,
    root: &str,
    source: serde_json::Value,
    label: &str,
) -> Result<(), String> {
    let records = match source {
        serde_json::Value::Array(values) => values,
        serde_json::Value::Null => Vec::new(),
        value => vec![value],
    };
    for (index, record) in records.into_iter().enumerate() {
        let id = record
            .get("id")
            .or_else(|| record.get("obligation"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("{label} {}", index + 1));
        let statement = serde_json::to_string(&record).map_err(|error| error.to_string())?;
        let node = builder.push(
            "context",
            "C",
            &id,
            statement,
            "supported",
            None,
            None,
            Some(record),
        );
        builder.edges.push(GsnEdge {
            kind: "in_context_of".to_owned(),
            from: root.to_owned(),
            to: node,
        });
    }
    Ok(())
}

struct Builder {
    nodes: Vec<GsnNode>,
    edges: Vec<GsnEdge>,
    ids: BTreeMap<String, String>,
    counters: BTreeMap<String, usize>,
}

impl Builder {
    fn add(&mut self, node: &CaseNode) -> Result<String, String> {
        if matches!(
            node.kind,
            NodeKind::Context
                | NodeKind::Assumption
                | NodeKind::Justification
                | NodeKind::CounterClaim
        ) && (!node.children.is_empty()
            || node.evidence.is_some()
            || node.analysis_evidence.is_some()
            || !node.context.is_empty())
        {
            return Err(format!(
                "terminal GSN node {} contains children, evidence, or nested context",
                node.id
            ));
        }
        let (kind, prefix) = match node.kind {
            NodeKind::Goal => ("goal", "G"),
            NodeKind::Strategy => ("strategy", "S"),
            NodeKind::Solution if node.status == NodeStatus::Open => ("goal", "G"),
            NodeKind::Solution => ("solution", "Sn"),
            NodeKind::Context => ("context", "C"),
            NodeKind::Assumption => ("assumption", "A"),
            NodeKind::Justification => ("justification", "J"),
            NodeKind::CounterClaim => ("counter_claim", "CC"),
        };
        let counter = self.counters.entry(prefix.to_owned()).or_default();
        *counter += 1;
        let generated = format!("{prefix}{counter}");
        self.ids.insert(node.id.clone(), generated.clone());
        let undeveloped = (matches!(node.kind, NodeKind::Goal | NodeKind::Strategy)
            || (node.kind == NodeKind::Solution && node.status == NodeStatus::Open))
            && node.children.is_empty();
        self.nodes.push(GsnNode {
            id: generated.clone(),
            kind: kind.to_owned(),
            source: node.id.clone(),
            statement: node.statement.clone(),
            status: if node.status == NodeStatus::Open {
                "open"
            } else {
                "supported"
            }
            .to_owned(),
            open_reason: node.because.clone(),
            undeveloped: undeveloped.then_some(true),
            evidence: if node.kind == NodeKind::Solution && node.status == NodeStatus::Supported {
                node.evidence
                    .as_ref()
                    .map(serde_json::to_value)
                    .transpose()
                    .map_err(|error| error.to_string())?
            } else {
                None
            },
            source_data: None,
        });
        for child in &node.children {
            let edge_position = self.edges.len();
            let child_id = self.add(child)?;
            self.edges.insert(
                edge_position,
                GsnEdge {
                    kind: "supported_by".to_owned(),
                    from: generated.clone(),
                    to: child_id,
                },
            );
        }
        for context in &node.context {
            let edge_position = self.edges.len();
            let context_id = self.add(context)?;
            self.edges.insert(
                edge_position,
                GsnEdge {
                    kind: "in_context_of".to_owned(),
                    from: generated.clone(),
                    to: context_id,
                },
            );
        }
        Ok(generated)
    }
}

/// Convert the authored view returned by FR-047 to GSN.
pub fn from_authored(value: &serde_json::Value) -> Result<GsnCase, String> {
    let mut b = Builder {
        nodes: Vec::new(),
        edges: Vec::new(),
        ids: BTreeMap::new(),
        counters: BTreeMap::new(),
    };
    let top = value
        .get("topClaim")
        .ok_or("authored view is missing topClaim")?;
    let root_source = str_field(top, "id")?;
    let reasoning_rows = value
        .get("reasoning")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let root = b.push(
        "goal",
        "G",
        root_source,
        str_field(top, "statement")?,
        status(top),
        top.get("reasons")
            .and_then(serde_json::Value::as_array)
            .and_then(|v| v.first())
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        reasoning_rows.is_empty().then_some(true),
        Some(top.clone()),
    );
    for reasoning in &reasoning_rows {
        let source = str_field(reasoning, "id")?;
        let criteria_rows = reasoning
            .get("criteria")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let strategy = b.push(
            "strategy",
            "S",
            source,
            str_field(reasoning, "statement")?,
            status(reasoning),
            criteria_rows
                .iter()
                .find_map(|c| c.get("reason").and_then(serde_json::Value::as_str))
                .map(str::to_owned),
            criteria_rows.is_empty().then_some(true),
            Some(reasoning.clone()),
        );
        b.edges.push(GsnEdge {
            kind: "supported_by".to_owned(),
            from: root.clone(),
            to: strategy.clone(),
        });
        for criterion in &criteria_rows {
            let criterion_text = str_field(criterion, "criterion")?;
            let source = format!("{}:{}", source, criterion_text);
            let reason = criterion
                .get("reason")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            let has_evidence = criterion
                .get("decision")
                .and_then(|d| d.get("evidenceRefs"))
                .and_then(serde_json::Value::as_array)
                .is_some_and(|refs| !refs.is_empty());
            let goal = b.push(
                "goal",
                "G",
                &source,
                criterion_text,
                status(criterion),
                reason,
                (!has_evidence).then_some(true),
                Some(criterion.clone()),
            );
            b.edges.push(GsnEdge {
                kind: "supported_by".to_owned(),
                from: strategy.clone(),
                to: goal.clone(),
            });
            if let Some(decision) = criterion.get("decision") {
                for reference in decision
                    .get("evidenceRefs")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(serde_json::Value::as_str)
                {
                    let evidence = b.push(
                        "solution",
                        "Sn",
                        reference,
                        reference,
                        "supported",
                        None,
                        None,
                        Some(serde_json::json!({"evidenceRef": reference, "decision": decision})),
                    );
                    if let Some(node) = b.nodes.last_mut() {
                        node.evidence = Some(serde_json::json!({"references": [reference]}));
                    }
                    b.edges.push(GsnEdge {
                        kind: "supported_by".to_owned(),
                        from: goal.clone(),
                        to: evidence,
                    });
                }
            }
        }
    }
    for assumption in value
        .get("assumptions")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let id = str_field(assumption, "id")?;
        let node = b.push(
            "assumption",
            "A",
            id,
            str_field(assumption, "statement")?,
            status(assumption),
            assumption
                .get("reason")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            None,
            Some(assumption.clone()),
        );
        b.edges.push(GsnEdge {
            kind: "in_context_of".to_owned(),
            from: root.clone(),
            to: node,
        });
    }
    for participant in value
        .get("participants")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let id = str_field(participant, "id")?;
        let node = b.push(
            "context",
            "C",
            id,
            format!(
                "{} ({})",
                str_field(participant, "role")?,
                str_field(participant, "authority")?
            ),
            "supported",
            None,
            None,
            Some(participant.clone()),
        );
        b.edges.push(GsnEdge {
            kind: "in_context_of".to_owned(),
            from: root.clone(),
            to: node,
        });
    }
    for challenge in value
        .get("challenges")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let id = str_field(challenge, "id")?;
        let target = str_field(challenge, "target")?;
        let open = challenge.get("status").and_then(serde_json::Value::as_str) == Some("open");
        let kind = if open {
            "counter_claim"
        } else {
            "justification"
        };
        let prefix = if open { "CC" } else { "J" };
        let node = b.push(
            kind,
            prefix,
            id,
            str_field(challenge, "statement")?,
            if open { "open" } else { "supported" },
            challenge
                .get("reason")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            None,
            Some(challenge.clone()),
        );
        let target_id = b.ids.get(&target).cloned().unwrap_or(target);
        b.edges.push(GsnEdge {
            kind: if open { "challenges" } else { "in_context_of" }.to_owned(),
            from: if open { node } else { target_id.clone() },
            to: if open { target_id } else { node },
        });
    }
    let unmapped = value
        .get("relationships")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|relationship| {
            let source = relationship.get("target")?.as_str()?;
            Some(Unmapped {
                source: source.to_owned(),
                reason: "authored relationship has no declared GSN edge mapping".to_owned(),
            })
        })
        .collect();
    let model = GsnCase {
        format: "quoin.gsn-case/v1".to_owned(),
        nodes: b.nodes,
        edges: b.edges,
        roots: vec![root],
        unreachable: Vec::new(),
        unmapped,
    };
    validate(&model)?;
    Ok(model)
}

impl Builder {
    fn push(
        &mut self,
        kind: &str,
        prefix: &str,
        source: &str,
        statement: impl Into<String>,
        status: &str,
        reason: Option<String>,
        undeveloped: Option<bool>,
        source_data: Option<serde_json::Value>,
    ) -> String {
        let count = self.counters.entry(prefix.to_owned()).or_default();
        *count += 1;
        let id = format!("{prefix}{count}");
        self.ids.insert(source.to_owned(), id.clone());
        self.nodes.push(GsnNode {
            id: id.clone(),
            kind: kind.to_owned(),
            source: source.to_owned(),
            statement: statement.into(),
            status: status.to_owned(),
            open_reason: reason,
            undeveloped,
            evidence: None,
            source_data,
        });
        id
    }
}

fn str_field<'a>(value: &'a serde_json::Value, name: &str) -> Result<&'a str, String> {
    value
        .get(name)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("authored view field {name} must be a string"))
}
fn status(value: &serde_json::Value) -> &'static str {
    if value.get("status").and_then(serde_json::Value::as_str) == Some("supported") {
        "supported"
    } else {
        "open"
    }
}

/// Validate the WF-1..8 graph constraints; errors name rules and affected ids.
pub fn validate(model: &GsnCase) -> Result<(), String> {
    let by_id: BTreeMap<&str, &GsnNode> = model
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    if by_id.len() != model.nodes.len() {
        return Err("WF-6 duplicate node id".to_owned());
    }
    let mut errors = Vec::new();
    for edge in &model.edges {
        let Some(from) = by_id.get(edge.from.as_str()) else {
            errors.push(format!("WF-6 missing source {}", edge.from));
            continue;
        };
        let Some(to) = by_id.get(edge.to.as_str()) else {
            errors.push(format!("WF-6 missing target {}", edge.to));
            continue;
        };
        match edge.kind.as_str() {
            "supported_by" => {
                if !matches!(from.kind.as_str(), "goal" | "strategy")
                    || !matches!(to.kind.as_str(), "goal" | "strategy" | "solution")
                {
                    errors.push(format!(
                        "WF-1 invalid supported_by endpoints {} -> {}",
                        from.id, to.id
                    ));
                }
            }
            "in_context_of" => {
                if !matches!(from.kind.as_str(), "goal" | "strategy")
                    || !matches!(to.kind.as_str(), "context" | "assumption" | "justification")
                {
                    errors.push(format!(
                        "WF-2 invalid in_context_of endpoints {} -> {}",
                        from.id, to.id
                    ));
                }
            }
            "challenges" => {
                if from.kind != "counter_claim"
                    || !matches!(to.kind.as_str(), "goal" | "strategy" | "assumption")
                {
                    errors.push(format!(
                        "WF-8 invalid challenges endpoints {} -> {}",
                        from.id, to.id
                    ));
                }
            }
            _ => errors.push(format!("unknown edge kind {}", edge.kind)),
        }
    }
    let outgoing = |id: &str, kind: &str| {
        model
            .edges
            .iter()
            .filter(|edge| edge.from == id && edge.kind == kind)
            .count()
    };
    for node in &model.nodes {
        if matches!(
            node.kind.as_str(),
            "solution" | "context" | "assumption" | "justification" | "counter_claim"
        ) && (outgoing(&node.id, "supported_by") > 0 || outgoing(&node.id, "in_context_of") > 0)
        {
            errors.push(format!("WF-3 terminal node {} has outgoing edge", node.id));
        }
        if matches!(node.kind.as_str(), "goal" | "strategy")
            && outgoing(&node.id, "supported_by") == 0
            && node.undeveloped != Some(true)
        {
            errors.push(format!(
                "WF-5 {} has no support and is not undeveloped",
                node.id
            ));
        }
        if node.undeveloped == Some(true) && outgoing(&node.id, "supported_by") > 0 {
            errors.push(format!("WF-7 {} is undeveloped but has support", node.id));
        }
        if node.kind == "counter_claim" && outgoing(&node.id, "challenges") != 1 {
            errors.push(format!(
                "WF-8 {} must have exactly one challenges edge",
                node.id
            ));
        }
    }
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for node in &model.nodes {
        if node.kind == "goal" || node.kind == "strategy" {
            cycle(
                node.id.as_str(),
                model,
                &mut visiting,
                &mut visited,
                &mut errors,
            );
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn cycle(
    id: &str,
    model: &GsnCase,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    if visited.contains(id) {
        return;
    }
    if !visiting.insert(id.to_owned()) {
        errors.push(format!("WF-4 supported_by cycle at {id}"));
        return;
    }
    for edge in model
        .edges
        .iter()
        .filter(|edge| edge.from == id && edge.kind == "supported_by")
    {
        cycle(edge.to.as_str(), model, visiting, visited, errors);
    }
    visiting.remove(id);
    visited.insert(id.to_owned());
}

/// Render a GSN model as a Mermaid flowchart.
pub fn render_diagram(model: &GsnCase) -> Result<String, String> {
    validate(model)?;
    let mut lines = vec![
        "flowchart TB".to_owned(),
        "  classDef open stroke:#d33".to_owned(),
    ];
    for node in &model.nodes {
        let full_label = format!("{}: {}", node.source, node.statement);
        let truncated = full_label.chars().take(160).collect::<String>();
        let label = if full_label.chars().count() > 160 {
            format!("{truncated}…")
        } else {
            truncated
        }
        .replace('"', "#quot;")
        .replace('\n', "<br/>");
        let shape = match node.kind.as_str() {
            "goal" => ("[", "]"),
            "strategy" => ("[/", "/]"),
            "solution" => ("((", "))"),
            "context" => ("(", ")"),
            _ => ("([", "])"),
        };
        lines.push(format!(
            "  {}{}\"{}{}\"{}",
            node.id,
            shape.0,
            label,
            if node.undeveloped == Some(true) {
                " ◇"
            } else {
                ""
            },
            shape.1
        ));
        if node.status == "open" {
            lines.push(format!("  class {} open", node.id));
        }
    }
    for edge in &model.edges {
        lines.push(format!(
            "  {} {} {}",
            edge.from,
            if edge.kind == "in_context_of" {
                "-.->"
            } else {
                "-->"
            },
            edge.to
        ));
    }
    Ok(lines.join("\n"))
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test failures should identify the failed assertion"
)]
mod tests {
    use super::{GsnCase, GsnEdge, GsnNode, validate};
    use ix_trace_rs::trace;

    fn node(id: &str, kind: &str, undeveloped: Option<bool>) -> GsnNode {
        GsnNode {
            id: id.to_owned(),
            kind: kind.to_owned(),
            source: id.to_owned(),
            statement: id.to_owned(),
            status: "open".to_owned(),
            open_reason: None,
            undeveloped,
            evidence: None,
            source_data: None,
        }
    }

    fn model(nodes: Vec<GsnNode>, edges: Vec<GsnEdge>) -> GsnCase {
        GsnCase {
            format: "quoin.gsn-case/v1".to_owned(),
            nodes,
            edges,
            roots: vec!["G1".to_owned()],
            unreachable: Vec::new(),
            unmapped: Vec::new(),
        }
    }

    /// Provenance: agent-ix/quoin#PLAT-1447, agent-ix/quoin#PLAT-1644.
    #[trace("FR-222-AC-1")]
    #[test]
    fn rejects_invalid_supported_by_endpoints_with_rule_and_ids() {
        let graph = model(
            vec![
                node("Sn1", "solution", None),
                node("G1", "goal", Some(true)),
            ],
            vec![GsnEdge {
                kind: "supported_by".to_owned(),
                from: "Sn1".to_owned(),
                to: "G1".to_owned(),
            }],
        );
        let error = validate(&graph).expect_err("solution cannot support a goal");
        assert!(error.contains("WF-1"));
        assert!(error.contains("Sn1"));
        assert!(error.contains("G1"));
    }

    /// Provenance: agent-ix/quoin#PLAT-1447.
    #[trace("FR-222-AC-4")]
    #[test]
    fn rejects_support_cycles_and_accepts_shared_children() {
        let cyclic = model(
            vec![node("G1", "goal", None), node("G2", "goal", None)],
            vec![
                GsnEdge {
                    kind: "supported_by".to_owned(),
                    from: "G1".to_owned(),
                    to: "G2".to_owned(),
                },
                GsnEdge {
                    kind: "supported_by".to_owned(),
                    from: "G2".to_owned(),
                    to: "G1".to_owned(),
                },
            ],
        );
        assert!(
            validate(&cyclic)
                .expect_err("cycle rejected")
                .contains("WF-4")
        );
        let shared = model(
            vec![
                node("G1", "goal", None),
                node("G2", "goal", None),
                node("Sn1", "solution", None),
            ],
            vec![
                GsnEdge {
                    kind: "supported_by".to_owned(),
                    from: "G1".to_owned(),
                    to: "Sn1".to_owned(),
                },
                GsnEdge {
                    kind: "supported_by".to_owned(),
                    from: "G2".to_owned(),
                    to: "Sn1".to_owned(),
                },
            ],
        );
        assert!(validate(&shared).is_ok());
    }

    /// Provenance: agent-ix/quoin#PLAT-1447.
    #[trace("FR-221-AC-3")]
    #[test]
    fn open_derived_obligation_becomes_undeveloped_goal_with_reason() {
        let case: crate::AssuranceCase = serde_json::from_value(serde_json::json!({
            "claims": [{"id":"StR-1", "kind":"goal", "statement":"Claim", "status":"open", "children":[
                {"id":"StR-1/strategy", "kind":"strategy", "statement":"Argument", "status":"open", "children":[
                    {"id":"FR-1-AC-1", "kind":"solution", "statement":"Requirement", "status":"open", "because":"audit reason", "children":[]}
                ]}
            ]}], "unreachable":[], "unreadable":[], "producerTrust":[]
        })).expect("case fixture deserializes");
        let model = super::from_case(&case).expect("valid case converts");
        let goal = model
            .nodes
            .iter()
            .find(|node| node.source == "FR-1-AC-1")
            .expect("obligation is present");
        assert_eq!(goal.kind, "goal");
        assert_eq!(goal.status, "open");
        assert_eq!(goal.open_reason.as_deref(), Some("audit reason"));
        assert_eq!(goal.undeveloped, Some(true));
    }

    /// Provenance: agent-ix/quoin#PLAT-1447.
    #[trace("FR-222-AC-2")]
    #[test]
    fn rejects_context_endpoints_terminal_nodes_development_and_bad_references() {
        let in_context_bad = model(
            vec![
                node("A1", "assumption", None),
                node("G1", "goal", Some(true)),
            ],
            vec![GsnEdge {
                kind: "in_context_of".to_owned(),
                from: "A1".to_owned(),
                to: "G1".to_owned(),
            }],
        );
        assert!(
            validate(&in_context_bad)
                .expect_err("WF-2 endpoints")
                .contains("WF-2")
        );

        let terminal_bad = model(
            vec![node("C1", "context", None), node("G1", "goal", Some(true))],
            vec![GsnEdge {
                kind: "supported_by".to_owned(),
                from: "C1".to_owned(),
                to: "G1".to_owned(),
            }],
        );
        let terminal_error =
            validate(&terminal_bad).expect_err("terminal nodes have no outgoing support");
        assert!(terminal_error.contains("WF-1"));
        assert!(terminal_error.contains("WF-3"));

        let undeveloped_missing = model(vec![node("G1", "goal", None)], Vec::new());
        assert!(
            validate(&undeveloped_missing)
                .expect_err("goal must be developed")
                .contains("WF-5")
        );

        let duplicate = model(
            vec![
                node("G1", "goal", Some(true)),
                node("G1", "goal", Some(true)),
            ],
            Vec::new(),
        );
        assert!(
            validate(&duplicate)
                .expect_err("ids are unique")
                .contains("WF-6")
        );

        let dangling = model(
            vec![node("G1", "goal", None)],
            vec![GsnEdge {
                kind: "supported_by".to_owned(),
                from: "G1".to_owned(),
                to: "missing".to_owned(),
            }],
        );
        assert!(
            validate(&dangling)
                .expect_err("endpoints exist")
                .contains("WF-6")
        );

        let undeveloped_with_support = model(
            vec![
                node("G1", "goal", Some(true)),
                node("G2", "goal", Some(true)),
            ],
            vec![GsnEdge {
                kind: "supported_by".to_owned(),
                from: "G1".to_owned(),
                to: "G2".to_owned(),
            }],
        );
        assert!(
            validate(&undeveloped_with_support)
                .expect_err("undeveloped goal has no children")
                .contains("WF-7")
        );

        let counterclaim_without_edge = model(
            vec![
                node("CC1", "counter_claim", None),
                node("G1", "goal", Some(true)),
            ],
            Vec::new(),
        );
        assert!(
            validate(&counterclaim_without_edge)
                .expect_err("counterclaim targets one argument node")
                .contains("WF-8")
        );
        let counterclaim_multiple_edges = model(
            vec![
                node("CC1", "counter_claim", None),
                node("G1", "goal", Some(true)),
                node("S1", "strategy", Some(true)),
            ],
            vec![
                GsnEdge {
                    kind: "challenges".to_owned(),
                    from: "CC1".to_owned(),
                    to: "G1".to_owned(),
                },
                GsnEdge {
                    kind: "challenges".to_owned(),
                    from: "CC1".to_owned(),
                    to: "S1".to_owned(),
                },
            ],
        );
        assert!(
            validate(&counterclaim_multiple_edges)
                .expect_err("counterclaim has one target")
                .contains("WF-8")
        );
        let counterclaim_bad_target = model(
            vec![
                node("CC1", "counter_claim", None),
                node("Sn1", "solution", None),
            ],
            vec![GsnEdge {
                kind: "challenges".to_owned(),
                from: "CC1".to_owned(),
                to: "Sn1".to_owned(),
            }],
        );
        assert!(
            validate(&counterclaim_bad_target)
                .expect_err("counterclaim targets argument nodes only")
                .contains("WF-8")
        );
    }

    /// Provenance: agent-ix/quoin#PLAT-1447.
    #[trace("FR-221-AC-6")]
    #[test]
    fn open_authored_challenge_is_only_a_counter_claim() {
        let view = serde_json::json!({
            "topClaim":{"id":"TC-1","statement":"Claim","status":"open","reasons":["challenge open"]},
            "reasoning":[], "assumptions":[], "participants":[],
            "challenges":[{"id":"CH-1","target":"TC-1","statement":"Objection","owner":"auditor","status":"open","declaredStatus":"open","resolutionRefs":[],"reason":"unresolved"}],
            "relationships":[]
        });
        let model =
            super::from_authored(&view).expect("open challenge maps to a valid counter-claim");
        let challenge = model
            .nodes
            .iter()
            .find(|node| node.source == "CH-1")
            .expect("counter-claim is present");
        assert_eq!(challenge.kind, "counter_claim");
        assert_eq!(challenge.open_reason.as_deref(), Some("unresolved"));
        assert_eq!(
            model
                .edges
                .iter()
                .filter(|edge| edge.kind == "challenges"
                    && edge.from == challenge.id
                    && edge.to == model.roots[0])
                .count(),
            1
        );
        assert!(
            !model
                .nodes
                .iter()
                .any(|node| node.source == "CH-1" && node.kind == "context")
        );
    }

    /// Provenance: agent-ix/quoin#PLAT-1645.
    #[trace("FR-221-AC-10")]
    #[test]
    fn diagram_escapes_quotes_newlines_and_truncates_code_points() {
        let text = format!("{}é", "x".repeat(170));
        let mut goal = node("G1", "goal", Some(true));
        goal.source = "FR-1".to_owned();
        goal.statement = format!("{text}\n\"quoted\"");
        let diagram = super::render_diagram(&model(vec![goal], Vec::new())).expect("valid diagram");
        assert!(diagram.starts_with("flowchart TB\n"));
        assert!(diagram.contains("…"));
        assert!(diagram.contains("class G1 open"));
        let mut escaped_goal = node("G1", "goal", Some(true));
        escaped_goal.statement = "first\n\"quoted\"".to_owned();
        let escaped =
            super::render_diagram(&model(vec![escaped_goal], Vec::new())).expect("valid diagram");
        assert!(escaped.contains("#quot;"));
        assert!(escaped.contains("<br/>"));
    }
}
