// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The assurance case as a graph (quoin#384, ported from `src/assurance/graph.ts`).
//!
//! A **claim** is argued by explicit reasoning over sub-claims, and the leaves
//! are **evidence**. A pile of green obligations is not an argument; the
//! argument is the authored reasoning and its visible gaps.
//!
//! **This is strictly a view.** It collects nothing, computes no coverage and
//! writes nothing. Where the case cannot be built, that is a finding against
//! the spec or the store — never something the view fills in.
//!
//! # The type budget, and why it is smaller than the import list
//!
//! `buildCase` imports five types from four directories, which is what
//! quoin#425 sized four missing crates from. The **field subset** it can
//! observe is twelve fields, and two of the five types it never reads:
//!
//! | type | what the view reads |
//! |---|---|
//! | `Finding` | 3 of 12 — `obligation`, `kind`, `summary` |
//! | `Obligation` | 2 of 9 — `id`, `statement` |
//! | `BundleDocument` | `frontmatter`, which is untyped at the source |
//! | `TrustAssessment` | one sort key |
//! | `IndependenceAssessment` | one sort key |
//!
//! So two get real types ([`quoin_finding_types::Finding`],
//! [`quoin_quire_types::Obligation`]) and three do not, each for a reason
//! stated where it applies below. Sizing from the import graph would have
//! produced four crates, three of which could declare only shapes no
//! behaviour here observes — and a difftest comparing canonical JSON bytes
//! cannot tell a faithful port of an unobserved shape from a wrong one.

use std::collections::{BTreeMap, BTreeSet};

use quoin_evidence::types::Binding;
use quoin_finding_types::Finding;
use quoin_quire_types::Obligation;
use serde::{Deserialize, Serialize};

use crate::matrix::bucket;
use crate::requirement_of;

/// Internal node kinds for the legacy derived view; not an external format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    /// A claim or sub-claim.
    Goal,
    /// The reasoning that decomposes a goal into its sub-goals; it names the
    /// edge verbs the corpus authored.
    Strategy,
    /// A leaf: one obligation.
    Solution,
}

/// Whether a node's part of the argument holds.
///
/// `open` is the point of the whole view. An undischarged obligation or a
/// distrusted binding stays in the tree as an **undeveloped goal** rather than
/// being dropped: a case that silently narrows to only what it can prove reads
/// as complete, and that is the failure this program exists to catch.
///
/// A closed enum here, unlike [`Finding::kind`], and the difference is which
/// side mints the value. This one is produced by this code, so the set is
/// genuinely closed; `kind` arrives from a caller, so it is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeStatus {
    /// Every part of this node's argument holds.
    Supported,
    /// Something is missing, and the reader must see it.
    Open,
}

/// The auditor's verdict on a bound leaf, read from its findings.
///
/// Never re-derived here: it is the precedence [`crate::matrix`] already
/// applies to the same findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    /// Bound, and the auditor raised nothing against it.
    Fresh,
    /// The auditor raised `stale-evidence`.
    Stale,
    /// The auditor distrusts the binding.
    Suspect,
}

/// One suite bound to a leaf, with the newest run or scan the store holds for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuiteRef {
    /// The bound suite.
    pub suite: String,
    /// ISO-8601 timestamp of its newest run or scan; absent when the store
    /// holds none for the suite.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub at: Option<String>,
}

/// What a solution leaf is bound to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeafEvidence {
    /// Bound suites, sorted.
    pub suites: Vec<SuiteRef>,
    /// The newest timestamp across `suites`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub newest: Option<String>,
    /// The auditor's verdict. Absent when the auditor raised a finding that is
    /// neither stale nor suspect; the finding itself is in `because`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub verdict: Option<Verdict>,
}

/// One run or scan the store holds, reduced to what the view shows.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    /// The suite the run or scan covered.
    pub suite: String,
    /// ISO-8601, as the store recorded it.
    pub timestamp: String,
}

/// One node of the case tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseNode {
    /// The document or obligation id.
    pub id: String,
    /// What sort of node this is.
    pub kind: NodeKind,
    /// What the node asserts, in the spec's own words where it has them.
    pub statement: String,
    /// Whether this part of the argument holds.
    pub status: NodeStatus,
    /// Why it is open — the auditor's finding, verbatim. Absent when supported.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub because: Option<String>,
    /// Bound suites, newest run and the auditor's verdict. Solutions only, and
    /// only when the obligation has a binding.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub evidence: Option<LeafEvidence>,
    /// Sub-claims first, then evidence.
    pub children: Vec<CaseNode>,
}

/// A document whose frontmatter could not be read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unreadable {
    /// Path, relative to the bundle root.
    pub path: String,
    /// Why it could not be read.
    pub reason: String,
}

/// The built case.
///
/// # The payload is `camelCase`, the request is `snake_case`
///
/// Not an inconsistency. The **payload** is quoin's existing contract: it is
/// what `src/assurance/graph.ts` already returns to every consumer, and the
/// difftest compares canonical stdout bytes, so a renamed key here is a
/// difference. The **request** is new surface minted at this boundary, and it
/// follows the boundary's own convention (`obligation_id`).
///
/// # Why the two assessments are type parameters
///
/// Because the same field is opaque to one operation and read field-by-field
/// by another, and quoin#425 is the ticket that found that out the hard way.
///
/// [`build_case`] never inspects producer trust or independence: it sorts each
/// by one key and re-emits it unchanged, so for that operation they are
/// `serde_json::Value` and must be — a struct would drop the fields it did not
/// declare, and the difftest compares canonical bytes.
///
/// `render_case` interpolates fourteen of their fields into markdown, so for
/// that operation they are [`quoin_evidence_types::TrustAssessment`] and
/// [`quoin_evidence_types::IndependenceAssessment`].
///
/// Stating that as two structs would work and would drift. Stating it as two
/// parameters means the compiler carries the distinction, and a reader asking
/// "is this field read here?" gets the answer from the signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AssuranceCase<Trust = serde_json::Value, Independence = serde_json::Value> {
    /// Top-level claims, one tree each.
    pub claims: Vec<CaseNode>,
    /// Present exactly when `claims` is empty: why there is no case, naming
    /// the claim types that were searched.
    ///
    /// The human renderer already explained this; the JSON payload carried no
    /// equivalent, so a machine consumer could not tell "the assurance case is
    /// clean" from "nothing matched, so nothing was argued" (quoin#170).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reason: Option<String>,
    /// Requirements reachable from no claim.
    ///
    /// Not an error and not hidden: a bundle whose `StR`s do not reach its `FR`s
    /// has a traceability gap, and the view's job is to make that visible
    /// rather than to render a tidy case over the reachable half.
    pub unreachable: Vec<String>,
    /// Documents whose frontmatter could not be read.
    pub unreadable: Vec<Unreadable>,
    /// Use-specific producer reliance shown as context, never claim support.
    ///
    /// Required on deserialization, with no default, because the retained
    /// renderer reads `assurance.producerTrust.length` unconditionally and
    /// throws when it is absent. The difftest found this: a default here made
    /// `quoin-core` render a case that the retained implementation could not,
    /// which is the port being MORE permissive than the thing it replaces —
    /// the same class of divergence as a closed enum being less permissive,
    /// and just as much a difference.
    ///
    /// `evidence_independence` below keeps its default, because there the
    /// retained code uses `?.` and genuinely tolerates absence. The two fields
    /// differ because the retained implementation treats them differently.
    pub producer_trust: Vec<Trust>,
    /// Profile-selected separation results, shown as context.
    // A path rather than bare `default`: the derive would otherwise demand
    // `Independence: Default`, which is a bound on the ASSESSMENT type for the
    // sake of an absent list. `Option::default` is `None` for any `T`.
    #[serde(skip_serializing_if = "Option::is_none", default = "Option::default")]
    pub evidence_independence: Option<Vec<Independence>>,
}

/// What the view is given to read.
///
/// # `documents` is `Value`, and that is the retained type
///
/// `BundleDocument.frontmatter` is `Record<string, unknown>` at the source,
/// and every read of it in `buildCase` is a dynamic lookup behind a `String()`
/// coercion and a runtime guard. `serde_json::Value` is therefore not a
/// weakening — it is the same type. Porting it as a struct would invent a
/// schema the TypeScript deliberately declines to assert, and the invented one
/// would then diverge from whatever the corpus actually writes.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseInput {
    /// Frontmatter of every document in the bundle.
    pub documents: Vec<serde_json::Value>,
    /// Obligations, from `quire coverage --json`.
    pub obligations: Vec<Obligation>,
    /// The auditor's findings, keyed by obligation elsewhere.
    pub findings: Vec<Finding>,
    /// The binding graph, from the store.
    #[serde(default)]
    pub bindings: Option<Vec<Binding>>,
    /// Every run and scan the store holds, reduced to suite and timestamp.
    #[serde(default)]
    pub observations: Option<Vec<Observation>>,
    /// Artifact types that are top-level claims.
    ///
    /// Defaults to `StR`. A safety or security bundle argues from a declared
    /// hazard or threat instead, and that vocabulary is module data — so it is
    /// a parameter here, never a constant.
    #[serde(default)]
    pub claim_types: Option<Vec<String>>,
    /// Documents whose frontmatter could not be read.
    #[serde(default)]
    pub unreadable: Option<Vec<Unreadable>>,
    /// Producer trust, carried through verbatim. See [`sorted_by_key`].
    #[serde(default)]
    pub producer_trust: Option<Vec<serde_json::Value>>,
    /// Independence assessments, carried through verbatim.
    #[serde(default)]
    pub evidence_independence: Option<Vec<serde_json::Value>>,
}

/// The edge verbs that mean "this document elaborates that one".
const DOWNWARD_FROM_CHILD: [&str; 6] = [
    "traces_to",
    "refines",
    "implements",
    "derives_from",
    "mitigates",
    "satisfies",
];

/// Build the case.
///
/// The decomposition is read from the documents' own `relationships`, in the
/// child→parent direction the corpus actually authors: an FR says it
/// `traces_to` an `StR`, not the other way round.
#[must_use]
pub fn build_case(input: &CaseInput) -> AssuranceCase {
    // Case-insensitive: `--claim-type str`, `STR` or `Hazard` used to match by
    // `===` against the authored `type:` and silently produce an empty case —
    // indistinguishable from a corpus that genuinely declares no claims
    // (quoin#170).
    let requested: Vec<String> = input
        .claim_types
        .clone()
        .unwrap_or_else(|| vec!["StR".to_owned()]);
    let claim_types: BTreeSet<String> = requested.iter().map(|t| t.to_lowercase()).collect();

    // `BTreeMap` rather than a hash map: the retained code iterates a JS `Map`
    // in insertion order to mint claims, then sorts them by id. Sorted
    // iteration reaches the same set and makes the order of `unreachable`
    // (also sorted downstream) independent of input order rather than
    // accidentally stable.
    let mut by_id: BTreeMap<String, &serde_json::Value> = BTreeMap::new();
    let mut insertion: Vec<String> = Vec::new();
    for doc in &input.documents {
        if let Some(id) = id_of(doc) {
            if by_id.insert(id.clone(), doc).is_none() {
                insertion.push(id);
            } else {
                // Last-wins, matching `Map.set`. The id keeps its first
                // position in `insertion`, which is also what JS does.
            }
        }
    }

    // parent id → child id → the verbs the child declares toward it, inverted
    // from the edges each child declares. A set, so a child that declares two
    // verbs toward one parent is one sub-claim argued over both.
    let mut children: Children = BTreeMap::new();
    for doc in &input.documents {
        let Some(id) = id_of(doc) else { continue };
        for (parent, verb) in parents_of(doc) {
            if !by_id.contains_key(&parent) {
                continue;
            }
            children
                .entry(parent)
                .or_default()
                .entry(id.clone())
                .or_default()
                .insert(verb);
        }
    }

    // The auditor already orders by severity, so the first finding for an
    // obligation is its most serious one, and that is the one a reader needs
    // on the node. All of them feed the verdict.
    let mut finding_for: FindingsFor = BTreeMap::new();
    for finding in &input.findings {
        finding_for
            .entry(&finding.obligation)
            .or_default()
            .push(finding);
    }

    let store = Store::new(input);

    let mut obligations_for: BTreeMap<&str, Vec<&Obligation>> = BTreeMap::new();
    for obligation in &input.obligations {
        obligations_for
            .entry(requirement_of(&obligation.id))
            .or_default()
            .push(obligation);
    }

    let mut claims: Vec<CaseNode> = Vec::new();
    let mut reached: BTreeSet<String> = BTreeSet::new();
    for id in &insertion {
        let Some(doc) = by_id.get(id.as_str()) else {
            continue;
        };
        let declared = doc
            .get("frontmatter")
            .and_then(|f| f.get("type"))
            .map_or_else(String::new, string_of)
            .to_lowercase();
        if !claim_types.contains(&declared) {
            continue;
        }
        // A fresh `ancestors` set per claim. Sharing one across claims made a
        // requirement that refines TWO claims appear under only the first,
        // while the second reported "no sub-claim traces to this claim" — a
        // statement that was false, in an assurance case, about the very edge
        // the author had written. Cycle prevention and "already rendered
        // somewhere" are different questions and have different sets.
        claims.push(node_for(
            id,
            &by_id,
            &children,
            &obligations_for,
            &finding_for,
            &store,
            &mut reached,
            &BTreeSet::new(),
        ));
    }
    claims.sort_by(|a, b| a.id.cmp(&b.id));

    let mut unreachable: Vec<String> = by_id
        .keys()
        .filter(|id| !reached.contains(*id) && obligations_for.contains_key(id.as_str()))
        .cloned()
        .collect();
    unreachable.sort();

    let reason = if claims.is_empty() {
        // The human renderer's explanation, made machine-readable. Naming the
        // SEARCHED types (as the caller spelled them) is what lets a pipeline
        // see that `--claim-type hazard` over a hazard-free corpus argued
        // nothing.
        Some(format!(
            "no document declares itself a top-level claim (searched claim types: {}); declare one or pass --claim-type",
            requested.join(", ")
        ))
    } else {
        None
    };

    AssuranceCase {
        claims,
        reason,
        unreachable,
        unreadable: input.unreadable.clone().unwrap_or_default(),
        producer_trust: sorted_by_key(input.producer_trust.as_deref().unwrap_or_default(), "id"),
        evidence_independence: input
            .evidence_independence
            .as_deref()
            .map(|values| sorted_by_key(values, "obligation")),
    }
}

/// Sort opaque assessments by one string field, carrying them through unchanged.
///
/// # Why these two are `Value` and not ported types
///
/// `TrustAssessment` and `IndependenceAssessment` are the two types
/// `buildCase` never inspects. Across all of `src/assurance/`, every mention
/// is a type position or one of these two sorts: it reads `id` and
/// `obligation` as sort keys and re-emits the whole value untouched. It never
/// branches on `status`, never reads `dimensions`, never reads `triggeredBy`.
///
/// Declaring those shapes here would declare something no behaviour observes,
/// in a crate whose only reason to exist is that a type name appears in an
/// import. The difftest compares canonical JSON bytes, so it could not tell a
/// faithful port of an unobserved field from a wrong one — a gate that cannot
/// fail. The difftest case `assurance/build-case-assessments-pass-through`
/// carries a non-trivial assessment through this function and proves the bytes
/// survive, which is what converts "it cannot fail" into "here it is passing".
///
/// A stable sort, like `Array.prototype.sort`: entries with equal keys keep
/// their input order rather than being reordered by an unstable algorithm.
fn sorted_by_key(values: &[serde_json::Value], key: &str) -> Vec<serde_json::Value> {
    let mut out = values.to_vec();
    out.sort_by(|a, b| {
        let left = a.get(key).map_or_else(String::new, string_of);
        let right = b.get(key).map_or_else(String::new, string_of);
        left.cmp(&right)
    });
    out
}

#[allow(
    clippy::too_many_arguments,
    reason = "mirrors the retained recursion's parameter list; bundling them into a context struct would hide which of the two sets is passed by value at each call, and that distinction is the quoin#170 fix"
)]
fn node_for(
    id: &str,
    by_id: &BTreeMap<String, &serde_json::Value>,
    children: &Children,
    obligations_for: &BTreeMap<&str, Vec<&Obligation>>,
    finding_for: &FindingsFor,
    store: &Store,
    reached: &mut BTreeSet<String>,
    ancestors: &BTreeSet<String>,
) -> CaseNode {
    reached.insert(id.to_owned());
    let mut ancestors = ancestors.clone();
    ancestors.insert(id.to_owned());

    // `String(doc?.frontmatter.title ?? id)`. The `??` is why an explicit
    // `null` falls through to the id rather than stringifying: a document that
    // writes `title:` with no value must render as its id, not as an empty
    // statement in an assurance case.
    let title = by_id
        .get(id)
        .and_then(|doc| doc.get("frontmatter"))
        .and_then(|f| f.get("title"))
        .filter(|title| !title.is_null())
        .map_or_else(|| id.to_owned(), string_of);

    // `ancestors`, not `reached`: a child already rendered under a DIFFERENT
    // claim must still appear here, and only a child on this path is a cycle.
    // The map is keyed by child id, so iteration is already sorted.
    let sub_edges: Vec<(&String, &BTreeSet<String>)> = children
        .get(id)
        .map(|edges| {
            edges
                .iter()
                .filter(|(child, _)| !ancestors.contains(*child))
                .collect()
        })
        .unwrap_or_default();

    let sub_nodes: Vec<CaseNode> = sub_edges
        .iter()
        .map(|(child, _)| {
            node_for(
                child,
                by_id,
                children,
                obligations_for,
                finding_for,
                store,
                reached,
                &ancestors,
            )
        })
        .collect();

    let mut parts: Vec<CaseNode> = Vec::new();
    if !sub_nodes.is_empty() {
        parts.push(strategy_for(id, &sub_edges, sub_nodes));
    }

    let mut evidence: Vec<&Obligation> = obligations_for.get(id).cloned().unwrap_or_default();
    evidence.sort_by(|a, b| a.id.cmp(&b.id));
    parts.extend(
        evidence
            .into_iter()
            .map(|obligation| solution_for(obligation, finding_for, store)),
    );

    // A goal with no sub-goals and no evidence is open, which a reader must
    // see. Rendering it as supported would assure a claim on the strength of
    // nobody having written anything.
    let status = if parts.is_empty() || parts.iter().any(|p| p.status == NodeStatus::Open) {
        NodeStatus::Open
    } else {
        NodeStatus::Supported
    };
    let because = if parts.is_empty() {
        Some("no sub-claim and no obligation traces to this claim".to_owned())
    } else {
        None
    };

    CaseNode {
        id: id.to_owned(),
        kind: NodeKind::Goal,
        statement: title,
        status,
        because,
        evidence: None,
        children: parts,
    }
}

/// Child id → the verbs it declares toward one parent.
type Children = BTreeMap<String, BTreeMap<String, BTreeSet<String>>>;

/// Obligation id → every auditor finding against it, most serious first.
type FindingsFor<'a> = BTreeMap<&'a str, Vec<&'a Finding>>;

/// What the store holds, as the leaves read it.
struct Store<'a> {
    /// Obligation id → the suites bound to it.
    suites_for: BTreeMap<&'a str, BTreeSet<&'a str>>,
    /// Suite → its newest run or scan timestamp.
    newest_for_suite: BTreeMap<&'a str, &'a str>,
}

impl<'a> Store<'a> {
    fn new(input: &'a CaseInput) -> Self {
        let mut newest_for_suite: BTreeMap<&str, &str> = BTreeMap::new();
        for observation in input.observations.iter().flatten() {
            let newest = newest_for_suite.entry(&observation.suite).or_default();
            if observation.timestamp.as_str() > *newest {
                *newest = &observation.timestamp;
            }
        }
        let mut suites_for: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for binding in input.bindings.iter().flatten() {
            suites_for
                .entry(binding.obligation.as_str())
                .or_default()
                .insert(binding.suite.as_str());
        }
        Self {
            suites_for,
            newest_for_suite,
        }
    }
}

/// The strategy a goal is argued by: its sub-goals, grouped by edge verb.
fn strategy_for(
    goal: &str,
    edges: &[(&String, &BTreeSet<String>)],
    sub_nodes: Vec<CaseNode>,
) -> CaseNode {
    let mut by_verb: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (child, verbs) in edges {
        for verb in *verbs {
            by_verb.entry(verb).or_default().push(child);
        }
    }
    let argued: Vec<String> = by_verb
        .iter()
        .map(|(verb, ids)| format!("`{verb}`: {}", ids.join(", ")))
        .collect();
    let status = if sub_nodes.iter().any(|n| n.status == NodeStatus::Open) {
        NodeStatus::Open
    } else {
        NodeStatus::Supported
    };
    CaseNode {
        id: format!("{goal}/strategy"),
        kind: NodeKind::Strategy,
        statement: format!("argued over {}", argued.join("; ")),
        status,
        because: None,
        evidence: None,
        children: sub_nodes,
    }
}

fn solution_for(obligation: &Obligation, finding_for: &FindingsFor, store: &Store) -> CaseNode {
    let findings = finding_for
        .get(obligation.id.as_str())
        .map_or(&[][..], Vec::as_slice);
    let finding = findings.first();
    CaseNode {
        id: obligation.id.clone(),
        kind: NodeKind::Solution,
        statement: obligation.statement.clone(),
        status: if finding.is_some() {
            NodeStatus::Open
        } else {
            NodeStatus::Supported
        },
        because: finding.map(|f| format!("{}: {}", f.kind, f.summary)),
        evidence: evidence_for(&obligation.id, findings, store),
        children: Vec::new(),
    }
}

/// The leaf's bound suites and the auditor's verdict, when anything is bound.
fn evidence_for(id: &str, findings: &[&Finding], store: &Store) -> Option<LeafEvidence> {
    let suites = store.suites_for.get(id)?;
    let suites: Vec<SuiteRef> = suites
        .iter()
        .map(|suite| SuiteRef {
            suite: (*suite).to_owned(),
            at: store.newest_for_suite.get(suite).map(|at| (*at).to_owned()),
        })
        .collect();
    let newest = suites.iter().filter_map(|s| s.at.clone()).max();
    // Same precedence as the matrix: suspect, then stale. A finding of any
    // other kind leaves the verdict unstated rather than calling it fresh.
    let verdict = match findings.iter().map(|f| bucket(&f.kind)).min() {
        None => Some(Verdict::Fresh),
        Some(0) => Some(Verdict::Suspect),
        Some(1) => Some(Verdict::Stale),
        Some(_) => None,
    };
    Some(LeafEvidence {
        suites,
        newest,
        verdict,
    })
}

/// The document's own id, from frontmatter.
///
/// Non-string or empty ids yield `None`, matching the retained guard
/// (`typeof id === "string" && id.length > 0`) rather than coercing — a
/// numeric `id: 7` is not an id here, and must not become `"7"`.
fn id_of(doc: &serde_json::Value) -> Option<String> {
    match doc.get("frontmatter")?.get("id")? {
        serde_json::Value::String(id) if !id.is_empty() => Some(id.clone()),
        _ => None,
    }
}

/// Ids this document declares itself to elaborate, with the verb it uses.
fn parents_of(doc: &serde_json::Value) -> Vec<(String, String)> {
    let Some(serde_json::Value::Array(rels)) =
        doc.get("frontmatter").and_then(|f| f.get("relationships"))
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in rels {
        if !entry.is_object() {
            continue;
        }
        let edge = entry.get("type").map_or_else(String::new, string_of);
        if !DOWNWARD_FROM_CHILD.contains(&edge.as_str()) {
            continue;
        }
        let target = entry.get("target").map_or_else(String::new, string_of);
        // `ix://org/component/ID` or a bare id — take the last segment either
        // way. `split("/").pop()` on an empty string yields `""` in JS, which
        // the retained code then rejects with `if (id)`.
        if let Some(id) = target.split('/').next_back()
            && !id.is_empty()
        {
            out.push((id.to_owned(), edge));
        }
    }
    out
}

/// `String(value)`, for the four JSON kinds the retained coercion can meet.
///
/// The retained code writes `String(entry.type ?? "")` and
/// `String(doc.frontmatter.title ?? id)`, so a non-string in those positions
/// is stringified rather than rejected, and this must do the same or the two
/// implementations disagree on malformed frontmatter — which is exactly the
/// frontmatter a real corpus has.
fn string_of(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        // JS `String([1,2])` is `"1,2"` and `String({})` is `"[object
        // Object]"`. Neither appears in the `type`/`title` positions of any
        // corpus document, and reproducing JS's array-join and object-tag
        // rules here would be porting a coercion table rather than a view.
        // `serde_json`'s rendering differs, so the difftest would catch it if
        // one ever arrived — which is the outcome we want over a silent
        // agreement on a value neither side should see.
        other => other.to_string(),
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::AssuranceCase;

    /// A complete case payload, in the shape [`AssuranceCase`] serialises.
    fn case() -> serde_json::Value {
        serde_json::json!({
            "claims": [{
                "id": "StR-001",
                "kind": "goal",
                "statement": "The bounded synthetic change is acceptable.",
                "status": "open",
                "children": [{
                    "id": "FR-001-AC-1",
                    "kind": "solution",
                    "statement": "It holds.",
                    "status": "open",
                    "because": "nothing discharges it",
                    "children": []
                }]
            }],
            "unreachable": ["FR-009"],
            "unreadable": [{ "path": "docs/broken.md", "reason": "no frontmatter" }],
            "producerTrust": [],
            "evidenceIndependence": []
        })
    }

    /// `assurance.render_case` reads a whole `AssuranceCase` off untrusted
    /// stdin, so every reader it reaches must refuse a field it does not know:
    /// a field the boundary silently drops is a field the caller believes it
    /// sent, and here that field would be a claim, a reason or a status the
    /// rendered case never shows.
    ///
    /// The key is injected where it is REACHABLE rather than each struct being
    /// read standalone: an isolated struct refusing it says nothing about the
    /// path a caller actually reaches it by.
    #[test]
    fn every_reachable_case_type_refuses_an_unknown_field() {
        serde_json::from_value::<AssuranceCase>(case()).expect("the fixture reads");

        for pointer in ["", "/claims/0", "/claims/0/children/0", "/unreadable/0"] {
            let mut forged = case();
            let target = if pointer.is_empty() {
                &mut forged
            } else {
                forged
                    .pointer_mut(pointer)
                    .unwrap_or_else(|| panic!("{pointer} is a path this case has"))
            };
            target
                .as_object_mut()
                .unwrap_or_else(|| panic!("{pointer} names an object"))
                .insert("inventedField".to_owned(), serde_json::json!(1));

            assert!(
                serde_json::from_value::<AssuranceCase>(forged).is_err(),
                "an unknown field at {pointer} must be refused"
            );
        }
    }
}
