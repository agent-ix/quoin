/**
 * The evidence store across the `quoin-core` boundary (quoin#458, Stage 8 of
 * quoin#373).
 *
 * Replaces `src/evidence/` entirely. Everything it decided — what an adapter
 * reads, which record type a producer document becomes, when a binding is
 * created and when it turns suspect, what a vacuous scan is, how independence
 * is assessed against a profile policy, which records `gc` collects, and what a
 * trust decision must declare — is now decided by `quoin-evidence`. Nothing in
 * this file reimplements any of it; it builds requests and reads payloads.
 *
 * # Paths
 *
 * **Every path a payload carries is absolute.** `quoin-evidence` names no host
 * capability, so its own return values are store-relative; the join happens on
 * the far side, in one function, against the root the evidence host reports
 * (`quoin_core::ops::evidence::absolute`). The retained implementation returned
 * absolute paths and the commands print them for a person to open, so a
 * relative path there would be a wrong path.
 *
 * # The constants below
 *
 * `MUTATION_SCORE_METRIC`, `STORE_SCHEMA_VERSION` and the four store file names
 * are declared here as plain values, not fetched. They are needed in pure code
 * — `src/auditor/audit.ts` compares a metric name inside a loop, and the
 * auditor must never spawn a subprocess — and a constant that costs a process
 * to read is a constant callers will copy instead.
 *
 * They are still not a second opinion: `evidence.store_facts` serves the
 * boundary's own values, and `tests/core-evidence.test.ts` asserts each of
 * these against that payload. This is the `REGISTRY_KEY_ORDER` arrangement in
 * {@link ./modules.ts} — state it locally, pin it against the authority — and
 * the test is what reports a drift, rather than a silent disagreement about
 * where `bindings.json` lives.
 */

import { join } from "node:path";

import { storeRoot } from "../store/paths.js";
import { carriesPayload, runCoreAllowFailure } from "./exec.js";
import type {
  AffirmPayload,
  AffirmRequest,
  AssuranceRecordPayload,
  AssuranceRecordRequest,
  AuditInputsPayload,
  AuditInputsRequest,
  GcPayload,
  GcRequest,
  IndependencePolicy,
  InspectMocksPayload,
  InspectMocksRequest,
  ParseLineagePayload,
  ParseLineageRequest,
  ParsePolicyPayload,
  ParsePolicyRequest,
  ParseResultsPayload,
  ParseResultsRequest,
  ReadBaselinePayload,
  RecordPayload,
  RecordRequest,
  RepoRequest,
  StoreFactsPayload,
  TrustAssessmentsPayload,
  TrustDecisionPayload,
  TrustDecisionRequest,
  WriteBaselinePayload,
  WriteBaselineRequest,
} from "./types.js";

export type {
  AffirmPayload,
  AssuranceRecordPayload,
  AuditInputsPayload,
  BaselineFile,
  Binding,
  EvidenceLineage,
  Finding,
  FindingRecord,
  GcPayload,
  IndependenceAssessment,
  IndependencePolicy,
  InspectMocksPayload,
  MockInjection,
  ParseResultsPayload,
  ReadBaselinePayload,
  RecordPayload,
  RunEntry,
  RunRecord,
  TrustAssessment,
  TrustAssessmentsPayload,
  TrustDecisionPayload,
  UnrepresentedView,
} from "./types.js";

/**
 * Every adapter `--adapter` accepts, run-shaped then finding-shaped.
 *
 * Declared rather than fetched for the same reason as the constants below, and
 * one more: it is read by an oclif `static flags` initialiser, which runs when
 * the command module is IMPORTED. Fetching it would spawn `quoin-core` to print
 * `quoin --help`.
 */
export const ADAPTER_NAMES: readonly string[] = [
  "entries",
  "junit",
  "cargo-mutants",
  "sbom",
  "agent-eval",
  "contract-conformance",
  "differential-report",
  "sarif",
  "audit-script",
  "cargo-audit",
];

/** The metric a `cargo-mutants` score is recorded under. */
export const MUTATION_SCORE_METRIC = "mutation-score";

/** The schema version stamped into every record envelope. */
export const STORE_SCHEMA_VERSION = 1;

/** The binding graph, as an absolute path. */
export function bindingsPath(repo: string): string {
  return join(storeRoot(repo), "bindings.json");
}

/** The ratchet baseline, as an absolute path. */
export function baselinePath(repo: string): string {
  return join(storeRoot(repo), "baseline.json");
}

/** The authored suite registry, as an absolute path. */
export function suitesPath(repo: string): string {
  return join(storeRoot(repo), "suites.md");
}

/** The authored inspection register, as an absolute path. */
export function inspectionsPath(repo: string): string {
  return join(storeRoot(repo), "inspections.md");
}

/** One boundary call, with the failure surfaced rather than swallowed. */
function call<T>(op: string, request: unknown): T {
  const result = runCoreAllowFailure(op, request);
  if (!carriesPayload(result.exitCode)) {
    const detail = result.diagnostics
      .map((d) => `${d.code}: ${d.message}`)
      .join("\n");
    throw new Error(
      detail ||
        `quoin-core ${op} exited ${result.exitCode} with no diagnostic.`,
    );
  }
  return result.payload as T;
}

/**
 * The constants and store-relative layout the boundary itself states.
 *
 * Read by `tests/core-evidence.test.ts` to pin the values declared above, and
 * by `quoin evidence record --help` for the adapter list. Not memoised: the
 * one caller in the command path asks once per process.
 */
export function storeFacts(): StoreFactsPayload {
  return call<StoreFactsPayload>("evidence.store_facts", {});
}

/** Drop run and scan records nothing references. Paths are absolute. */
export function gc(repo: string, dryRun = false): string[] {
  return call<GcPayload>("evidence.gc", {
    repo,
    dry_run: dryRun,
  } satisfies GcRequest).deleted;
}

/** Re-affirm a binding after the statement it was made against changed. */
export function affirm(request: AffirmRequest): AffirmPayload {
  return call<AffirmPayload>("evidence.affirm", request);
}

/** Validate one evidence-lineage document. */
export function parseLineage(text: string): ParseLineagePayload["lineage"] {
  return call<ParseLineagePayload>("evidence.parse_lineage", {
    text,
  } satisfies ParseLineageRequest).lineage;
}

/**
 * Validate one independence policy, and check it against today's obligations.
 *
 * Both questions in one call because they have one answer: a policy naming an
 * obligation nothing derives reports every requirement as vacuously assessed.
 */
export function parsePolicy(
  text: string,
  knownObligations: string[] = [],
): IndependencePolicy {
  return call<ParsePolicyPayload>("evidence.parse_policy", {
    text,
    known_obligations: knownObligations,
  } satisfies ParsePolicyRequest).policy;
}

/** Read one producer document through the selected adapter. */
export function parseResults(
  request: ParseResultsRequest,
): ParseResultsPayload {
  return call<ParseResultsPayload>("evidence.parse_results", request);
}

/**
 * Transcribe one suite run, or one finding-shaped scan, and bind what it
 * discharged.
 *
 * One operation for both record types, because the choice between them is made
 * by the adapter registry before anything is parsed. The payload is tagged, so
 * the caller reads which it got rather than inferring it from which fields are
 * present.
 */
export function record(request: RecordRequest): RecordPayload {
  return call<RecordPayload>("evidence.record", request);
}

/** Transcribe one trust decision and report its effective state. */
export function trustDecision(
  repo: string,
  decision: unknown,
): TrustDecisionPayload {
  return call<TrustDecisionPayload>("evidence.trust_decision", {
    repo,
    decision,
  } satisfies TrustDecisionRequest);
}

/** Every readable trust decision's assessment, and the ones that would not read. */
export function trustAssessments(repo: string): TrustAssessmentsPayload {
  return call<TrustAssessmentsPayload>("evidence.trust_assessments", {
    repo,
  } satisfies RepoRequest);
}

/** Inspect a suite's test source and record the mock substitutions found. */
export function inspectMocks(
  request: InspectMocksRequest,
): InspectMocksPayload {
  return call<InspectMocksPayload>("evidence.inspect_mocks", request);
}

/**
 * The content-derived identity of one stored assurance record.
 *
 * The record itself crosses as an opaque value: the two families have
 * different bodies under one operation shape, and the caller prints the record
 * and reads one field off it. Narrowing happens here rather than at each call
 * site casting.
 */
export function recordId(record: unknown): string {
  const id = (record as { recordId?: unknown } | null)?.recordId;
  if (typeof id !== "string") {
    throw new Error("the stored record carries no `recordId`");
  }
  return id;
}

/** Publish one content-addressed experiment record. */
export function recordExperiment(
  repo: string,
  document: unknown,
): AssuranceRecordPayload {
  return call<AssuranceRecordPayload>("evidence.record_experiment", {
    repo,
    document,
  } satisfies AssuranceRecordRequest);
}

/** Publish one content-addressed operational evidence record. */
export function recordOperational(
  repo: string,
  document: unknown,
): AssuranceRecordPayload {
  return call<AssuranceRecordPayload>("evidence.record_operational", {
    repo,
    document,
  } satisfies AssuranceRecordRequest);
}

/**
 * Everything the pure auditor needs from the store, in one call.
 *
 * One operation and not eight. The auditor asks two of its questions — scan
 * vacuity and profile independence — inside per-obligation loops, so a
 * per-question operation would have cost one subprocess per obligation.
 */
export function auditInputs(
  repo: string,
  headCommit?: string,
  independencePolicy?: IndependencePolicy,
): AuditInputsPayload {
  return call<AuditInputsPayload>("evidence.audit_inputs", {
    repo,
    ...(headCommit === undefined ? {} : { head_commit: headCommit }),
    ...(independencePolicy === undefined
      ? {}
      : { independence_policy: independencePolicy }),
  } satisfies AuditInputsRequest);
}

/**
 * The ratchet baseline, and the absolute path it would be at.
 *
 * The path comes back whether or not the baseline exists, because the notice
 * for a missing one has to name it (#169).
 */
export function readBaseline(repo: string): ReadBaselinePayload {
  return call<ReadBaselinePayload>("evidence.read_baseline", {
    repo,
  } satisfies RepoRequest);
}

/** Accept a set of findings as the ratchet baseline. */
export function writeBaseline(
  repo: string,
  commit: string,
  accepted: string[],
): string {
  return call<WriteBaselinePayload>("evidence.write_baseline", {
    repo,
    commit,
    accepted,
  } satisfies WriteBaselineRequest).path;
}
