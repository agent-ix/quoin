// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `measurement` domain's wire shapes, and the ceilings they are read
//! under.
//!
//! Request and payload types only: what a caller may say, and what it gets
//! back. No decision is taken here. The operations that take them live in
//! [`super::record`], [`super::produce`], [`super::report`] and
//! [`super::portfolio`], and the exit-taxonomy mapping in [`super::taxonomy`].
//!
//! # Twelve routes, six request shapes
//!
//! Every route locates a measurement store and then says one more thing about
//! what to do with it, so the requests are shared rather than declared per
//! route: [`RepoRequest`] locates one store, [`PortfolioRequest`] locates
//! several, and the four remaining shapes add a record, a definition, a
//! revision or a metric. A per-route request type would be twelve declarations
//! of "where is the store", and the twelfth would be the one that spells it
//! differently.
//!
//! # The ceilings, and what they are honestly about
//!
//! Each ceiling is a property of a REQUEST and not of the work, and
//! [`super`] applies each one **before `quoin-measurement` is called**. stdin
//! is untrusted (rust-style §11).
//!
//! Three of the twelve names quoin#478 specifies —
//! [`MAX_ASSURANCE_DOCUMENT_BYTES`], [`MAX_WORKFLOW_YAML_BYTES`] and
//! [`MAX_RETAINED_EXPORT_BYTES`] — name documents that **never cross this
//! boundary**: `quoin-measurement` reads them off disk from paths the request
//! names. A ceiling cannot be applied to bytes that do not arrive, so each is
//! declared here as the whole-request ceiling of the route that is ABOUT that
//! class of document, and each says so below rather than posing as a bound on
//! the file. That divergence from the ticket's names is stated in quoin#478's
//! pull request; the alternative — inventing three bounds nothing applies —
//! would be a census of ceilings that do not exist.

use serde::{Deserialize, Serialize};

use quoin_measurement::intervention::agent_eval::AgentEvalInterventionDefinition;
use quoin_measurement::operational::github_release::GitHubReleaseProducerDefinition;

/// The largest `measurement.record` request publishing a measurement
/// collection, in bytes.
///
/// One collection as its producer published it: a plan's observations for one
/// source revision, with their dimensions and populations.
pub const MAX_COLLECTION_BYTES: usize = 4 * 1024 * 1024;

/// The largest `measurement.record` request publishing an intervention
/// experiment record, in bytes.
///
/// One `intervention_experiment` record: two arms, their changed variables,
/// held-constant declarations, interactions and confounders.
pub const MAX_INTERVENTION_RECORD_BYTES: usize = 4 * 1024 * 1024;

/// The largest `measurement.record` request publishing an operational evidence
/// record, in bytes.
///
/// One `operational_evidence` record — a standing capability or an exercise of
/// one — as its producer published it.
pub const MAX_OPERATIONAL_RECORD_BYTES: usize = 4 * 1024 * 1024;

/// The largest producer definition either producer route accepts, in bytes.
///
/// A FIELD bound, applied to the `definition` member alone, so that an
/// oversized definition is named as the definition rather than as "the request
/// was big". It is strictly below both producer routes' whole-request ceilings
/// so that it stays reachable.
pub const MAX_PRODUCER_DEFINITION_BYTES: usize = 1024 * 1024;

/// The largest `measurement.produce_agent_eval_intervention` request, in bytes.
///
/// Named for the retained agent-eval exports the route is about. Those exports
/// do NOT cross this boundary — the definition names two store-relative paths
/// and `quoin-measurement` reads them — so this is the ceiling on the request
/// that names them, which is the only quantity this boundary can refuse.
pub const MAX_RETAINED_EXPORT_BYTES: usize = 2 * 1024 * 1024;

/// The largest `measurement.produce_github_release_operational` request, in
/// bytes.
///
/// Named for the workflow YAML the route is about. The workflow file, the
/// workflow-run export and the workflow-jobs export are all read off disk by
/// `quoin-measurement` from paths the definition names, so — as with
/// [`MAX_RETAINED_EXPORT_BYTES`] — this bounds the request that names them.
pub const MAX_WORKFLOW_YAML_BYTES: usize = 2 * 1024 * 1024;

/// The largest single-store report request, in bytes.
///
/// `measurement.build_report`, `measurement.render_report`,
/// `measurement.build_comparison`, `measurement.render_comparison` and
/// `measurement.build_series`. Named for the `MeasurementPlan` and
/// `AssuranceProfile` documents these routes read; those are frontmatter in
/// `spec/assurance/` and are read off disk, so this bounds the request that
/// locates them.
pub const MAX_ASSURANCE_DOCUMENT_BYTES: usize = 1024 * 1024;

/// The largest portfolio request, in bytes.
///
/// `measurement.build_portfolio`, `measurement.render_portfolio`,
/// `measurement.build_graph_portfolio` and
/// `measurement.render_graph_portfolio`. A list of repository roots, plus the
/// graph mappings on the two graph routes.
pub const MAX_PORTFOLIO_ROOTS_BYTES: usize = 4 * 1024 * 1024;

/// The largest single graph mapping list, in bytes.
///
/// A FIELD bound applied to each of `graph_exports`, `graph_premises`,
/// `graph_audits` and `changed` separately, below
/// [`MAX_PORTFOLIO_ROOTS_BYTES`] so that it stays reachable.
pub const MAX_GRAPH_MAPPING_BYTES: usize = 256 * 1024;

/// The largest metric name `measurement.build_series` accepts, in bytes.
///
/// A metric is a plan's `metric` frontmatter key — `finding_recall`, not a
/// document.
pub const MAX_METRIC_NAME_BYTES: usize = 256;

/// The largest source revision the comparison routes accept, in bytes.
///
/// A git revision as a producer recorded it.
pub const MAX_REVISION_BYTES: usize = 256;

/// The largest record identity a producer definition may carry, in bytes.
///
/// Deliberately `quoin_measurement`'s own constant rather than a second `128`
/// written here: the store already refuses a longer identity when it derives a
/// basename from it, and a boundary that disagreed with it would refuse
/// records the store accepts or admit records it will not write.
pub const MAX_RECORD_ID_BYTES: usize = quoin_measurement::MAX_RECORD_ID_BYTES;

/// The largest `measurement.verify` request, in bytes.
///
/// A plan id, an optional claimed verdict and the intake order: one group of
/// collection ids per git commit that added collections. The order grows with
/// the store — 49 ids of about 40 bytes today — so this is sized like
/// [`MAX_PORTFOLIO_ROOTS_BYTES`], not like a single name.
pub const MAX_VERIFY_REQUEST_BYTES: usize = 4 * 1024 * 1024;

/// One measurement store, located.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepoRequest {
    /// The repository root holding the measurement store.
    pub repo: String,
}

/// A record to publish, and the store to publish it into.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordRequest {
    /// The repository root holding the measurement store.
    pub repo: String,
    /// The candidate document, exactly as its producer published it.
    ///
    /// Untyped on purpose: which of the three intakes accepts it is decided by
    /// its own `record_type` member, and a typed union here would be a second
    /// declaration of the three record shapes `quoin-measurement` already owns.
    pub record: serde_json::Value,
}

/// A plan to verify, and the store holding its collections (FR-108).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifyRequest {
    /// The repository root holding the measurement store.
    pub repo: String,
    /// The `MeasurementPlan` id to verify.
    pub plan: String,
    /// The verdict someone claims for the plan, when there is one to check:
    /// `accept`, `reject` or `inconclusive`.
    #[serde(default)]
    pub claimed: Option<String>,
    /// Collection ids in an order the producer cannot choose, earliest group
    /// first; ids in one group are tied. A collection named in no group has
    /// no attested position.
    #[serde(default)]
    pub intake_order: Vec<Vec<String>>,
}

/// A revision to compare the latest collection against.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonRequest {
    /// The repository root holding the measurement store.
    pub repo: String,
    /// The earlier source revision.
    pub before_revision: String,
}

/// One metric's history.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeriesRequest {
    /// The repository root holding the measurement store.
    pub repo: String,
    /// The metric to read the series of.
    pub metric: String,
}

/// An agent-eval intervention to produce.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentEvalRequest {
    /// The repository root holding the measurement store.
    pub repo: String,
    /// The versioned producer definition.
    pub definition: AgentEvalInterventionDefinition,
}

/// A GitHub-release operational pair to produce.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitHubReleaseRequest {
    /// The repository root holding the measurement store.
    pub repo: String,
    /// The versioned release-producer definition.
    pub definition: GitHubReleaseProducerDefinition,
}

/// Several measurement stores, located.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortfolioRequest {
    /// The repository roots to include.
    pub locations: Vec<String>,
}

/// Several measurement stores, plus the graph mappings read over them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphPortfolioRequest {
    /// The repository roots to include.
    pub locations: Vec<String>,
    /// `<repository>=<path>` mappings naming an existing Quire assurance
    /// export.
    #[serde(default)]
    pub graph_exports: Vec<String>,
    /// `<repository>=<path>` mappings naming accepted graph premises.
    #[serde(default)]
    pub graph_premises: Vec<String>,
    /// `<repository>=<path>` mappings naming a source-bound audit envelope.
    #[serde(default)]
    pub graph_audits: Vec<String>,
    /// `<repository>=<requirement>` mappings for graph change-impact.
    #[serde(default)]
    pub changed: Vec<String>,
    /// The directory relative mapping paths resolve against.
    #[serde(default)]
    pub cwd: Option<String>,
}

/// The payload every publishing and producing route writes to stdout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PathPayload {
    /// Where the record was written.
    pub path: String,
}

/// The payload every `render_*` route writes to stdout.
///
/// The rendered document is a JSON string field rather than raw markdown on
/// stdout, because the boundary's rule is that stdout carries a canonical JSON
/// payload and nothing else — the same shape `assurance.render_case` takes. A
/// consumer wanting the file writes the field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RenderedPayload {
    /// The rendered markdown.
    pub rendered: String,
}
