// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `evidence` domain's wire shapes, and the ceilings they are read under.
//!
//! Request and payload types only: what a caller may say, and what it gets
//! back. No decision is taken here. The operations that take them live in
//! [`super`], and the exit-taxonomy mapping in [`super::taxonomy`].
//!
//! Two rules shape this file.
//!
//! **The store's own record vocabulary is re-used, never restated.** Every
//! payload that carries a run, a scan, a binding, an inspection, a trust
//! assessment or an independence assessment carries `quoin_evidence`'s type
//! directly. A second declaration of those shapes here would be a second answer
//! to what a `RunRecord` is, and the first thing to drift from the bytes on
//! disk — the store is `camelCase` because that is what is already written
//! there (NFR-025), and this file does not get to have an opinion about it.
//!
//! **The wrapper fields are `snake_case`**, as every other domain's request is.
//! The two conventions sit side by side on purpose: the wrapper is this
//! boundary's vocabulary, the records inside it are the store's.
//!
//! The size ceilings sit with the shapes rather than with the operations
//! because each is a property of a request and not of the work: they bound what
//! may be read off stdin at all, and [`super`] applies each one **before the
//! evidence host is consulted**. stdin is untrusted (rust-style §11).

use serde::{Deserialize, Serialize};

use quoin_evidence::types::{
    BaselineFile, Binding, EvidenceLineage, Finding, FindingRecord, IndependenceAssessment,
    IndependencePolicy, MockInjection, RunEntry, RunRecord, TrustAssessment, TrustTrigger,
};
use quoin_evidence::{ObligationId, SuiteId};

/// The largest `evidence.store_facts` request, in bytes.
///
/// The request is empty. The ceiling exists so the operation has one at all:
/// a request with no ceiling is a request whose size nothing states.
pub const MAX_STORE_FACTS_BYTES: usize = 4 * 1024;

/// The largest `evidence.gc` request, in bytes.
///
/// A repository path and a boolean.
pub const MAX_GC_BYTES: usize = 8 * 1024;

/// The largest `evidence.read_baseline` request, in bytes.
pub const MAX_READ_BASELINE_BYTES: usize = 8 * 1024;

/// The largest `evidence.trust_assessments` request, in bytes.
pub const MAX_TRUST_ASSESSMENTS_BYTES: usize = 8 * 1024;

/// The largest `evidence.inspect_mocks` request, in bytes.
///
/// Suite, commit, tool and timestamp. The SOURCE it inspects never rides on
/// the request — the host walks it.
pub const MAX_INSPECT_MOCKS_BYTES: usize = 64 * 1024;

/// The largest `evidence.affirm` request, in bytes.
pub const MAX_AFFIRM_BYTES: usize = 64 * 1024;

/// The largest `evidence.parse_lineage` request, in bytes.
///
/// Five optional short strings as a JSON document the caller read off disk.
pub const MAX_PARSE_LINEAGE_BYTES: usize = 64 * 1024;

/// The largest `evidence.write_baseline` request, in bytes.
///
/// One `<kind>:<obligation>` key per accepted finding. A corpus-wide backlog is
/// tens of thousands of short keys, not a document.
pub const MAX_WRITE_BASELINE_BYTES: usize = 4 * 1024 * 1024;

/// The largest `evidence.parse_policy` request, in bytes.
///
/// The policy document plus every derived obligation id, because
/// `require_known_policy_obligations` is asked in the same call.
pub const MAX_PARSE_POLICY_BYTES: usize = 8 * 1024 * 1024;

/// The largest `evidence.record_experiment` or `evidence.record_operational`
/// request, in bytes.
///
/// One assurance record as its producer published it.
pub const MAX_ASSURANCE_RECORD_BYTES: usize = 4 * 1024 * 1024;

/// The largest `evidence.trust_decision` request, in bytes.
pub const MAX_TRUST_DECISION_BYTES: usize = 4 * 1024 * 1024;

/// The largest `evidence.audit_inputs` request, in bytes.
///
/// The store is READ by the host, so this bounds the repository path, the head
/// commit and an optional independence policy — not the records.
pub const MAX_AUDIT_INPUTS_BYTES: usize = 8 * 1024 * 1024;

/// The largest `evidence.parse_results` request, in bytes.
///
/// A producer document. A `JUnit` XML or a SARIF report from a corpus-wide scan
/// is the largest thing this boundary reads that is not a repository.
pub const MAX_PARSE_RESULTS_BYTES: usize = 32 * 1024 * 1024;

/// The largest `evidence.record` request, in bytes.
///
/// [`MAX_PARSE_RESULTS_BYTES`] worth of producer document, plus every
/// obligation quire derived for the repository.
pub const MAX_RECORD_BYTES: usize = 48 * 1024 * 1024;

/// The largest single path, identity or timestamp this domain accepts, in
/// bytes.
///
/// A repository root, a suite id, a commit, a tool identity and an ISO-8601
/// stamp are all one of these. 4 KiB is past every platform's `PATH_MAX` and
/// still refuses a stream.
pub const MAX_SCALAR_BYTES: usize = 4 * 1024;

/// A request with no arguments.
///
/// Deserialised rather than ignored so a caller that sends a field learns that
/// this operation takes none, instead of having it silently dropped.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EmptyRequest {}

/// A request naming only the repository whose store is read.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RepoRequest {
    /// The repository root. The store is `<repo>/spec/evidence`.
    pub repo: String,
}

/// The payload `evidence.store_facts` writes to stdout.
///
/// The constants the TypeScript command layer needs *before* it can build a
/// request at all — the adapter names an `--adapter` flag offers, the metric a
/// mutation score is recorded under, the schema version, and where the two
/// checked-in store files live. They are served rather than restated so that
/// `src/core/evidence.ts` has one pinned copy instead of a second declaration.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct StoreFactsPayload {
    /// Every adapter name, run-shaped then finding-shaped, in `--help` order.
    pub adapter_names: Vec<String>,
    /// The metric name a `cargo-mutants` score is recorded under.
    pub mutation_score_metric: String,
    /// The version stamped into every record envelope.
    pub store_schema_version: u32,
    /// Store-relative path of the binding graph.
    pub bindings_path: String,
    /// Store-relative path of the ratchet baseline.
    pub baseline_path: String,
    /// Store-relative path of the authored suite registry.
    pub suites_path: String,
    /// Store-relative path of the authored inspection register.
    pub inspections_path: String,
    /// The family directories `evidence.gc` collects.
    pub collected_families: Vec<String>,
    /// The revalidation triggers every trust decision must declare.
    pub required_triggers: Vec<TrustTrigger>,
}

/// The request accepted by `evidence.gc`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GcRequest {
    /// The repository root.
    pub repo: String,
    /// Report what would be collected and remove nothing.
    #[serde(default)]
    pub dry_run: bool,
}

/// The payload `evidence.gc` writes to stdout.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GcPayload {
    /// The collected records, as **absolute** paths, sorted.
    ///
    /// Every path this domain reports is absolute, and none of the ones
    /// `quoin_evidence` returns are: a library that names no host capability
    /// cannot know the root. The join is [`super::absolute`], against the root
    /// [`crate::capabilities::EvidenceHost::store_root`] reports.
    pub deleted: Vec<String>,
}

/// The request accepted by `evidence.affirm`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AffirmRequest {
    /// The repository root.
    pub repo: String,
    /// The obligation whose binding is re-affirmed.
    pub obligation: String,
    /// One suite, when the reviewer means only one.
    #[serde(default)]
    pub suite: Option<String>,
    /// The statement hash as quire derives it **today**.
    pub statement_hash: String,
    /// Who is affirming. Recorded verbatim.
    pub who: String,
    /// The commit the affirmation is made at.
    pub commit: String,
    /// Why the evidence still holds, when they said.
    #[serde(default)]
    pub note: Option<String>,
}

/// The payload `evidence.affirm` writes to stdout.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AffirmPayload {
    /// Whether any binding matched. Nothing is written when none did.
    pub found: bool,
    /// The obligation, echoed.
    pub obligation: String,
    /// Who affirmed, echoed.
    pub who: String,
    /// The commit, echoed.
    pub commit: String,
    /// The statement hash now stamped on the binding.
    pub statement_hash: String,
}

/// The request accepted by `evidence.parse_lineage`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ParseLineageRequest {
    /// The lineage document's text, as the caller read it.
    pub text: String,
}

/// The payload `evidence.parse_lineage` writes to stdout.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ParseLineagePayload {
    /// The validated lineage.
    pub lineage: EvidenceLineage,
}

/// The request accepted by `evidence.parse_policy`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ParsePolicyRequest {
    /// The policy document's text, as the caller read it.
    pub text: String,
    /// Every obligation id the corpus derives today.
    ///
    /// Sent with the document rather than checked in a second call: a policy
    /// naming an obligation nothing derives reports every requirement as
    /// vacuously assessed, and the two questions have one answer.
    #[serde(default)]
    pub known_obligations: Vec<String>,
}

/// The payload `evidence.parse_policy` writes to stdout.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ParsePolicyPayload {
    /// The validated policy.
    pub policy: IndependencePolicy,
}

/// The request accepted by `evidence.parse_results`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ParseResultsRequest {
    /// The producer document's text, as the caller read it.
    pub text: String,
    /// An explicit adapter name. An unknown one is an error, never a fall back.
    #[serde(default)]
    pub adapter: Option<String>,
    /// The suite's declared tool, which selects an adapter when none is named.
    #[serde(default)]
    pub tool: Option<String>,
}

/// The payload `evidence.parse_results` writes to stdout.
///
/// Tagged, because the two adapter registries produce different record types
/// and the caller must be able to tell which it received without inferring it
/// from which fields are present.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ParseResultsPayload {
    /// A run-shaped adapter's transcript.
    Run {
        /// The transcribed results.
        entries: Vec<RunEntry>,
        /// Results the producer reported that no run outcome represents.
        #[serde(skip_serializing_if = "Option::is_none")]
        unrepresented: Option<Vec<UnrepresentedView>>,
        /// The evidence kind, when the FORMAT ITSELF determines it.
        #[serde(skip_serializing_if = "Option::is_none")]
        evidence_kind: Option<String>,
    },
    /// A finding-shaped adapter's transcript.
    Finding {
        /// The transcribed findings. Empty is meaningful.
        findings: Vec<Finding>,
        /// The scanner, where the document names it.
        #[serde(skip_serializing_if = "Option::is_none")]
        tool: Option<String>,
        /// The ruleset, where the document names it.
        #[serde(skip_serializing_if = "Option::is_none")]
        ruleset: Option<String>,
        /// How many rules the scanner reported evaluating.
        #[serde(skip_serializing_if = "Option::is_none")]
        rules_evaluated: Option<u64>,
    },
}

/// One producer result the run-entry vocabulary cannot carry.
///
/// Declared here rather than re-used from `quoin_evidence::adapters` because
/// that type is not `Serialize`: it is an internal shape, and a `derive` added
/// to it there would put a wire contract on a type nothing wires.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct UnrepresentedView {
    /// The producer's own identity for the result.
    pub symbol: String,
    /// The producer's own state name, verbatim.
    pub state: String,
    /// Why no run-entry outcome carries it.
    pub reason: String,
}

/// The request accepted by `evidence.record`.
///
/// One request for both record types on purpose. The choice between a run
/// record and a finding-shaped scan record is made by the ADAPTER REGISTRY
/// before anything is parsed, and splitting it into two operations would move
/// that choice to the caller — which is exactly how a scan ends up in `runs/`
/// with the clean-versus-unrun distinction lost at the point of intake
/// (FR-034).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RecordRequest {
    /// The repository root.
    pub repo: String,
    /// The suite that ran.
    pub suite: String,
    /// The commit it ran at.
    pub commit: String,
    /// Tool and version, as it identifies itself.
    pub tool: String,
    /// The declared evidence kind, when the caller names one.
    #[serde(default)]
    pub kind: Option<String>,
    /// Separation facts for every binding this record creates.
    #[serde(default)]
    pub lineage: Option<EvidenceLineage>,
    /// ISO-8601. Supplied, never read from the clock here.
    pub timestamp: String,
    /// An explicit adapter name.
    #[serde(default)]
    pub adapter: Option<String>,
    /// The producer document's text, as the caller read it.
    pub results: String,
    /// Obligation ids a finding-shaped scan was run to check.
    ///
    /// Ignored on the run path, where the obligations discharged are derived
    /// from the trace ids the entries carry.
    #[serde(default)]
    pub discharges: Vec<String>,
    /// The obligations as quire derives them today.
    #[serde(default)]
    pub obligations: Vec<quoin_quire_types::Obligation>,
}

/// The payload `evidence.record` writes to stdout.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum RecordPayload {
    /// A run record was written.
    Run {
        /// Where it was written, as an **absolute** path.
        run_path: String,
        /// Obligations bound for the first time, sorted.
        bound: Vec<ObligationId>,
        /// Obligations whose statement changed since they were bound, sorted.
        suspect: Vec<ObligationId>,
        /// Trace ids matching no derived obligation, sorted.
        unmatched: Vec<String>,
        /// Results no run outcome represents.
        #[serde(skip_serializing_if = "Option::is_none")]
        unrepresented: Option<Vec<UnrepresentedView>>,
    },
    /// A finding-shaped scan record was written.
    Scan {
        /// Where it was written, as an **absolute** path.
        scan_path: String,
        /// The transcribed findings.
        findings: Vec<Finding>,
        /// Obligations newly bound by the scan, sorted.
        bound: Vec<String>,
        /// Named `--discharges` ids no obligation states, sorted.
        unknown: Vec<String>,
        /// Whether the scan evaluated no rules, and so bound nothing.
        vacuous: bool,
        /// How many rules the scanner reported evaluating.
        #[serde(skip_serializing_if = "Option::is_none")]
        rules_evaluated: Option<u64>,
    },
}

/// The request accepted by `evidence.trust_decision`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TrustDecisionRequest {
    /// The repository root.
    pub repo: String,
    /// The decision document, as the caller read it.
    pub decision: serde_json::Value,
}

/// The payload `evidence.trust_decision` writes to stdout.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TrustDecisionPayload {
    /// Where the decision was written, as an **absolute** path.
    pub path: String,
    /// Its effective state, as compared against the observed context.
    pub assessment: TrustAssessment,
}

/// The payload `evidence.trust_assessments` writes to stdout.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TrustAssessmentsPayload {
    /// One assessment per readable decision, in file-name order.
    pub assessments: Vec<TrustAssessment>,
    /// Paths of decisions that would not parse or would not validate.
    ///
    /// Named, not counted, and not fatal: one malformed decision must not hide
    /// every other reliance judgement in the store.
    pub unreadable: Vec<String>,
}

/// The request accepted by `evidence.inspect_mocks`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct InspectMocksRequest {
    /// The repository root.
    pub repo: String,
    /// The suite whose test source is inspected.
    pub suite: String,
    /// Full commit sha whose source is inspected.
    pub commit: String,
    /// The inspecting tool and its version.
    pub tool: String,
    /// ISO-8601 inspection time.
    pub timestamp: String,
    /// Inspect and report, writing no observation record.
    #[serde(default)]
    pub dry_run: bool,
}

/// The payload `evidence.inspect_mocks` writes to stdout.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct InspectMocksPayload {
    /// Where the record was written as an **absolute** path, or `null` on a
    /// dry run.
    pub path: Option<String>,
    /// One entry per substituting symbol, sorted.
    pub injections: Vec<MockInjection>,
}

/// The request accepted by `evidence.record_experiment` and
/// `evidence.record_operational`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AssuranceRecordRequest {
    /// The repository root.
    pub repo: String,
    /// The record as its producer published it.
    pub document: serde_json::Value,
}

/// The payload `evidence.record_experiment` and `evidence.record_operational`
/// write to stdout.
///
/// The record itself is an opaque value rather than the typed shape: the
/// caller prints it and reads `recordId` off it, and the two record families
/// have different bodies under one operation shape.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AssuranceRecordPayload {
    /// Where it was written, as an **absolute** path.
    pub path: String,
    /// Whether this call created it, or found identical bytes already there.
    pub created: bool,
    /// The stored record, including the identity derived from its content.
    pub record: serde_json::Value,
}

/// The request accepted by `evidence.audit_inputs`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AuditInputsRequest {
    /// The repository root.
    pub repo: String,
    /// HEAD, when the caller could resolve one.
    ///
    /// `None` is not an error: a repository with no git history still has a
    /// store, and the mock-inspection input is then empty because only records
    /// **at that commit** count.
    #[serde(default)]
    pub head_commit: Option<String>,
    /// The profile-selected independence policy, when one was given.
    #[serde(default)]
    pub independence_policy: Option<IndependencePolicy>,
}

/// The payload `evidence.audit_inputs` writes to stdout.
///
/// Everything the pure auditor needs from the store, in one call. One
/// subprocess per obligation is what a per-question operation would have cost,
/// and the auditor asks two of its questions — scan vacuity and profile
/// independence — inside per-obligation loops.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AuditInputsPayload {
    /// The binding graph.
    pub bindings: Vec<Binding>,
    /// The newest run per recorded suite, by timestamp.
    pub runs: Vec<RunRecord>,
    /// The newest finding-shaped scan per recorded suite, by timestamp.
    pub scans: Vec<FindingRecord>,
    /// Mock injections recorded at exactly `head_commit`.
    pub injections: Vec<MockInjection>,
    /// The suites that have an inspection record at exactly `head_commit`.
    ///
    /// Distinct from `injections`: an empty completed inspection means "looked
    /// and found none", no record at all means "nobody looked".
    pub mock_inspection_suites: Vec<SuiteId>,
    /// Suites whose newest scan evaluated no rules.
    ///
    /// Answered here rather than asked per obligation. `None` — the tool
    /// reported no rule count — is not in this list, which is the same silence
    /// `scan_is_vacuous` returns for it.
    pub vacuous_scan_suites: Vec<SuiteId>,
    /// One assessment per policy requirement, over that requirement's
    /// obligation's bindings. Empty when no policy was given.
    pub independence: Vec<IndependenceAssessment>,
    /// Store paths that would not parse, named rather than counted.
    pub skipped: Vec<String>,
}

/// The payload `evidence.read_baseline` writes to stdout.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ReadBaselinePayload {
    /// The baseline, or `null` when none has been accepted.
    ///
    /// `null` is the fact `--ratchet` turns on: a missing baseline degrades the
    /// run to a full report, and labelling that full report "new violations
    /// only" told a day-one reader their whole backlog was new (#169).
    pub baseline: Option<BaselineFile>,
    /// Where it would be, as an **absolute** path, for the notice that names it.
    pub path: String,
}

/// The request accepted by `evidence.write_baseline`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct WriteBaselineRequest {
    /// The repository root.
    pub repo: String,
    /// The commit the baseline is accepted at.
    pub commit: String,
    /// The accepted findings as `<kind>:<obligation>`.
    pub accepted: Vec<String>,
}

/// The payload `evidence.write_baseline` writes to stdout.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct WriteBaselinePayload {
    /// Where it was written, as an **absolute** path.
    pub path: String,
}
