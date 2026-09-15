/**
 * The quoin-core boundary types (FR-097). GENERATED — DO NOT EDIT.
 *
 * Written by `quoin-schemas/quoin-schemas-gen` from the JSON Schema `schemars`
 * reads off the canonical Rust declarations in
 * `rust/crates/quoin-core/src/protocol.rs`, `.../src/ops/` and the
 * domain crates those operations answer from.
 * Rust is the source of truth: where this file and a Rust type
 * disagree, the Rust type is right and this file is stale.
 *
 * Regenerate with `make types`. A hand edit does not survive review
 * and does not survive the gate: `quoin-schemas` asserts the digest
 * below against BOTH these bytes and a fresh render, so an edit here
 * fails `make rust-test` at an unchanged path, and a Rust type change
 * that was never regenerated fails it too.
 *
 * The machine-readable provenance is the `CORE_TYPES_PROVENANCE`
 * record below, and it is the ONLY copy: a second, prose copy up
 * here would be one more thing that can disagree with the artefact
 * it describes.
 */

/**
 * What produced this file, readable by the TypeScript side.
 *
 * Generated status is established by THIS record, not by the
 * artefact's directory name (FR-097-CON-2): moving the file does not
 * make it hand-written, and writing a file into a `generated/`
 * directory does not make it generated.
 */
export const CORE_TYPES_PROVENANCE = {
  generator: "quoin-schemas/quoin-schemas-gen",
  generatorVersion: "0.9.0",
  sourceSchemaSha256:
    "ee3e514907f97891b0ff799b6fc649ad69a9d58352bfad8cb932731c2808f2ed",
} as const;

/**
 * The advisor's verdict for one obligation.
 */
export interface Advice {
  /**
   * The authored method, normalized.
   */
  authored?: string | null;
  /**
   * True when no rule matched anything. The honest outcome — an advisor
   * that recommends `Test` because it found nothing is the habit this
   * replaces.
   */
  inconclusive: boolean;
  /**
   * A genuine disagreement — *did you mean to inspect this rather than test
   * it?* Set when the authored method **is a declared method or class**, is
   * not among the recommendations, AND the advisor had rules to go on.
   *
   * Never set for an uncatalogued value (agent-ix/quoin#168): a word the
   * catalog has never heard of is a vocabulary problem, not a choice the
   * advisor disagrees with.
   */
  mismatch: boolean;
  /**
   * The obligation the verdict is about.
   */
  obligation: string;
  /**
   * Deterministic recommendations, strongest (most rules matched) first.
   */
  recommended: Recommendation[];
  /**
   * The authored value is in neither the catalog's method set nor its class
   * set, per the engine's own `uncatalogued-verification-method` diagnostic.
   *
   * Orthogonal to `inconclusive`: the vocabulary fact does not disappear
   * because the advisor was silent.
   */
  uncatalogued: boolean;
}

/**
 * One advice row per obligation, plus what the vocabulary join could tell.
 */
export interface AdvisePayload {
  /**
   * Advice, in the order the obligations arrived.
   */
  advice: Advice[];
  /**
   * True when the engine emitted the diagnostic without a `value`.
   *
   * The caller warns on it. It is carried separately from the advice
   * because it is a fact about the ENGINE, not about any obligation: every
   * row still got an answer, and the answer is two-state rather than three.
   */
  degraded: boolean;
}

/**
 * What `quoin advise` sends: one request for every obligation, not one each.
 */
export interface AdviseRequest {
  /**
   * The binding graph from the store.
   */
  bindings?: Binding[];
  /**
   * The merged verification-method catalog.
   */
  catalog: MethodCatalog;
  /**
   * The coverage payload's diagnostics, for the uncatalogued-method join.
   */
  diagnostics?: CoverageDiagnostic[];
  /**
   * The obligations the coverage payload carried, in its order.
   */
  obligations: Obligation[];
  /**
   * Every run record the store holds.
   */
  runs?: RunRecord[];
  /**
   * Obligation id → what `quire properties` classified the criterion as.
   *
   * A second engine call the caller makes, because the coverage payload
   * carries neither field and this domain spawns nothing.
   */
  shapes?: Record<string, PropertyShape>;
}

/**
 * The payload `evidence.affirm` writes to stdout.
 */
export interface AffirmPayload {
  /**
   * The commit, echoed.
   */
  commit: string;
  /**
   * Whether any binding matched. Nothing is written when none did.
   */
  found: boolean;
  /**
   * The obligation, echoed.
   */
  obligation: string;
  /**
   * The statement hash now stamped on the binding.
   */
  statement_hash: string;
  /**
   * Who affirmed, echoed.
   */
  who: string;
}

/**
 * The request accepted by `evidence.affirm`.
 */
export interface AffirmRequest {
  /**
   * The commit the affirmation is made at.
   */
  commit: string;
  /**
   * Why the evidence still holds, when they said.
   */
  note?: string | null;
  /**
   * The obligation whose binding is re-affirmed.
   */
  obligation: string;
  /**
   * The repository root.
   */
  repo: string;
  /**
   * The statement hash as quire derives it **today**.
   */
  statement_hash: string;
  /**
   * One suite, when the reviewer means only one.
   */
  suite?: string | null;
  /**
   * Who is affirming. Recorded verbatim.
   */
  who: string;
}

/**
 * Someone re-affirming a binding after the statement it was made against
 * changed.
 */
export interface Affirmation {
  /**
   * The commit they affirmed at.
   */
  commit: Commit;
  /**
   * Why, when they said.
   */
  note?: string | null;
  /**
   * Who affirmed.
   */
  who: string;
}

/**
 * The argument's lifecycle state. Closed, because the retained code validates.
 *
 * quoin#425 asked whether a TypeScript union ports as a closed Rust type, and
 * answered "only where the retained code checks membership at run time".
 * `Finding.kind` was open because the retained renderer interpolates whatever
 * arrives. These go through `literal()`, which checks and **throws**, so
 * refusing an unlisted value IS the retained behaviour. Same test, different
 * answer, because the code differs.
 */
export type ArgumentStatus = "proposed" | "active" | "retired";

/**
 * The argument's identity, without its body.
 */
export interface ArgumentSummary {
  /**
   * `AA-<digits>`.
   */
  id: string;
  /**
   * Who owns it.
   */
  owner: string;
  /**
   * The governing profile.
   */
  profile: string;
  /**
   * Its lifecycle state. Anything but `active` opens the top claim.
   */
  status: ArgumentStatus;
  /**
   * Its title.
   */
  title: string;
}

/**
 * What to assess, as content the CALLER read.
 *
 * The wire form of `AssessOptions` (quoin#445). `bundle_root` is a LABEL
 * here, echoed into the report so the user reads the root the command looked
 * in; nothing in the library half turns it back into a path.
 */
export interface AssessInput {
  /**
   * The root the caller walked, echoed into the report unchanged.
   */
  bundle_root: string;
  /**
   * Every document the caller read.
   */
  documents: DocumentSource[];
  /**
   * Every module the caller located, with its manifest and schemas.
   */
  modules: ModuleSource[];
  /**
   * Promote an admitted gap to a failing verdict.
   */
  strict: boolean;
  /**
   * Documents the caller could not read, and why. Reported alongside the
   * ones whose frontmatter does not parse: a file the walk found and could
   * not open is the same kind of gap as one whose YAML is broken, and
   * dropping it here would turn a broken bundle into a clean one.
   */
  unreadable?: UnreadableDocument[];
}

/**
 * An assumption's declared state.
 */
export type AssumptionStatus = "open" | "accepted" | "invalidated";

/**
 * One assumption, as it reads at the stated instant.
 */
export interface AssumptionView {
  /**
   * What the author declared, kept beside the derived status so a reader
   * can see that "accepted" and "supported" are different facts.
   */
  declaredStatus: AssumptionStatus;
  /**
   * The assumption's id.
   */
  id: string;
  /**
   * Who owns it.
   */
  owner: string;
  /**
   * Why it is open. Omitted when it is not.
   */
  reason?: string | null;
  /**
   * The authored review instant.
   */
  reviewBy: string;
  /**
   * What is assumed.
   */
  statement: string;
  /**
   * Open when it is not accepted, or when its review has come due.
   */
  status: ViewStatus;
}

/**
 * The payload `evidence.record_experiment` and `evidence.record_operational`
 * write to stdout.
 *
 * The record itself is an opaque value rather than the typed shape: the
 * caller prints it and reads `recordId` off it, and the two record families
 * have different bodies under one operation shape.
 */
export interface AssuranceRecordPayload {
  /**
   * Whether this call created it, or found identical bytes already there.
   */
  created: boolean;
  /**
   * Where it was written, as an **absolute** path.
   */
  path: string;
  /**
   * The stored record, including the identity derived from its content.
   */
  record: unknown;
}

/**
 * The request accepted by `evidence.record_experiment` and
 * `evidence.record_operational`.
 */
export interface AssuranceRecordRequest {
  /**
   * The record as its producer published it.
   */
  document: unknown;
  /**
   * The repository root.
   */
  repo: string;
}

/**
 * One thing wrong with the evidence for one obligation.
 *
 * # Three fields are read by the assurance view; twelve are written
 *
 * This type began (quoin#384) as the assurance view's *reader*: `build_case`
 * keys findings by `obligation` and renders `` `{kind}: {summary}` ``, and
 * nothing else. quoin#383 ports the producer into the same workspace, and the
 * producer writes all twelve. One home, not two — a second `Finding` would be
 * two declarations of one wire shape, and the pair would drift.
 *
 * The nine that only the producer writes are `Option` with
 * `skip_serializing_if`, so a finding the view constructs still serialises to
 * the same three keys it always did. The retained source spreads
 * `...(located.path ? { path: located.path } : {})` at every construction
 * site; a `null` on the wire would be a byte the store has never held.
 *
 * # Unknown fields are accepted, and must be
 *
 * There is no `deny_unknown_fields`. The caller hands this type the auditor's
 * own output verbatim, and a future field is not a parse error.
 */
export interface AuditFinding {
  /**
   * Where to make the change.
   */
  changeTarget?: string | null;
  /**
   * What kind of finding it is.
   *
   * # Why this is a `String` and not an enum
   *
   * The retained type declares twelve spellings in a closed union, and it
   * is tempting to mirror that here — the spellings *are* observable,
   * because `because` interpolates this value into the payload.
   *
   * But a TypeScript union is erased at run time. `buildCase` performs no
   * validation: it interpolates whatever string arrives. A closed Rust enum
   * would therefore **refuse input the retained implementation accepts**,
   * turning a payload into a `BadRequest` — a behavioural difference, in
   * the direction of the port being stricter than the thing it replaces.
   *
   * This is the same defect class as the suffix enumeration in
   * `quoin_assurance::requirement_of`: writing the type from what the
   * declaration *says* rather than from what the code *does*. There the
   * enumeration dropped every `-M-` obligation; here it would drop every
   * finding kind added after this file was written. The difftest case
   * `assurance/build-case-unknown-finding-kind` holds the property.
   *
   * The twelve the auditor mints today are named by
   * `FindingKind`(crate::FindingKind), for the reader's benefit and not
   * as a constraint.
   */
  kind: string;
  /**
   * The line within `path`.
   */
  line?: number | null;
  /**
   * The safe next step, when no fix is known.
   */
  nextDiagnosticStep?: string | null;
  /**
   * The obligation the finding is against. The view's join key.
   */
  obligation: string;
  /**
   * Repo-relative source locus, when the producer names one.
   */
  path?: string | null;
  /**
   * The fix, when one is known.
   */
  remedy?: string | null;
  /**
   * How serious it is, when the producer said.
   *
   * Optional because the assurance corpus holds findings without one: four
   * of the six findings in `quoin-assurance/tests/golden/cases.json` omit
   * the key entirely, and the view has always rendered them. The auditor
   * itself always writes one — `new` takes it by value, so a
   * finding minted through the constructor cannot lack it.
   */
  severity?: AuditSeverity | null;
  /**
   * What the structured action is about.
   *
   * The JSON finding contract requires a target and exactly one remedy or
   * safe diagnostic step whenever this is present.
   */
  subject?: string | null;
  /**
   * The finding's one-line summary, rendered verbatim into `because`.
   */
  summary: string;
  /**
   * The symbol the finding is about.
   */
  symbol?: string | null;
}

/**
 * Everything one audit reads.
 */
export interface AuditInput {
  /**
   * The binding graph from the store.
   */
  bindings?: Binding[];
  /**
   * The merged verification-method catalog, for method conformance.
   */
  catalog?: MethodCatalog | null;
  /**
   * Commit being audited, for freshness.
   */
  headCommit?: string | null;
  /**
   * One assessment per policy requirement, already made against the
   * bindings.
   *
   * Supplied rather than computed for the same reason as `injections`: the
   * assessment is the store's judgement over the binding graph, and the
   * auditor reads.
   */
  independence?: IndependenceAssessment[] | null;
  /**
   * Exact obligations and separation axes selected by an `AssuranceProfile`.
   */
  independencePolicy?: IndependencePolicy | null;
  /**
   * What each suite's tests INJECT in place of real behaviour (#204).
   *
   * Supplied by the caller because the auditor reads the store, not source.
   * An absent list means "nobody looked", never "nothing was mocked": the
   * result is recorded under `unevaluated`, not counted as healthy.
   */
  injections?: MockInjection[] | null;
  /**
   * Suites with a current completed inspection, including clean inspections.
   */
  mockInspectionSuites?: string[] | null;
  /**
   * Criticality values that demand two independent methods.
   */
  multiplicityRequires?: string[] | null;
  /**
   * Criticality value → the mutation score its obligations must reach.
   *
   * **Unset by default**, for the reason CR-008 removed the hardcoded
   * `multiplicityRequires: ["P0"]`: a built-in floor is a rule that fires on
   * everything the moment a criticality column appears, and nobody chose it.
   */
  mutationFloor?: Record<string, number> | null;
  /**
   * Obligations as quire derives them today.
   */
  obligations?: Obligation[];
  /**
   * Every run record the store holds, newest per suite.
   */
  runs?: RunRecord[];
  /**
   * Every finding-shaped scan record the store holds, newest per suite.
   *
   * Separate from `runs` because the two answer different questions. A scan
   * has no symbols and no pass/fail, so every check that reasons over
   * `entries` is meaningless against it.
   */
  scans?: FindingRecord[] | null;
  /**
   * Suites whose newest finding-shaped scan evaluated no rules (FR-034).
   *
   * Answered by the store rather than recomputed here, and answered as a
   * closed list: a tool that reported no rule count is NOT in it. An absent
   * list means nothing was asked, never that nothing was vacuous.
   */
  vacuousScanSuites?: string[] | null;
}

/**
 * The payload `evidence.audit_inputs` writes to stdout.
 *
 * Everything the pure auditor needs from the store, in one call. One
 * subprocess per obligation is what a per-question operation would have cost,
 * and the auditor asks two of its questions — scan vacuity and profile
 * independence — inside per-obligation loops.
 */
export interface AuditInputsPayload {
  /**
   * The binding graph.
   */
  bindings: Binding[];
  /**
   * One assessment per policy requirement, over that requirement's
   * obligation's bindings. Empty when no policy was given.
   */
  independence: IndependenceAssessment[];
  /**
   * Mock injections recorded at exactly `head_commit`.
   */
  injections: MockInjection[];
  /**
   * The suites that have an inspection record at exactly `head_commit`.
   *
   * Distinct from `injections`: an empty completed inspection means "looked
   * and found none", no record at all means "nobody looked".
   */
  mock_inspection_suites: SuiteId[];
  /**
   * The newest run per recorded suite, by timestamp.
   */
  runs: RunRecord[];
  /**
   * The newest finding-shaped scan per recorded suite, by timestamp.
   */
  scans: FindingRecord[];
  /**
   * Store paths that would not parse, named rather than counted.
   */
  skipped: string[];
  /**
   * Suites whose newest scan evaluated no rules.
   *
   * Answered here rather than asked per obligation. `None` — the tool
   * reported no rule count — is not in this list, which is the same silence
   * `scan_is_vacuous` returns for it.
   */
  vacuous_scan_suites: SuiteId[];
}

/**
 * The request accepted by `evidence.audit_inputs`.
 */
export interface AuditInputsRequest {
  /**
   * HEAD, when the caller could resolve one.
   *
   * `None` is not an error: a repository with no git history still has a
   * store, and the mock-inspection input is then empty because only records
   * **at that commit** count.
   */
  head_commit?: string | null;
  /**
   * The profile-selected independence policy, when one was given.
   */
  independence_policy?: IndependencePolicy | null;
  /**
   * The repository root.
   */
  repo: string;
}

/**
 * What an audit answered.
 */
export interface AuditPayload {
  /**
   * Profile-selected evidence independence, when a policy was given.
   *
   * # Why this is lifted out of the report
   *
   * `quoin-auditor` writes it into `AuditReport`'s passthrough map, and
   * that ruling is about READING retained store bytes wider than the
   * declared type (quoin#383/#385): a captured corpus holds an
   * `independence` member that is not an assessment at all, so the retained
   * type cannot declare one.
   *
   * The boundary is not reading retained bytes. It is answering a request
   * it just computed, and it knows exactly what it put there — so the
   * member is MOVED here and typed, rather than copied. Copied would be the
   * same list encoded twice, the shape FR-097 forbids, and left in place it
   * would reach TypeScript as an undeclared key no generated type carries.
   */
  independence?: IndependenceAssessment[] | null;
  /**
   * The whole report: findings, healthy, unevaluated.
   */
  report: AuditReport;
  /**
   * The findings that survived the ratchet, when one was applied.
   *
   * `None` when the request carried no baseline — the caller then reports
   * `report.findings`. Present-and-equal would be the same list encoded
   * twice, which is the shape FR-097 forbids.
   */
  reported?: AuditFinding[] | null;
}

/**
 * The audit result, ordered so the same input yields the same report.
 *
 * # Unknown fields are accepted
 *
 * Same reader posture as `Finding`: this deserialises whatever the auditor
 * wrote, including fields a later version adds.
 */
export interface AuditReport {
  /**
   * Every finding, sorted by obligation then kind.
   */
  findings: AuditFinding[];
  /**
   * Obligations with a binding whose hash still matches, sorted.
   */
  healthy: string[];
  /**
   * Checks that could not run, sorted by obligation then check.
   */
  unevaluated: UnevaluatedCheck[];
}

/**
 * What `quoin assurance` and `quoin evidence audit` send.
 */
export interface AuditRequest {
  /**
   * The accepted-finding keys a `--ratchet` run was given.
   *
   * `None` is not `Some([])`. Absent means no baseline was read and the
   * full report is the answer; an empty baseline means one was read and
   * accepted nothing, so every finding is new. Labelling the first case
   * "new violations only" told a day-one reader their whole backlog was new
   * (agent-ix/quoin#169), so the two stay distinguishable on the wire.
   */
  accepted?: string[] | null;
  /**
   * Everything the audit reads, assembled by the caller.
   */
  input: AuditInput;
}

/**
 * How serious one finding is.
 *
 * # Why this is a newtype and not a closed enum
 *
 * `src/auditor/audit.ts:34` declares `type Severity = "low" | "medium" |
 * "high"`, and mirroring that as a three-variant Rust enum is the obvious
 * move. It is also the one this workspace has already paid for once: PR #492
 * declared exactly that enum for the graph-analysis port, and the workspace
 * gate then refused a finding `build_case` has always accepted, because
 * `quoin-assurance`'s captured corpus carries `severity: "error"`.
 *
 * **A TypeScript type is not a runtime check.** The union is erased before
 * any value reaches the wire, nothing between the producer and here validates
 * it, and the retained corpora are wider than the declaration. Grepped across
 * `rust/`, `corpus/` and `tests/` before this type was written, the severities
 * actually retained are `error`, `medium`, `warning`, `high`, `advisory`,
 * `violation`, `yanked`, `unmaintained`, `vulnerability` and `unsound` — ten
 * spellings, of which the declaration admits two.
 *
 * Where the retained data is wider than the declared type, the retained data
 * wins. `LOW`, `MEDIUM` and `HIGH` name
 * the three the auditor itself mints; everything else round-trips unchanged.
 */
export type AuditSeverity = string;

/**
 * The complete authored view, as JSON.
 */
export interface AuthoredArgumentView {
  /**
   * The argument's identity.
   */
  argument: ArgumentSummary;
  /**
   * The instant the view was evaluated at, echoed verbatim from the
   * request — the authored spelling, not a re-formatting of the parse.
   */
  asOf: string;
  /**
   * In authored order.
   */
  assumptions: AssumptionView[];
  /**
   * In authored order.
   */
  challenges: ChallengeView[];
  /**
   * The clause discharge report, when one was supplied. Omitted when not.
   */
  discharge?: DischargeReport | null;
  /**
   * Carried through unchanged, so authority and independence stay visible.
   */
  participants: Participant[];
  /**
   * In authored order.
   */
  reasoning: ReasoningView[];
  /**
   * Carried through unchanged.
   */
  relationships: Relationship[];
  /**
   * Always `V1`.
   */
  schemaVersion: ViewSchemaVersion;
  /**
   * The claim and why it does or does not stand.
   */
  topClaim: TopClaimView;
  /**
   * In the order the decisions were supplied.
   */
  unusedDecisions: UnusedDecision[];
}

/**
 * The accepted violation set a ratchet compares against.
 *
 * `accepted` holds `<kind>:<obligation>` keys for *every*
 * finding kind. The original shape carried two named buckets, so five other
 * kinds could never appear in a baseline and `--ratchet` reported the whole
 * existing backlog for them — the outcome the mode exists to prevent
 * (agent-ix/quoin#105).
 */
export interface BaselineFile {
  /**
   * Accepted findings as `<kind>:<obligation>`, sorted.
   */
  accepted: string[];
  /**
   * The commit the baseline was accepted at.
   */
  commit: Commit;
  /**
   * Always `STORE_SCHEMA_VERSION`(super::STORE_SCHEMA_VERSION).
   */
  schemaVersion: number;
}

/**
 * The keys a baseline would accept.
 */
export interface BaselinePayload {
  /**
   * Every finding's key, sorted, as the baseline file records them.
   */
  accepted: string[];
}

/**
 * What `quoin evidence baseline` sends: the same audit, a different question.
 */
export interface BaselineRequest {
  /**
   * Everything the audit reads, assembled by the caller.
   */
  input: AuditInput;
}

/**
 * One obligation bound to the evidence that discharges it.
 *
 * `statement_hash_at_binding` is the entire suspect mechanism:
 * suspect detection is `current != statement_hash_at_binding` against the
 * obligation quire re-derives today. quire computes the hash; quoin only ever
 * compares.
 */
export interface Binding {
  /**
   * Re-affirmations recorded after a statement changed.
   */
  affirmations?: Affirmation[] | null;
  /**
   * The commit of the run that first discharged it.
   */
  commit: Commit;
  /**
   * Separation facts, for profile-selected independence checks.
   */
  lineage?: EvidenceLineage | null;
  /**
   * The obligation id the matrix keys on.
   */
  obligation: EvidenceObligationId;
  /**
   * The statement hash as it stood when the binding was first made.
   */
  statementHashAtBinding: StatementHash;
  /**
   * The suite that discharged it.
   */
  suite: SuiteId;
  /**
   * Symbols within that suite carrying the obligation's trace id.
   */
  symbols: SymbolId[];
}

/**
 * Everything the view is built from.
 */
export interface BuildAuthoredArgumentRequest {
  /**
   * The authored argument, unvalidated. `unknown` in the retained source,
   * and a `Value` here for the same reason: the seventeen
   * predicates in `argument` are the contract, and a typed field
   * would be a second door past them.
   */
  argument: unknown;
  /**
   * The evaluation instant. This layer never reads the wall clock.
   */
  asOf: string;
  /**
   * The sufficiency decisions, also unvalidated. The retained signature
   * types these and the retained BODY re-parses every one, which is what
   * FR-047-AC-5 exercises by casting an object with an extra key through
   * it; a `Vec<SufficiencyDecision>` here would have made that test
   * unwritable.
   */
  decisions: unknown[];
  /**
   * The clause discharge report, when the caller has one. Absent and `null`
   * mean the same thing — no report — because the retained signature makes
   * the parameter optional and a caller that omits it is not in error.
   */
  discharge?: DischargeReport | null;
}

/**
 * What `buildDischargeReport` is given.
 */
export interface BuildDischargeRequest {
  /**
   * Explicit evaluation instant; this layer never reads the wall clock.
   */
  asOf: string;
  /**
   * A validated `clause-binding-v1` report.
   */
  binding: ClauseBindingReport;
  /**
   * Unvalidated discharge facts. See the module header for why these are
   * untyped.
   */
  facts: unknown[];
}

/**
 * The report the command prints.
 */
export interface BundleAssessment {
  /**
   * The root that was read.
   */
  bundleRoot: string;
  /**
   * Every gap found, sorted.
   */
  findings: CompletenessFinding[];
  /**
   * One tally per declaration.
   */
  rollups: VocabularyRollup[];
  /**
   * Documents whose frontmatter could not be parsed.
   */
  unreadable: UnreadableDocument[];
  /**
   * Declarations whose vocabulary could not be resolved.
   */
  unresolved: UnresolvedDeclaration[];
  /**
   * The verdict.
   */
  verdict: Verdict;
  /**
   * Declarations found, so a report of zero findings can be told from zero
   * checks.
   */
  vocabularies: VocabularyName[];
}

/**
 * One document's frontmatter and body, as read from the bundle.
 */
export interface BundleDocument {
  /**
   * Everything after it.
   */
  body: string;
  /**
   * The parsed leading `---` block.
   */
  frontmatter: Record<string, unknown>;
  /**
   * Path, relative to the bundle root, with `/` separators.
   */
  path: string;
}

/**
 * The complete catalog projection.
 */
export interface Catalog {
  /**
   * Type names supplied by more than one module.
   */
  duplicates: Duplicate[];
  /**
   * Artifact and object entries, in manifest order.
   */
  entries: SpecCatalogEntry[];
  /**
   * The active modules, in candidate-root order after duplicate suppression.
   */
  modules: SpecModule[];
}

/**
 * A challenge's declared state.
 */
export type ChallengeStatus = "open" | "resolved" | "accepted-risk";

/**
 * One challenge, as it reads at the stated instant.
 */
export interface ChallengeView {
  /**
   * What the author declared.
   */
  declaredStatus: ChallengeStatus;
  /**
   * The authored expiry, when there is one.
   */
  expiresAt?: string | null;
  /**
   * The challenge's id.
   */
  id: string;
  /**
   * Who owns it.
   */
  owner: string;
  /**
   * Why it is open. Omitted when it is not.
   */
  reason?: string | null;
  /**
   * Always present, and empty when none were authored. The retained view
   * spreads `[...refs]` unconditionally, so this is NOT the optional field
   * the definition carries.
   */
  resolutionRefs: string[];
  /**
   * The objection.
   */
  statement: string;
  /**
   * Resolved only with a reference, and — for accepted risk — a current
   * expiry.
   */
  status: ChallengeViewStatus;
  /**
   * The node it targets.
   */
  target: string;
}

/**
 * A challenge answers "resolved", never "supported".
 *
 * A separate enum rather than a reuse of `ViewStatus`: the retained view
 * spells these two words and the renderer's tick mark keys off both, so
 * collapsing them would have changed the JSON to make the Rust tidier.
 */
export type ChallengeViewStatus = "resolved" | "open";

/**
 * One clause and the binder's verdict on it.
 */
export interface ClauseBinding {
  /**
   * The clause's id within its clause set.
   */
  clauseId: string;
  /**
   * The outputs the clause expects. May be empty.
   */
  expectedOutputs: string[];
  /**
   * How strongly it binds.
   */
  force: ClauseForce;
  /**
   * Whether it applies.
   */
  outcome: ClauseBindingOutcome;
  /**
   * May be empty, which is why the discharge report falls back to
   * `"applicability is unresolved"`.
   */
  reasons: ClauseBindingReason[];
}

/**
 * Whether a clause applies to the evaluated context.
 *
 * `Unresolved` is the load-bearing member and is deliberately **not** a
 * synonym for `NotBinding`: FR-046 keeps unresolved applicability outside the
 * discharge denominator entirely, because collapsing the two would let a
 * missing-context clause read as a decided one.
 */
export type ClauseBindingOutcome = "binding" | "not_binding" | "unresolved";

/**
 * Why the binder reached the outcome it did.
 */
export interface ClauseBindingReason {
  /**
   * A stable machine code for the reason.
   */
  code: string;
  /**
   * The context dimension the reason is about, when it names one.
   */
  dimension?: string | null;
  /**
   * The reason in the binder's own words. `buildDischargeReport` joins
   * these with `"; "` when a clause is unresolved.
   */
  message: string;
}

/**
 * Validated output of `quire clauses evaluate --format json`.
 */
export interface ClauseBindingReport {
  /**
   * Which clause set was evaluated.
   */
  clauseSet: ClauseSetKey;
  /**
   * The digest of the clause set that produced these verdicts.
   */
  clauseSetDigest: ClauseSetDigest;
  /**
   * Every clause the set declares, in the set's own order. That order is
   * significant downstream: FR-046's `unusedFacts` lists clause-ordered
   * entries before fact-ordered ones.
   */
  clauses: ClauseBinding[];
  /**
   * The evaluated context, verbatim. Ordered, so the discharge report that
   * copies it serialises to stable bytes.
   */
  context: Record<string, string>;
  /**
   * Which executable and engine produced the report, when stated.
   */
  engine?: EngineProvenance | null;
  /**
   * Always `clause-binding-v1`.
   */
  schemaVersion: ClauseBindingSchemaVersion;
}

/**
 * The one accepted `schemaVersion` of a clause-binding report.
 *
 * Closed rather than a `String`: a `clause-binding-v2` payload must fail to
 * read, not be read with v1 semantics and reported as a clean partition.
 */
export type ClauseBindingSchemaVersion = "clause-binding-v1";

/**
 * One clause, with the state the accounting put it in.
 */
export interface ClauseDischarge {
  /**
   * The clause's id.
   */
  clauseId: string;
  /**
   * Copied from the binding report. May be empty.
   */
  expectedOutputs: string[];
  /**
   * The fact that decided it, when one did. Absent, not null, otherwise.
   */
  fact?: DischargeFact | null;
  /**
   * Copied from the binding report.
   */
  force: ClauseForce;
  /**
   * Why, when there is a why. The key is **absent**, not null, when there
   * is not — the retained `entry()` spreads `...(reason ? { reason } : {})`.
   */
  reason?: string | null;
  /**
   * Where it landed.
   */
  state: DischargeState;
}

/**
 * How strongly a clause binds.
 *
 * Closed, for the reason quoin#425 settled: the retained `parseClauseBinding`
 * validates membership against the pinned schema and refuses anything else,
 * so refusing an unlisted value IS the retained behaviour.
 */
export type ClauseForce = "mandatory" | "recommended" | "permitted";

/**
 * The digest of the clause set a report was evaluated against.
 *
 * A newtype rather than a `String` because the stored spelling is the only
 * thing quoin can check about this value: it cannot recompute the digest, it
 * has no copy of the clause set, and it copies the value verbatim into the
 * `clause-discharge-v1` document it emits. The retained
 * `parseClauseBinding` (`src/quire/validate.ts`) enforced the format through
 * the vendored `clause-binding-v1` schema's
 * `"pattern": "^sha256:[0-9a-f]{64}$"`; with the schema gone (quoin#502) the
 * refusal lives on the type that deserialises the report, which is the same
 * place, expressed once.
 *
 * Trace: FR-046-AC-1
 */
export type ClauseSetDigest = string;

/**
 * Exact identity of a module-supplied clause set (quire-rs FR-067).
 */
export interface ClauseSetKey {
  /**
   * The authority that publishes the clause set.
   */
  authority: string;
  /**
   * The clause set's id within that authority.
   */
  id: string;
  /**
   * The clause set's version.
   */
  version: string;
}

/**
 * A full commit sha, as the caller reported it.
 *
 * Not validated as hexadecimal: the retained store accepts whatever the
 * caller's CI reports, and refusing here would refuse records that already
 * exist on disk.
 */
export type Commit = string;

/**
 * A resolved git commit id, as forty lowercase hex characters.
 */
export type CommitSha = string;

/**
 * How a module treats a downstream consumer's additions.
 */
export type CompatibilityPosture = "strict" | "additive" | "declared-lossy";

/**
 * One gap in declared-vocabulary coverage.
 */
export interface CompletenessFinding {
  /**
   * Document carrying the exclusion, for the two exclusion kinds.
   */
  document?: string | null;
  /**
   * What kind of gap it is.
   */
  kind: CompletenessFindingKind;
  /**
   * Human-readable detail. Not contractual.
   */
  message: string;
  /**
   * How much it is worth.
   */
  severity: Severity;
  /**
   * The vocabulary value, e.g. `safety`.
   */
  value: VocabularyValue;
  /**
   * Declaration this concerns, e.g. `quality-characteristics`.
   */
  vocabulary: VocabularyName;
}

/**
 * What kind of gap a finding records.
 *
 * The schema name is spelled out rather than taken from the Rust name.
 * `quoin-validators` owns a `FindingKind` too, and `schemars` resolves a
 * collision by appending a digit to whichever type it generated SECOND — so
 * the bare name would belong to whichever crate `boundary_schema()` happened
 * to register first, and reordering those calls would silently re-point an
 * exported TypeScript type at the other crate's union. `FindingKind2` says
 * nothing about which domain it describes; this says it. `tc_1617` pins the
 * name to this union's members so a future collision cannot take it back.
 */
export type CompletenessFindingKind =
  "unowned" | "unjustified-exclusion" | "undeclared-exclusion";

/**
 * The `semantic.contract_version` a manifest declares.
 */
export type ContractVersion = string;

/**
 * One coverage diagnostic, restricted to the two fields the advisor joins on.
 *
 * # Two fields of seven
 *
 * quire emits `declaration`, `reason`, `message`, `path`, `line`, `value` and
 * a subject field. The advisor reads `reason` — to select
 * `uncatalogued-verification-method` — and `value`, which quire-rs CR-091
 * guarantees is byte-equal to the `method` it is about. It
 * renders none of the rest, so none of the rest is declared.
 *
 * `value` is the one `Option` that carries meaning by
 * being absent: an engine predating CR-091 emits the reason with no value,
 * and the advisor must degrade to two-state behaviour rather than misread
 * silence as "every authored method is catalogued".
 */
export interface CoverageDiagnostic {
  /**
   * The open machine vocabulary quire classifies the diagnostic under
   * (quire-rs FR-055 leaves it open, so this is a `String`).
   */
  reason: string;
  /**
   * The catalog or vocabulary value the diagnostic is about, verbatim.
   */
  value?: string | null;
}

/**
 * The payload `quire.coverage` writes to stdout.
 *
 * Two fields of the engine's report, and that is the whole of what the
 * retained TypeScript read: six commands parsed `quire coverage --json` and
 * between them touched `obligations` and `diagnostics` and nothing else. The
 * rest of `CoverageReport` — totals, rows, symbols, the status census — is
 * rendered by `quire` itself and was never quoin's to carry.
 */
export interface CoveragePayload {
  /**
   * The run's diagnostics, in the engine's order.
   */
  diagnostics: CoverageDiagnostic[];
  /**
   * The obligations the run derived, in the engine's order.
   */
  obligations: Obligation[];
}

/**
 * The request accepted by `quire.coverage`.
 */
export interface CoverageRequest {
  /**
   * Module roots supplying the traceability model, in the order given.
   *
   * Empty is not "no modules": it is ambient discovery, the resolution
   * `quire coverage` performs with no `--module`. A closed set replaces
   * discovery rather than adding to it (quire-rs#405).
   */
  modules?: string[];
  /**
   * The repository root to derive obligations from.
   */
  scope: string;
}

/**
 * One authored criterion, as decided or not decided.
 */
export interface CriterionView {
  /**
   * The authored criterion text.
   */
  criterion: string;
  /**
   * The decision consulted, when one was found — including when it was
   * found and refused. A reader who sees only `status` cannot tell an
   * expired decision from an absent one, and the difference is the work.
   */
  decision?: SufficiencyDecision | null;
  /**
   * Why it is open. Omitted when it is not.
   */
  reason?: string | null;
  /**
   * Supported only with a current decision that says satisfied.
   */
  status: ViewStatus;
}

/**
 * A decision's own state, which is not the same as the criterion's status.
 *
 * `Satisfied` is a claim by the decider; the criterion is supported only if
 * the decision is also current. Keeping the two spellings apart is what stops
 * `state` reading as a verdict.
 */
export type DecisionState = "satisfied" | "open";

/**
 * One entry of the stderr array.
 *
 * A `Serialize` struct rather than a hand-built `serde_json::Value`: the
 * field list is then reviewable, and the compiler checks that every branch
 * populated it.
 */
export interface Diagnostic {
  /**
   * The stable code, from the catalogued enum — never a literal invented
   * at the call site.
   */
  code: string;
  /**
   * Ordered context. `BTreeMap` for byte-stable serialisation.
   */
  context: Record<string, string>;
  /**
   * A sentence for an operator.
   */
  message: string;
}

/**
 * Evidence that a clause's expected output exists.
 *
 * `Serialize` only: it is reachable from the wire exclusively as the payload
 * of a `DischargeFact`, whose one door is `parse_fact`. A derived
 * `Deserialize` here would be that second, permissive door one level down.
 */
export interface DirectDischargeFact {
  /**
   * Who attested, and for how long.
   */
  attestation: DischargeAttestation;
  /**
   * The clause this discharges.
   */
  clauseId: string;
  /**
   * Non-empty, and its entries unique.
   */
  evidenceRefs: string[];
}

/**
 * Who attested to a discharge, under what authority, and for how long.
 *
 * Deserialised through `parse_attestation` and never by a derived reader —
 * see the module header. The wire form is the permissive
 * `Value`; the predicates decide whether it becomes one of
 * these.
 */
export interface DischargeAttestation {
  /**
   * When it was made. An instant.
   */
  attestedAt: string;
  /**
   * The actor making the attestation.
   */
  attestedBy: string;
  /**
   * The authority they hold to make it.
   */
  authority: string;
  /**
   * `sha256:<64 lowercase hex>`.
   */
  evidenceDigest: string;
  /**
   * When it stops being current. An instant, strictly after
   * `attested_at`.
   */
  expiresAt: string;
  /**
   * The revision the attestation is about.
   */
  sourceRevision: string;
}

/**
 * The binding population, partitioned three ways.
 */
export interface DischargeBinding {
  /**
   * Discharged by evidence.
   */
  direct: ClauseDischarge[];
  /**
   * Discharged by an authorised decision.
   */
  dispositions: ClauseDischarge[];
  /**
   * Binding and not discharged.
   */
  open: ClauseDischarge[];
}

/**
 * Evidence.
 */
export interface DischargeFactDirect {
  kind: "direct";
}

/**
 * A decision.
 */
export interface DischargeFactDisposition {
  kind: "disposition";
}

/**
 * One discharge fact.
 *
 * # Why an internally tagged enum
 *
 * The retained type is a discriminated union on `kind` whose members carry
 * their fields **flat** beside the discriminant:
 * `{"kind":"direct","clauseId":…,"evidenceRefs":[…],"attestation":{…}}`.
 * `#[serde(tag = "kind")]` is the one serde representation that reproduces
 * those bytes — externally tagged would nest the payload under a `"direct"`
 * key and adjacently tagged would add a second one, and either would change
 * the `clause-discharge-v1` document that `ClauseDischarge::fact` embeds.
 * A struct with a `kind` field would have reproduced the JSON too, at the
 * cost of making `evidenceRefs` and `approvalRef` simultaneously optional in
 * the type — which is the invariant the union exists to state.
 *
 * # Why the READER is hand-written and not derived
 *
 * The tagged representation above describes what this type EMITS. What it
 * ACCEPTS is `parse_fact`, reached through the hand-written
 * `Deserialize` below, so the predicates a
 * `build_discharge` request goes through are the same ones a
 * `render_discharge` request goes through. See the module header for the
 * forged report that made the difference observable.
 */
export type DischargeFact = DischargeFactDirect | DischargeFactDisposition;

/**
 * A complete, non-scored discharge partition.
 *
 * Every input population appears. There is no aggregate anywhere in this
 * type, and FR-046-AC-2 asserts the absence directly — a score over a
 * denominator that excludes unresolved applicability would read as a
 * measurement of something nobody measured.
 */
export interface DischargeReport {
  /**
   * The ORIGINAL request string, not a re-rendering of the parsed instant.
   * `buildDischargeReport` compares `instant(request.asOf)` and emits
   * `request.asOf`, so `2026-08-15T00:00:00.000Z` is echoed with its
   * fraction and `+00:00` is echoed as `+00:00`.
   */
  asOf: string;
  /**
   * The binding population.
   */
  binding: DischargeBinding;
  /**
   * Copied from the binding report.
   */
  clauseSet: ClauseSetKey;
  /**
   * Copied from the binding report.
   */
  clauseSetDigest: string;
  /**
   * A copy of the binding report's context, ordered so the serialised
   * document is byte-stable.
   */
  context: Record<string, string>;
  /**
   * Clauses that do not apply.
   */
  notBinding: ClauseDischarge[];
  /**
   * Always `clause-discharge-v1`.
   */
  schemaVersion: DischargeSchemaVersion;
  /**
   * Undecided applicability, kept out of the binding denominator.
   */
  unresolved: ClauseDischarge[];
  /**
   * Facts supplied and not spent. Clause-ordered `unresolved`/`not_binding`
   * entries first, then fact-ordered `unknown_clause` entries — the two
   * loops of the retained implementation, in its order.
   */
  unusedFacts: UnusedDischargeFact[];
}

/**
 * The one accepted `schemaVersion` of a discharge report.
 */
export type DischargeSchemaVersion = "clause-discharge-v1";

/**
 * Where one clause landed in the partition.
 */
export type DischargeState =
  "direct" | "disposition" | "open" | "unresolved" | "not_binding";

/**
 * What an approved disposition decided.
 */
export type DispositionDecision =
  "accepted_risk" | "temporary_exception" | "delegated";

/**
 * An authorised decision about a clause.
 *
 * FR-046's constraint, restated where the type is: a disposition is evidence
 * of an authorised decision, **not** evidence that the clause's expected
 * output exists. That is why it never joins the `direct` population.
 *
 * `Serialize` only, for the reason `DirectDischargeFact` states.
 */
export interface DispositionFact {
  /**
   * A reference to the approval itself.
   */
  approvalRef: string;
  /**
   * Who attested, and for how long.
   */
  attestation: DischargeAttestation;
  /**
   * The clause this disposes of.
   */
  clauseId: string;
  /**
   * What was decided.
   */
  decision: DispositionDecision;
  /**
   * Why.
   */
  rationale: string;
}

/**
 * One document as the CALLER read it: a bundle-relative path and raw bytes.
 *
 * The boundary's reason for existing (quoin#445). `quoin-core`'s library half
 * is audited as a `ReusableLibrary` by
 * `quoin-core/tests/tc_library_containment.rs`, so an operation that takes a
 * path and reads the disk fails the gate. The walk and the reads stay in the
 * command shell — which is where a CLI's I/O belongs — and the decision
 * crosses the boundary as bytes.
 */
export interface DocumentSource {
  /**
   * Path, relative to the bundle root, with `/` separators.
   */
  path: string;
  /**
   * The whole file, as text.
   */
  raw: string;
}

/**
 * A type name supplied by multiple modules.
 */
export interface Duplicate {
  /**
   * The namespace in which the collision occurred.
   */
  kind: EntryKind;
  /**
   * Module names that supplied it, sorted as TypeScript did.
   */
  modules: string[];
  /**
   * The duplicate name, preserving its first declaration spelling.
   */
  name: string;
}

/**
 * A method id more than one module declared, in first-wins order.
 */
export interface DuplicateMethod {
  /**
   * The contested id.
   */
  id: string;
  /**
   * The modules that declared it, the winner first.
   */
  modules: string[];
}

/**
 * One located gate-that-gates-nothing defect.
 *
 * `subject`, `changeTarget`, `remedy` and `summary` are advisory prose for the
 * operator. Verdict parity is defined on `(kind, obligation, path, line,
 * wiredBy)`; the prose is reproduced faithfully because it is part of the
 * emitted payload, not because its wording is contractual.
 */
export interface EmptyGateFinding {
  /**
   * `path:line`, the exact locus an operator must edit.
   */
  changeTarget: string;
  /**
   * Always `GateThatGatesNothing` today.
   */
  kind: FindingKind;
  /**
   * The 1-based line of the unasserted count.
   */
  line: LineNumber;
  /**
   * The obligation the gate comment claims to enforce.
   */
  obligation: ObligationId;
  /**
   * The shell script holding the unasserted count.
   */
  path: RepoPath;
  /**
   * What to change.
   */
  remedy: string;
  /**
   * Human label for the gate.
   */
  subject: string;
  /**
   * The full three-way join, stated once.
   */
  summary: string;
  /**
   * The build or CI file that wires the script, proving it is a gate.
   */
  wiredBy: RepoPath;
}

/**
 * Which executable and engine produced a payload.
 *
 * # Why a struct and not a `serde_json::Value`
 *
 * quire declares `EngineProvenance` with three concrete fields — `cli`,
 * `engine`, `capabilities` — so a struct is the honest model: it says what
 * the format is, and the crate's rule is that a reader spells quire's field
 * names exactly. A `Value` would have been the honest choice only if the
 * shape were open or undeclared, and it is neither.
 *
 * The field is `Option` on `ClauseBindingReport` rather than required
 * because the retained `src/quire/types.ts` declared it `engine?:` (deleted
 * in quoin#502). That is the one documented exception to the crate header's
 * "fields quoin reads are required": quoin does not read it at all —
 * `buildDischargeReport` never looks at it — so there is no blank row for an
 * absence to produce.
 */
export interface EngineProvenance {
  /**
   * What that engine declares it can do.
   */
  capabilities: string[];
  /**
   * The executable that ran.
   */
  cli: string;
  /**
   * The engine behind it.
   */
  engine: string;
}

/**
 * The payload `modules.ensure_defaults` writes to stdout.
 */
export interface EnsureDefaultsPayload {
  /**
   * Entries installed for the first time.
   */
  installed: string[];
  /**
   * Entries skipped because `defaultEnabled` is `false`.
   */
  skipped: string[];
  /**
   * Entries already present and correctly pinned; no network was used.
   */
  unchanged: string[];
  /**
   * Entries re-resolved to a different commit.
   */
  updated: string[];
}

/**
 * The request accepted by `modules.ensure_defaults`.
 */
export interface EnsureDefaultsRequest {
  /**
   * The `~/.ix` home to reconcile.
   */
  home?: string | null;
  /**
   * The text of `default-modules.yaml`.
   *
   * Carried in the request rather than located here: the file ships inside
   * the npm package, so the caller already knows where its own package root
   * is and the boundary does not need to guess at one.
   */
  manifest: string;
  /**
   * `lazy` (the default) installs only what is missing or re-pinned; `sync`
   * re-resolves every entry.
   */
  mode?: Mode;
}

/**
 * The two catalog namespaces.
 */
export type EntryKind = "artifact" | "object";

/**
 * Separation facts carried by one evidence relationship.
 *
 * Every field is optional and the absence of one stays visible: a policy that
 * asks for a dimension nothing records reports the suites that are missing it,
 * rather than inferring a value (FR-094).
 */
export interface EvidenceLineage {
  /**
   * Who produced the evidence.
   */
  actor?: string | null;
  /**
   * Where the inputs came from.
   */
  dataSource?: string | null;
  /**
   * The toolchain the implementation under test was built with.
   */
  implementationToolchain?: string | null;
  /**
   * The review path the result travelled.
   */
  reviewPath?: string | null;
  /**
   * The verification technique used.
   */
  technique?: string | null;
}

/**
 * A obligation id, the join the matrix keys on.
 */
export type EvidenceObligationId = string;

/**
 * Which of the two fact shapes this is.
 *
 * Separate from `DischargeFact` because the retained
 * `UnusedDischargeFact.kind` is declared `DischargeFact["kind"]` — the
 * discriminant without the payload — and an unused fact reports only that.
 */
export type FactKind = "direct" | "disposition";

/**
 * One scanner result, transcribed.
 *
 * `severity` is the scanner's own word, never normalized (FR-034-CON-2).
 */
export interface Finding {
  /**
   * The line the finding is about.
   */
  line?: number | null;
  /**
   * The scanner's message.
   */
  message?: string | null;
  /**
   * The path the finding is about.
   */
  path?: string | null;
  /**
   * The scanner's rule identity.
   */
  ruleId: string;
  /**
   * The scanner's own severity word.
   */
  severity?: string | null;
  /**
   * Criterion ids the scanner named.
   */
  traceIds?: string[] | null;
}

/**
 * The class of defect a finding reports.
 *
 * One variant today. It is an enum and not a `&'static str` because the kind is
 * the payload's discriminant: a second validator adds a variant here and every
 * `match` on it becomes a compiler-checked edit site.
 */
export type FindingKind = "gate-that-gates-nothing";

/**
 * One finding-shaped scan of ONE suite at ONE commit.
 */
export interface FindingRecord {
  /**
   * Full commit sha the scan was performed at.
   */
  commit: Commit;
  /**
   * The declared `test_type` this scan produced, when the caller names one.
   */
  evidenceKind?: string | null;
  /**
   * One entry per finding the scanner reported.
   */
  findings: Finding[];
  /**
   * Number of rules the scan actually evaluated, when the tool reports it.
   */
  rulesEvaluated?: number | null;
  /**
   * What the scan covered, as the tool reported it.
   *
   * Load-bearing for vacuity: a scan that ran with no rules enabled also
   * reports zero findings and cannot be told apart by the findings alone.
   */
  ruleset?: string | null;
  /**
   * Always `STORE_SCHEMA_VERSION`(super::STORE_SCHEMA_VERSION).
   */
  schemaVersion: number;
  /**
   * The suite this scan covered.
   */
  suite: SuiteId;
  /**
   * ISO-8601, supplied by the caller — never read from the clock here.
   */
  timestamp: string;
  /**
   * Tool and version, as the adapter reported them.
   */
  tool: string;
}

/**
 * One classified artifact.
 */
export interface FormFinding {
  /**
   * The advisory diagnostic, for a legacy form.
   */
  diagnostic?: LegacyFormDiagnostic | null;
  /**
   * The form found.
   */
  form: PropertiesForm;
  /**
   * 1-based line of the first Properties block, when one exists.
   */
  line?: number | null;
  /**
   * Repo-relative artifact path, prefixed with its repository.
   */
  path: string;
}

/**
 * Every document under a bundle root that carries parseable frontmatter.
 */
export interface FrontmatterRead {
  /**
   * The documents.
   */
  documents: BundleDocument[];
  /**
   * Documents whose frontmatter could not be parsed.
   */
  unreadable: UnreadableDocument[];
}

/**
 * The payload `evidence.gc` writes to stdout.
 */
export interface GcPayload {
  /**
   * The collected records, as **absolute** paths, sorted.
   *
   * Every path this domain reports is absolute, and none of the ones
   * `quoin_evidence` returns are: a library that names no host capability
   * cannot know the root. The join is `absolute`, against the root
   * `store_root` reports.
   */
  deleted: string[];
}

/**
 * The request accepted by `evidence.gc`.
 */
export interface GcRequest {
  /**
   * Report what would be collected and remove nothing.
   */
  dry_run?: boolean;
  /**
   * The repository root.
   */
  repo: string;
}

/**
 * Explainable result for one requested obligation, successful or not.
 */
export interface IndependenceAssessment {
  /**
   * One entry per requested dimension, in the requested order.
   */
  dimensions: IndependenceDimensionAssessment[];
  /**
   * The obligation it applies to.
   */
  obligation: EvidenceObligationId;
  /**
   * The profile that asked.
   */
  profile: ProfileId;
  /**
   * The requirement's id.
   */
  requirement: string;
  /**
   * The two suites that satisfied it, when one pair did.
   */
  satisfiedBy?: [SuiteId, SuiteId] | null;
  /**
   * The verdict.
   */
  status: IndependenceStatus;
  /**
   * A sentence naming what was found or what was missing.
   */
  summary: string;
}

/**
 * One separation axis a profile can ask two evidence lines to differ on.
 */
export type IndependenceDimension =
  | "actor"
  | "implementation-toolchain"
  | "technique"
  | "data-source"
  | "review-path";

/**
 * What one dimension looked like across the bound suites.
 */
export interface IndependenceDimensionAssessment {
  /**
   * The dimension.
   */
  dimension: IndependenceDimension;
  /**
   * The suites whose lineage records nothing for it, sorted.
   */
  missingSuites: SuiteId[];
  /**
   * The distinct recorded values, sorted and deduplicated.
   */
  values: string[];
}

/**
 * Normalized projection of profile-selected independence requirements.
 */
export interface IndependencePolicy {
  /**
   * `AP-<digits>`.
   */
  profile: ProfileId;
  /**
   * The requirements, as the profile declared them.
   */
  requirements: IndependenceRequirement[];
  /**
   * Always `1`.
   */
  schemaVersion: number;
}

/**
 * One exact obligation for which a profile requests two separated lines.
 */
export interface IndependenceRequirement {
  /**
   * The dimensions two lines must both differ on.
   */
  dimensions: IndependenceDimension[];
  /**
   * The requirement's own id, unique within the policy.
   */
  id: string;
  /**
   * The obligation it applies to, named at most once in the policy.
   */
  obligation: EvidenceObligationId;
  /**
   * Why the profile asks for it.
   */
  rationale: string;
}

/**
 * Whether two separated lines were found.
 */
export type IndependenceStatus = "satisfied" | "insufficient";

/**
 * The payload `evidence.inspect_mocks` writes to stdout.
 */
export interface InspectMocksPayload {
  /**
   * One entry per substituting symbol, sorted.
   */
  injections: MockInjection[];
  /**
   * Where the record was written as an **absolute** path, or `null` on a
   * dry run.
   */
  path?: string | null;
}

/**
 * The request accepted by `evidence.inspect_mocks`.
 */
export interface InspectMocksRequest {
  /**
   * Full commit sha whose source is inspected.
   */
  commit: string;
  /**
   * Inspect and report, writing no observation record.
   */
  dry_run?: boolean;
  /**
   * The repository root.
   */
  repo: string;
  /**
   * The suite whose test source is inspected.
   */
  suite: string;
  /**
   * ISO-8601 inspection time.
   */
  timestamp: string;
  /**
   * The inspecting tool and its version.
   */
  tool: string;
}

/**
 * The payload `modules.install` writes to stdout.
 */
export interface InstallPayload {
  /**
   * The registry record that was written.
   */
  module: InstalledModule;
  /**
   * Whether a previous version of the same module was replaced.
   */
  replaced_previous: boolean;
}

/**
 * The request accepted by `modules.install`.
 */
export interface InstallRequest {
  /**
   * The `~/.ix` home to install into.
   */
  home?: string | null;
  /**
   * The CLI source argument, in the `path:` / `github:` / `package:`
   * spellings `src/plugins.ts`'s `parseSourceArg` accepted.
   */
  source: string;
}

/**
 * One installed module's registry record.
 */
export interface InstalledModule {
  /**
   * RFC 3339 timestamp of the install.
   */
  installedAt: string;
  /**
   * The module's declared name; also its directory name.
   */
  name: ModuleName;
  /**
   * The git ref (tag/branch) that was requested, if any.
   */
  ref?: string | null;
  /**
   * The cache path the content was materialized from.
   */
  resolvedPath: string;
  /**
   * The semantic contract pin, when the module declares a semantic block.
   */
  semantic?: SemanticPin | null;
  /**
   * The resolved commit id — the durable pin used for drift detection.
   */
  sha?: CommitSha | null;
  /**
   * Where it came from.
   */
  source: Source;
  /**
   * The materialized path under the modules directory.
   */
  targetPath: string;
}

/**
 * The payload `change_assurance.intake` writes to stdout.
 */
export interface IntakePayload {
  /**
   * The directory the pair became visible in.
   */
  directory: string;
  /**
   * Whether this call retained the pair, or found the same pair already
   * there.
   */
  retained: boolean;
  /**
   * How many bytes of output were retained.
   */
  size_bytes: number;
}

/**
 * The request accepted by `change_assurance.intake`.
 */
export interface IntakeRequest {
  /**
   * The sealed attestation's exact bytes, as hex.
   */
  attestation_hex: string;
  /**
   * The output's exact bytes, as hex.
   */
  output_hex: string;
  /**
   * Repository root holding the evidence store.
   */
  repo: string;
}

/**
 * The advisory diagnostic a legacy form earns (FR-074).
 */
export interface LegacyFormDiagnostic {
  /**
   * Always `semantic.legacy-properties-form`.
   */
  code: string;
  /**
   * Which legacy form was found.
   */
  form: PropertiesForm;
  /**
   * 1-based line of the block.
   */
  line: number;
  /**
   * Always `typed-table`.
   */
  migration: string;
  /**
   * Always `warning`.
   */
  severity: string;
}

/**
 * Severity of legacy Properties forms (FR-074).
 */
export type LegacyForms = "warning" | "error";

/**
 * A 1-based line number inside a source file.
 *
 * `NonZeroU64` rather than `usize`: line 0 does not exist, and the payload
 * crosses a JSON boundary where a 0 would be read as "unknown".
 */
export type LineNumber = number;

/**
 * The payload `modules.list` writes to stdout.
 */
export interface ListPayload {
  /**
   * Every installed module, in registry order.
   *
   * Registry order and not sorted here: `src/commands/module/list.ts`
   * printed what the registry held, in the order it held it, and a sort
   * introduced at the boundary would be a user-visible change smuggled in
   * under a port.
   */
  modules: InstalledModule[];
}

/**
 * The `home` a request may name, with its bound already checked.
 */
export interface ListRequest {
  /**
   * The `~/.ix` home to read, or absent for the one the host resolves.
   */
  home?: string | null;
}

/**
 * Request accepted by `catalog.load`.
 */
export interface LoadRequest {
  /**
   * Explicit module candidates, or absent for the host's default discovery.
   */
  roots?: string[] | null;
}

/**
 * A named representation mapping (FR-071..073).
 */
export type MappingName = string;

/**
 * Why a method was recommended — the rule and the value that matched.
 */
export interface MatchReason {
  /**
   * The applicability axis.
   */
  rule: string;
  /**
   * The value on that axis the obligation carries.
   */
  value: string;
}

/**
 * The merged catalog plus what the merge could not use.
 *
 * Merge is **first-wins by method id**, matching quire-rs FR-054 exactly. If
 * the two disagreed, the advisor would recommend from one catalog while the
 * auditor checked conformance against another.
 */
export interface MethodCatalog {
  /**
   * Contested ids, sorted by id.
   */
  duplicates: DuplicateMethod[];
  /**
   * Every method, sorted by id.
   */
  methods: VerificationMethod[];
  /**
   * Unreadable module roots, sorted by root.
   */
  unreadable: UnreadableModule[];
}

/**
 * The payload `semantic.migration_example` writes to stdout.
 */
export interface MigrationExamplePayload {
  /**
   * The migration guidance a legacy-form diagnostic cites (FR-074).
   */
  example: string;
}

/**
 * One test symbol observed substituting a stand-in for real behaviour.
 */
export interface MockInjection {
  /**
   * Identifiers substituted for real behaviour, sorted and deduplicated.
   */
  injects: string[];
  /**
   * The line the call was seen on.
   */
  line?: number | null;
  /**
   * Repo-relative source location, when the inspection can name one.
   */
  path?: string | null;
  /**
   * The suite whose source was inspected.
   */
  suite: SuiteId;
  /**
   * The test symbol containing the substitution.
   */
  symbol: SymbolId;
}

/**
 * How hard a reconcile should look, in the wire spelling.
 */
export type Mode = "lazy" | "sync";

/**
 * The declared name of a spec module.
 *
 * A module name is used directly as a directory name under
 * `~/.ix/filament/modules`, so it is validated against path separators and
 * relative-path components at construction. Every place that joins a name onto
 * a directory takes a `ModuleName`, not a `String`, which is what makes that
 * check unskippable.
 */
export type ModuleName = string;

/**
 * What one module root's `semantic` block came to.
 *
 * The `data_schema` resolutions `quoin_semantic::SemanticModule` also carries
 * are deliberately NOT here. Nothing on the TypeScript side reads them — they
 * exist so the diagnostics below can be produced — and a payload field with no
 * reader is a wire shape nobody maintains and every future change has to keep
 * working.
 */
export interface ModuleSemanticView {
  /**
   * The parsed block, absent when the manifest declares none or its block
   * was refused.
   */
  block?: SemanticBlock | null;
  /**
   * Every diagnostic reading the block produced, in the order produced.
   */
  diagnostics: SemanticDiagnostic[];
  /**
   * The module root this entry answers for, echoed back.
   *
   * Echoed rather than left to positional correlation: the caller pairs the
   * answer with its own module record, and a list that says which root each
   * entry is for cannot be mis-paired by a change to either side.
   */
  root: string;
}

/**
 * One module's text, as the CALLER read it.
 *
 * The boundary's reason for existing (quoin#445): `quoin-core`'s library half
 * may not touch a filesystem, so the command shell locates the module, reads
 * `manifest.yaml`, asks `schema_refs_of` which schemas that manifest needs,
 * reads those, and sends all of it as bytes.
 */
export interface ModuleSource {
  /**
   * What to call this module when its manifest declares no `name`. The
   * filesystem shell passes the module root; the wire caller passes whatever
   * it resolved, and the boundary never turns it back into a path.
   */
  label: string;
  /**
   * `manifest.yaml`, as text.
   */
  manifest: string;
  /**
   * Every `frontmatter_schema_ref` this manifest names, keyed by the ref
   * exactly as the manifest spells it.
   */
  schemas: Record<string, SchemaSource>;
}

/**
 * A module's `version`.
 */
export type ModuleVersion = string;

/**
 * An object type's name, as `object_types[].name` declares it.
 */
export type ObjectTypeName = string;

/**
 * One obligation, as quoin reads it off `quire coverage --json`.
 *
 * # Two fields of nine
 *
 * quire emits `source`, `id`, `document`, `statement`, `statement_hash`,
 * `method`, `parameters`, `criticality` and `target_ids`. The assurance view
 * reads `id` (to derive the owning requirement, and as the solution node's
 * id) and `statement` (as the node's statement). It reads none of the other
 * seven — not even `document`, which a reader might reasonably expect a view
 * to cite, and does not.
 *
 * The three required fields here are required for the reason in the module
 * header: they are read unconditionally, so their absence must be an error
 * rather than a silent empty. `statement_hash` joined the set when the
 * evidence store arrived (agent-ix/quoin#456): a binding stamps it, and a
 * default would make every binding agree with every statement.
 */
export interface Obligation {
  /**
   * The obligation's declared criticality, verbatim (`P0`, `high`, …).
   *
   * Carried, never interpreted. CR-008 deleted a hardcoded `["P0"]`
   * precisely so the engine does not decide which values count as high.
   */
  criticality?: string | null;
  /**
   * The obligation id, e.g. `FR-001-AC-1` or `NFR-010-M-2`.
   */
  id: string;
  /**
   * The authored `Verification` cell, verbatim.
   *
   * # Why the three below are `Option` when the header says fields quoin reads are required
   *
   * The rule in the module header is about **drift**: a field quoin reads
   * unconditionally must fail loudly if quire renames it, rather than read
   * as an empty string. These three are not read unconditionally — quire
   * emits them as `method?: string | null`, `parameters?` and
   * `criticality?: string | null`, and every reader here branches on the
   * absence first. `unknownMethodFinding` returns `null` without a method;
   * `multiplicityFinding` and `mutationFinding` return `null` without a
   * criticality. An absence is an answer, so it must be representable.
   *
   * `null` and the missing key both read as `None`, which is what the
   * retained `!obligation.method` guard does with either.
   */
  method?: string | null;
  /**
   * The obligation's structured parameters, as quire emits them —
   * `{"target": "< 4 min", "threshold": "< 5 min"}` on an NFR row.
   *
   * A `BTreeMap` and not a `Value`: the advisor asks whether the keys
   * `target` or `threshold` are present, which an opaque value would put a
   * cast in front of at the one read site.
   */
  parameters?: Record<string, string> | null;
  /**
   * The criterion's statement, in the spec's own words.
   */
  statement: string;
  /**
   * quire's hash of the statement, as it stands at the read.
   *
   * quoin never computes this and only ever compares it: suspect detection
   * is `stamped != current`, so a hash quoin derived itself would compare
   * equal to itself and detect nothing.
   */
  statement_hash: string;
  /**
   * Test-case ids the criterion's method cell names, when it names any.
   *
   * The indirection an agent-eval report and every other Test-Matrix-keyed
   * tool arrives on: the tool reports `TC-EV-057`, the row says that test
   * case verifies this criterion, and quire-rs FR-053-AC-11 carries the join
   * here rather than making quoin re-parse the table (agent-ix/quoin#144).
   */
  target_ids?: string[] | null;
}

/**
 * A requirement obligation as it was written in the gate comment, e.g.
 * `FR-001-AC-1`.
 *
 * The claim regex is case-insensitive, so this deliberately preserves the
 * author's spelling rather than normalising it: the finding must point at what
 * the file actually says.
 */
export type ObligationId = string;

/**
 * What a producer said about one symbol.
 *
 * A closed enum here and not a `String`, unlike
 * `kind`: the retained TypeScript's union is
 * enforced at every construction site inside `src/evidence/`, every adapter
 * picks from these four, and the store's readers branch on all four.
 */
export type Outcome = "pass" | "fail" | "skip" | "error";

/**
 * A semantic package's IR identity, `<org>/<repo>` — never a URL and never
 * an `ix://` identity (FR-070).
 */
export type PackageIdentity = string;

/**
 * The payload `evidence.parse_lineage` writes to stdout.
 */
export interface ParseLineagePayload {
  /**
   * The validated lineage.
   */
  lineage: EvidenceLineage;
}

/**
 * The request accepted by `evidence.parse_lineage`.
 */
export interface ParseLineageRequest {
  /**
   * The lineage document's text, as the caller read it.
   */
  text: string;
}

/**
 * The payload `evidence.parse_policy` writes to stdout.
 */
export interface ParsePolicyPayload {
  /**
   * The validated policy.
   */
  policy: IndependencePolicy;
}

/**
 * The request accepted by `evidence.parse_policy`.
 */
export interface ParsePolicyRequest {
  /**
   * Every obligation id the corpus derives today.
   *
   * Sent with the document rather than checked in a second call: a policy
   * naming an obligation nothing derives reports every requirement as
   * vacuously assessed, and the two questions have one answer.
   */
  known_obligations?: string[];
  /**
   * The policy document's text, as the caller read it.
   */
  text: string;
}

/**
 * A run-shaped adapter's transcript.
 */
export interface ParseResultsPayloadRun {
  /**
   * The transcribed results.
   */
  entries: RunEntry[];
  /**
   * The evidence kind, when the FORMAT ITSELF determines it.
   */
  evidence_kind?: string | null;
  kind: "run";
  /**
   * Results the producer reported that no run outcome represents.
   */
  unrepresented?: UnrepresentedView[] | null;
}

/**
 * A finding-shaped adapter's transcript.
 */
export interface ParseResultsPayloadFinding {
  /**
   * The transcribed findings. Empty is meaningful.
   */
  findings: Finding[];
  kind: "finding";
  /**
   * How many rules the scanner reported evaluating.
   */
  rules_evaluated?: number | null;
  /**
   * The ruleset, where the document names it.
   */
  ruleset?: string | null;
  /**
   * The scanner, where the document names it.
   */
  tool?: string | null;
}

/**
 * The payload `evidence.parse_results` writes to stdout.
 *
 * Tagged, because the two adapter registries produce different record types
 * and the caller must be able to tell which it received without inferring it
 * from which fields are present.
 */
export type ParseResultsPayload =
  ParseResultsPayloadRun | ParseResultsPayloadFinding;

/**
 * The request accepted by `evidence.parse_results`.
 */
export interface ParseResultsRequest {
  /**
   * An explicit adapter name. An unknown one is an error, never a fall back.
   */
  adapter?: string | null;
  /**
   * The producer document's text, as the caller read it.
   */
  text: string;
  /**
   * The suite's declared tool, which selects an adapter when none is named.
   */
  tool?: string | null;
}

/**
 * A named actor and the authority they hold.
 */
export interface Participant {
  /**
   * What they may decide.
   */
  authority: string;
  /**
   * This participant's id, referenced by sufficiency decisions.
   */
  id: string;
  /**
   * What they are independent of.
   */
  independence: string;
  /**
   * Their role.
   */
  role: string;
}

/**
 * The payload `core.ping` writes to stdout.
 */
export interface PingPayload {
  /**
   * The `quoin-core` crate version.
   */
  core_version: string;
  /**
   * Whatever `echo` held, unchanged.
   */
  echo?: string | null;
  /**
   * The protocol revision this build speaks.
   */
  protocol_version: number;
}

/**
 * The request accepted by `core.ping`.
 *
 * `deny_unknown_fields` so a caller that misspells a field is refused rather
 * than silently ignored — a field the boundary drops is a field the caller
 * believes it sent.
 */
export interface PingRequest {
  /**
   * An opaque token returned unchanged, for correlating a call with its
   * answer across the pipe. Absent is fine.
   */
  echo?: string | null;
  /**
   * The protocol revision the caller believes it is speaking. When it
   * disagrees with this build's, the answer is still complete — it is how
   * the caller learns which revision it is actually talking to — so the
   * disagreement is reported as `Partial`, not as a
   * failure.
   */
  expect_protocol?: number | null;
}

/**
 * An assurance profile id, `AP-<digits>`.
 */
export type ProfileId = string;

/**
 * The four shapes a `## Properties` section can take, plus its absence.
 */
export type PropertiesForm =
  "typed-table" | "free-column-table" | "bullet-list" | "sysml-fence" | "none";

/**
 * The payload `quire.properties` writes to stdout.
 *
 * The **shape map**, not the classification report. `quoin advise` was the one
 * caller, and it walked every document and every criterion to build
 * `Map<row_id, {property, archetype}>` — so the map is the answer, and
 * `PropertiesReport`, `Document`, `Criterion`, `AcShape`, `Extraction` and
 * `Span` never needed to cross at all. `shapes` is keyed by obligation id and
 * feeds `auditor.advise`'s `shapes` field unchanged.
 */
export interface PropertiesPayload {
  /**
   * Obligation id → what the classifier made of that criterion.
   */
  shapes: Record<string, PropertyShape>;
  /**
   * Documents that resolved to no archetype, scope-relative.
   *
   * Reported rather than raised: `quire properties` exits 1 when ANY
   * document fails to resolve while still classifying every one that did,
   * and the retained `propertyShapes` swallowed that exit deliberately so
   * two untyped assets could not cost the whole shape axis. Here the
   * partial result and the list of what was skipped are one answer, which
   * is what an exit status could not say.
   */
  unresolved: string[];
}

/**
 * The request accepted by `quire.properties`.
 */
export interface PropertiesRequest {
  /**
   * Documents to classify, scope-relative.
   *
   * Named by the caller rather than discovered here, because a glob is a
   * filesystem walk and this half performs none. `quoin_quire::properties::
   * documents_under_spec` is the host's way to answer the `spec/**\/*.md`
   * the retained `quoin advise` passed; an empty list asks for exactly that.
   */
  documents?: string[];
  /**
   * Module roots supplying the archetypes and the `property_idioms`
   * registry. Empty means ambient discovery, as in `CoverageRequest`.
   */
  modules?: string[];
  /**
   * The repository root documents resolve and report relative to.
   */
  scope: string;
}

/**
 * What quire's `properties` view classified one criterion as.
 *
 * Read from a second engine call rather than from the coverage payload, which
 * carries neither field. The caller supplies the map because that call spawns
 * a process and this module does no I/O.
 */
export interface PropertyShape {
  /**
   * The owning document's archetype, e.g. `FR`.
   */
  archetype: string;
  /**
   * The FR-052 property shape, e.g. `round-trip`.
   */
  property: string;
}

/**
 * The payload `evidence.read_baseline` writes to stdout.
 */
export interface ReadBaselinePayload {
  /**
   * The baseline, or `null` when none has been accepted.
   *
   * `null` is the fact `--ratchet` turns on: a missing baseline degrades the
   * run to a full report, and labelling that full report "new violations
   * only" told a day-one reader their whole backlog was new (#169).
   */
  baseline?: BaselineFile | null;
  /**
   * Where it would be, as an **absolute** path, for the notice that names it.
   */
  path: string;
}

/**
 * The payload `semantic.read_blocks` writes to stdout.
 */
export interface ReadBlocksPayload {
  /**
   * One entry per requested root, in the order they were requested.
   */
  modules: ModuleSemanticView[];
}

/**
 * The request accepted by `semantic.read_blocks`.
 */
export interface ReadBlocksRequest {
  /**
   * The module roots to read, each a directory holding a `manifest.yaml`.
   *
   * A list rather than one root per call: `loadCatalog` reads every
   * installed module on every `quoin write`, and one subprocess per module
   * would make the cost of the boundary proportional to the module set.
   */
  roots: string[];
}

/**
 * The request accepted by `completeness.read_frontmatter`.
 */
export interface ReadFrontmatterRequest {
  /**
   * Every markdown document the caller read, in the order it walked them.
   */
  documents: DocumentSource[];
  /**
   * Documents the caller found and could not open, and why. Optional: a walk
   * that hit no OS error should not have to send an empty list.
   */
  unreadable?: UnreadableDocument[];
}

/**
 * One reasoning step and its criteria.
 */
export interface ReasoningView {
  /**
   * In authored order.
   */
  criteria: CriterionView[];
  /**
   * The step's id.
   */
  id: string;
  /**
   * The reasoning.
   */
  statement: string;
  /**
   * Supported only when EVERY criterion is.
   */
  status: ViewStatus;
  /**
   * What it argues toward.
   */
  supports: string;
}

/**
 * The payload `change_assurance.receipt` writes to stdout.
 */
export interface ReceiptPayload {
  /**
   * The sealed verification receipt, in full.
   *
   * The whole receipt rather than a verdict: the caller prints the proof
   * rows and the reasons, and a payload that carried only `outcome` would
   * make "why" a second call.
   */
  receipt: unknown;
}

/**
 * The request accepted by `change_assurance.receipt`.
 */
export interface ReceiptRequest {
  /**
   * The retained FR-032 audit reports as a JSON array, as hex of the exact
   * input bytes. Absent means no audit was retained, which stays distinct
   * from an audit with no findings.
   */
  audits_hex?: string | null;
  /**
   * The candidate revision the selected attestations must be bound to.
   */
  candidate_revision: string;
  /**
   * The retained ix-flow decision history, as hex of the exact input bytes.
   */
  decisions_hex: string;
  /**
   * Digests of stored parent records, named rather than walked.
   */
  parent_digests: string[];
  /**
   * Digest of the stored record to verify.
   */
  record_digest: string;
  /**
   * Repository root holding the evidence store.
   */
  repo: string;
  /**
   * Which attestation is offered for which obligation. Only these are read.
   */
  selections: SelectionRequest[];
}

/**
 * One recommendation for one obligation.
 */
export interface Recommendation {
  /**
   * Its IADT class.
   */
  class: string;
  /**
   * The evidence kind it produces, when the module declared one.
   */
  evidenceKind?: string | null;
  /**
   * The recommended method's id.
   */
  method: string;
  /**
   * Every rule that matched, sorted by rule then value.
   */
  reasons: MatchReason[];
}

/**
 * A run record was written.
 */
export interface RecordPayloadRun {
  /**
   * Obligations bound for the first time, sorted.
   */
  bound: EvidenceObligationId[];
  kind: "run";
  /**
   * Where it was written, as an **absolute** path.
   */
  run_path: string;
  /**
   * Obligations whose statement changed since they were bound, sorted.
   */
  suspect: EvidenceObligationId[];
  /**
   * Trace ids matching no derived obligation, sorted.
   */
  unmatched: string[];
  /**
   * Results no run outcome represents.
   */
  unrepresented?: UnrepresentedView[] | null;
}

/**
 * A finding-shaped scan record was written.
 */
export interface RecordPayloadScan {
  /**
   * Obligations newly bound by the scan, sorted.
   */
  bound: string[];
  /**
   * The transcribed findings.
   */
  findings: Finding[];
  kind: "scan";
  /**
   * How many rules the scanner reported evaluating.
   */
  rules_evaluated?: number | null;
  /**
   * Where it was written, as an **absolute** path.
   */
  scan_path: string;
  /**
   * Named `--discharges` ids no obligation states, sorted.
   */
  unknown: string[];
  /**
   * Whether the scan evaluated no rules, and so bound nothing.
   */
  vacuous: boolean;
}

/**
 * The payload `evidence.record` writes to stdout.
 */
export type RecordPayload = RecordPayloadRun | RecordPayloadScan;

/**
 * The request accepted by `evidence.record`.
 *
 * One request for both record types on purpose. The choice between a run
 * record and a finding-shaped scan record is made by the ADAPTER REGISTRY
 * before anything is parsed, and splitting it into two operations would move
 * that choice to the caller — which is exactly how a scan ends up in `runs/`
 * with the clean-versus-unrun distinction lost at the point of intake
 * (FR-034).
 */
export interface RecordRequest {
  /**
   * An explicit adapter name.
   */
  adapter?: string | null;
  /**
   * The commit it ran at.
   */
  commit: string;
  /**
   * Obligation ids a finding-shaped scan was run to check.
   *
   * Ignored on the run path, where the obligations discharged are derived
   * from the trace ids the entries carry.
   */
  discharges?: string[];
  /**
   * The declared evidence kind, when the caller names one.
   */
  kind?: string | null;
  /**
   * Separation facts for every binding this record creates.
   */
  lineage?: EvidenceLineage | null;
  /**
   * The obligations as quire derives them today.
   */
  obligations?: Obligation[];
  /**
   * The repository root.
   */
  repo: string;
  /**
   * The producer document's text, as the caller read it.
   */
  results: string;
  /**
   * The suite that ran.
   */
  suite: string;
  /**
   * ISO-8601. Supplied, never read from the clock here.
   */
  timestamp: string;
  /**
   * Tool and version, as it identifies itself.
   */
  tool: string;
}

/**
 * The payload `change_assurance.recover` writes to stdout.
 */
export interface RecoverPayload {
  /**
   * How many interrupted-intake staging directories were removed.
   */
  removed: number;
}

/**
 * The request accepted by `change_assurance.recover`.
 */
export interface RecoverRequest {
  /**
   * Repository root holding the evidence store.
   */
  repo: string;
}

/**
 * An edge to a document outside this argument.
 */
export interface Relationship {
  /**
   * An `ix://` reference.
   */
  target: string;
  /**
   * How it reads.
   */
  type: RelationshipType;
}

/**
 * How a relationship reads.
 */
export type RelationshipType = "supports" | "challenges" | "references";

/**
 * The payload `modules.remove` writes to stdout.
 */
export interface RemovePayload {
  /**
   * The module that was removed.
   */
  removed: string;
}

/**
 * The request accepted by `modules.remove`.
 */
export interface RemoveRequest {
  /**
   * The `~/.ix` home to remove from.
   */
  home?: string | null;
  /**
   * The installed module's name.
   */
  name: string;
}

/**
 * The payload `assurance.render_authored_argument` writes to stdout.
 *
 * A JSON string field rather than raw markdown, for the same reason
 * `RenderCasePayload` is one: stdout carries a canonical JSON payload and
 * nothing else.
 */
export interface RenderAuthoredArgumentPayload {
  /**
   * The rendered markdown.
   */
  rendered: string;
}

/**
 * The payload `assurance.render_discharge` writes to stdout.
 */
export interface RenderDischargePayload {
  /**
   * The rendered markdown.
   */
  rendered: string;
}

/**
 * A path relative to the repository root, always written with `/` separators.
 *
 * Findings are compared, sorted, and matched against build wiring by this
 * string, so the separator normalisation is part of the identity and not a
 * display concern — a Windows-shaped `scripts\gate.sh` and a POSIX
 * `scripts/gate.sh` are the same gate.
 */
export type RepoPath = string;

/**
 * A request naming only the repository whose store is read.
 */
export interface RepoRequest {
  /**
   * The repository root. The store is `<repo>/spec/evidence`.
   */
  repo: string;
}

/**
 * A corpus root as it appears in the report.
 */
export interface ReportCorpusRoot {
  /**
   * `<org>/<repo>`.
   */
  repository: string;
  /**
   * The revision swept.
   */
  revision: string;
}

/**
 * The payload `config.resolve_org` writes to stdout.
 *
 * Field-for-field `ResolvedOrg` in the deleted `src/org.ts`, so the caller in
 * `src/core/org.ts` is a rename and not a reshaping: `org` is omitted rather
 * than null when unresolved, exactly as the TypeScript `org?: string` was.
 */
export interface ResolveOrgPayload {
  /**
   * Whether a config layer fell back rather than contributing its content.
   *
   * New at the boundary, and not a widening of the contract: `src/org.ts`
   * discarded this fact silently, which is why a malformed config file
   * resolved to "no stored org" with nothing to show for it. It rides the
   * payload; the run is still a success, because a broken config must not
   * stop an author writing specs (FR-027-AC-5).
   */
  degraded: boolean;
  /**
   * The organization, omitted when nothing yielded one.
   */
  org?: string | null;
  /**
   * Which source won: `flag`, `env`, `config`, `git` or `none`.
   */
  source: string;
}

/**
 * The request accepted by `config.resolve_org`.
 *
 * Every field is state the caller already holds. Nothing here is a path this
 * operation would open.
 */
export interface ResolveOrgRequest {
  /**
   * The environment the resolution reads, supplied rather than read.
   *
   * `QUOIN_ORG` is a *declared binding* (`QUOIN_ENV_BINDINGS`), so it is
   * layered over the config document by the schema machinery rather than
   * consulted directly — one precedence rule in one place, which is the
   * property `src/org.ts` went out of its way to keep and this preserves.
   */
  env?: Record<string, string>;
  /**
   * The explicit `--org` value, when one was passed.
   */
  flag?: string | null;
  /**
   * The repository's `.git/config`, absent when there is none to read.
   */
  git_config?: string | null;
  /**
   * The project-level `.ix` config document, absent when no project layer
   * applies or the file does not exist.
   */
  project_config?: string | null;
  /**
   * The user-level config document, absent when the file does not exist.
   */
  user_config?: string | null;
}

/**
 * One producer result, transcribed.
 */
export interface RunEntry {
  /**
   * The configuration dimension values this entry was executed under.
   *
   * `Record<string, string>` in the retained source, and a `BTreeMap` here
   * rather than a `Value`: the auditor reads the dimension names to say
   * which t-way combinations a run reached, and an opaque `Value` would put
   * that cast at every read site.
   */
  config?: Record<string, string> | null;
  /**
   * What `score` measures. See `MUTATION_SCORE_METRIC`.
   */
  metric?: string | null;
  /**
   * What the producer said.
   */
  outcome: Outcome;
  /**
   * A native numeric result, where the format has one.
   */
  score?: number | null;
  /**
   * The producer's own identity for the result.
   */
  symbol: SymbolId;
  /**
   * Obligation or criterion ids the producer named for this result.
   */
  traceIds?: string[] | null;
}

/**
 * The payload `validators.run` writes to stdout.
 *
 * One field, named `findings`, because that is the byte shape
 * `quoin validate --json` has always emitted and the cutover is not licence to
 * change a user-visible document.
 */
export interface RunPayload {
  /**
   * Every finding, ordered by `(path, line, obligation)`.
   */
  findings: EmptyGateFinding[];
}

/**
 * One run of ONE suite at ONE commit.
 *
 * The suite is the atomic unit of evidence: aggregation is a view, and a
 * partial run must never be able to masquerade as a full one. Re-runs at the
 * same commit are last-write-wins, latest only — the file name carries the
 * short commit and nothing distinguishing one attempt from the next.
 */
export interface RunRecord {
  /**
   * Full commit sha the run was performed at.
   */
  commit: Commit;
  /**
   * One entry per symbol the producer reported.
   */
  entries: RunEntry[];
  /**
   * The declared `test_type` this run produced, when the caller names one.
   *
   * Its absence is not an invitation to guess: method conformance once
   * inferred "this was a test run" from a non-empty entry list, which is
   * true of a transcribed inspection too (agent-ix/quoin#105).
   */
  evidenceKind?: string | null;
  /**
   * Always `STORE_SCHEMA_VERSION`(super::STORE_SCHEMA_VERSION).
   */
  schemaVersion: number;
  /**
   * The suite this run covered.
   */
  suite: SuiteId;
  /**
   * ISO-8601, supplied by the caller — never read from the clock here.
   */
  timestamp: string;
  /**
   * Tool and version, as the adapter reported them.
   */
  tool: string;
}

/**
 * The request accepted by `validators.run`.
 *
 * A snapshot of the repository, not a path to it. `deny_unknown_fields` so a
 * caller that misspells a field is refused rather than silently ignored — a
 * field the boundary drops is a field the caller believes it sent.
 */
export interface RunRequest {
  /**
   * Every file the caller found, keyed by repository-relative,
   * `/`-separated path, holding the file's lines split on `\n`.
   *
   * `null` means the caller found the path but could not read it. That is
   * not the same as omitting it: an unreadable file still classifies as a
   * shell script or as wiring, and refuses only if the analysis reaches for
   * its text — which is the on-demand read the TypeScript oracle performs.
   *
   * Lines rather than one string because that is the shape the golden corpus
   * already carries. Joining the lines with a newline reconstructs the file,
   * so a caller must split on a newline alone and leave any carriage return
   * on the line.
   */
  files: Record<string, string[] | null>;
  /**
   * Directories the caller could not list, repository-relative. The empty
   * string names the repository root itself.
   *
   * Separate from an unreadable file because the walk **aborts** on one: a
   * subtree nobody could read means the answer would be computed over a
   * repository nobody has seen, and an empty result must mean the validator
   * looked and found nothing.
   */
  unlistable?: string[];
}

/**
 * The payload `change_assurance.schema` writes to stdout.
 */
export interface SchemaAssetPayload {
  /**
   * The requested asset's exact bytes, or `null` when none was requested.
   *
   * The text as compiled in, trailing newline included — a consumer
   * validates against the same bytes the sealing code was written against,
   * and a re-serialization here would defeat that.
   */
  schema?: string | null;
  /**
   * Every asset name this build ships, in the order the vocabulary declares.
   */
  schemas: string[];
}

/**
 * The request accepted by `change_assurance.schema`.
 */
export interface SchemaAssetRequest {
  /**
   * Which asset to emit, or absent to ask only for the vocabulary.
   *
   * Absent and "emit nothing" are the same answer here because the
   * vocabulary rides on every response: a caller listing the assets and a
   * caller fetching one both learn what the build ships.
   */
  name?: string | null;
}

/**
 * The payload `completeness.schema_refs` writes to stdout.
 */
export interface SchemaRefsPayload {
  /**
   * Every `frontmatter_schema_ref` the manifest's declared coverage reaches
   * for, deduplicated, in manifest order. Paths are relative to the module
   * root; the caller reads them and sends the content back.
   */
  refs: string[];
}

/**
 * The request accepted by `completeness.schema_refs`.
 */
export interface SchemaRefsRequest {
  /**
   * One module's `manifest.yaml`, as text.
   */
  manifest: string;
}

/**
 * One module's frontmatter schema, as the CALLER read it.
 *
 * `Unreadable` carries the caller's own message rather than being modelled as
 * an absent key, because the reason is what the user reads: "frontmatter
 * schema 'schemas/nfr.json' unreadable: ENOENT …" names the file and the OS
 * error, and an absent key could only ever produce a generic sentence.
 */
export type SchemaSource = { text: string } | { unreadable: string };

/**
 * The payload `change_assurance.seal_attestation` writes to stdout.
 */
export interface SealAttestationPayload {
  /**
   * The sealed attestation. Emitted, not retained.
   */
  attestation: unknown;
}

/**
 * The request accepted by `change_assurance.seal_attestation`.
 */
export interface SealAttestationRequest {
  /**
   * The attestation body, without `digest` and without `retained_output`,
   * as hex of the exact input bytes.
   */
  attestation_hex: string;
  /**
   * The media type the caller declares for that file.
   *
   * Stated rather than sniffed, so a producer's own content type is
   * preserved exactly.
   */
  media_type: string;
  /**
   * The retained result file's exact bytes, as hex.
   */
  output_hex: string;
}

/**
 * The payload `change_assurance.seal_record` writes to stdout.
 */
export interface SealRecordPayload {
  /**
   * Where it was retained, relative to the repository root as supplied.
   */
  path: string;
  /**
   * The sealed record, `digest` included.
   */
  record: unknown;
  /**
   * Whether this call retained the record, or found the same bytes already
   * there. Re-sealing an identical record is not an error and never was.
   */
  retained: boolean;
}

/**
 * The request accepted by `change_assurance.seal_record`.
 */
export interface SealRecordRequest {
  /**
   * The record body, without its `digest`, as hex of the exact input bytes.
   */
  record_hex: string;
  /**
   * Repository root holding the evidence store.
   */
  repo: string;
}

/**
 * One `<proof-id>=<attestation-digest>` selection.
 */
export interface SelectionRequest {
  /**
   * The digest of the attestation offered.
   */
  attestation_digest: string;
  /**
   * The obligation the attestation is offered against.
   */
  proof_id: string;
}

/**
 * One module's validated `semantic` block.
 */
export interface SemanticBlock {
  /**
   * `compatibility_posture`, defaulted to `additive` when absent.
   */
  compatibility_posture: CompatibilityPosture;
  /**
   * `contract_version`.
   */
  contract_version: ContractVersion;
  /**
   * `exports`, in manifest order.
   */
  exports: ObjectTypeName[];
  /**
   * `imports`, package identity to exact version.
   */
  imports: Record<string, ModuleVersion>;
  /**
   * `legacy_forms`, defaulted to `warning` when absent.
   */
  legacy_forms: LegacyForms;
  /**
   * `mappings`, in manifest order.
   */
  mappings: MappingName[];
  /**
   * `package`.
   */
  package: PackageIdentity;
  /**
   * `semantic_core`.
   */
  semantic_core: SemanticCoreVersion;
  /**
   * `sweep_report`, when the manifest carries one as a string.
   */
  sweep_report?: string | null;
  /**
   * `targets`, in manifest order.
   */
  targets: string[];
}

/**
 * The `@agent-ix/semantic-core` version a manifest compiles against.
 */
export type SemanticCoreVersion = string;

/**
 * One install-time refusal or advisory.
 */
export interface SemanticDiagnostic {
  /**
   * The stable code.
   *
   * `string` in the schema, not a closed union of the 29 spellings:
   * `DiagnosticCode` is `#[non_exhaustive]`, so freezing today's set into
   * a TypeScript literal type would make a code added here a type error at
   * a call site that only forwards the value. The catalogue lives in
   * `all`, where it can be enumerated, and the wire
   * spelling is `as_str`.
   */
  code: string;
  /**
   * Human-readable detail. **Not contractual** — see `DIVERGENCE.md`.
   */
  message: string;
  /**
   * Manifest or file locus, e.g. `object_types[entity].data_schema.schema`.
   */
  path: string;
  /**
   * How much it is worth.
   */
  severity: SemanticSeverity;
}

/**
 * The registry pin recorded under an installed module's `semantic` key.
 *
 * Field names match `SemanticRegistryPin` in `src/semantic/package-manifest.ts`
 * because the registry file is shared with the retained TypeScript.
 */
export interface SemanticPin {
  /**
   * Digest per exported symbol.
   */
  exports: Record<string, string>;
  /**
   * The module's declared semantic package.
   */
  package: string;
  /**
   * The semantic core it binds to.
   */
  semanticCore: string;
}

/**
 * How much a diagnostic is worth.
 */
export type SemanticSeverity = "error" | "warning";

/**
 * How much a finding is worth.
 *
 * Note the asymmetry, which **is** the policy: `unowned` is medium and an
 * unjustified exclusion is high. Saying nothing about reliability is an
 * admitted gap a reader can see. Excusing it without a reason is an assertion
 * of completeness with nothing behind it, and it removes the finding that would
 * have prompted the work.
 */
export type Severity = "medium" | "high";

/**
 * A GitHub repository whose module root is the repository root.
 */
export interface SourceGithub {
  /**
   * A tag or branch to pin to.
   */
  ref?: string | null;
  /**
   * `owner/repo`, or a full clonable url.
   */
  repo: string;
  /**
   * A commit id to pin to; outranks `ref`.
   */
  sha?: string | null;
  type: "github";
}

/**
 * A module living in a subdirectory of a git repository.
 */
export interface SourceGitSubdir {
  /**
   * The subdirectory holding the module root.
   */
  path: string;
  /**
   * A tag or branch to pin to.
   */
  ref?: string | null;
  /**
   * A commit id to pin to; outranks `ref`.
   */
  sha?: string | null;
  type: "git-subdir";
  /**
   * `owner/repo`, or a full clonable url.
   */
  url: string;
}

/**
 * A git repository by url.
 */
export interface SourceGit {
  /**
   * A tag or branch to pin to.
   */
  ref?: string | null;
  /**
   * A commit id to pin to; outranks `ref`.
   */
  sha?: string | null;
  type: "git";
  /**
   * A clonable url.
   */
  url: string;
}

/**
 * A plain url. Accepted structurally, not resolvable.
 */
export interface SourceUrl {
  /**
   * A tag or branch, carried for round-tripping.
   */
  ref?: string | null;
  /**
   * A commit id, carried for round-tripping.
   */
  sha?: string | null;
  type: "url";
  /**
   * The url.
   */
  url: string;
}

/**
 * A local directory.
 */
export interface SourcePath {
  /**
   * The directory.
   */
  path: string;
  type: "path";
}

/**
 * An npm package. Accepted structurally, not resolvable here.
 */
export interface SourceNpm {
  /**
   * The package name.
   */
  package: string;
  /**
   * A registry override.
   */
  registry?: string | null;
  type: "npm";
  /**
   * An exact version.
   */
  version?: string | null;
}

/**
 * Where a module's content comes from.
 */
export type Source =
  | SourceGithub
  | SourceGitSubdir
  | SourceGit
  | SourceUrl
  | SourcePath
  | SourceNpm;

/**
 * One artifact or object type supplied by a module.
 */
export interface SpecCatalogEntry {
  /**
   * The raw object `data_schema` value, preserving inline JSON and refs.
   */
  dataSchema?: unknown;
  /**
   * Which kind of type this entry is.
   */
  kind: EntryKind;
  /**
   * The module declaring this entry.
   */
  moduleName: string;
  /**
   * The declaring module root.
   */
  moduleRoot: string;
  /**
   * The artifact/object name.
   */
  name: string;
  /**
   * The artifact schema path, derived from `schema_ref`.
   */
  schemaPath?: string | null;
  /**
   * The artifact frontmatter schema reference.
   */
  schemaRef?: string | null;
  /**
   * A real skeleton filename with disk-accurate casing, when supplied.
   */
  skeletonPath?: string | null;
}

/**
 * One module represented by the catalog.
 */
export interface SpecModule {
  /**
   * Declared artifact-type names.
   */
  artifactTypes: string[];
  /**
   * Manifest name, or the root basename when it is absent/non-string.
   */
  name: string;
  /**
   * Declared object-type names.
   */
  objectTypes: string[];
  /**
   * The resolved module root.
   */
  root: string;
  /**
   * The parsed semantic block, when one passed semantic validation.
   */
  semantic?: SemanticBlock | null;
  /**
   * Semantic diagnostics observed while loading this module.
   */
  semanticDiagnostics?: SemanticDiagnostic[] | null;
  /**
   * Manifest version when it is a string.
   */
  version?: string | null;
}

/**
 * A statement hash as quire computed it; quoin only ever compares these.
 */
export type StatementHash = string;

/**
 * The payload `evidence.store_facts` writes to stdout.
 *
 * The constants the TypeScript command layer needs *before* it can build a
 * request at all — the adapter names an `--adapter` flag offers, the metric a
 * mutation score is recorded under, the schema version, and where the two
 * checked-in store files live. They are served rather than restated so that
 * `src/core/evidence.ts` has one pinned copy instead of a second declaration.
 */
export interface StoreFactsPayload {
  /**
   * Every adapter name, run-shaped then finding-shaped, in `--help` order.
   */
  adapter_names: string[];
  /**
   * Store-relative path of the ratchet baseline.
   */
  baseline_path: string;
  /**
   * Store-relative path of the binding graph.
   */
  bindings_path: string;
  /**
   * The family directories `evidence.gc` collects.
   */
  collected_families: string[];
  /**
   * Store-relative path of the authored inspection register.
   */
  inspections_path: string;
  /**
   * The metric name a `cargo-mutants` score is recorded under.
   */
  mutation_score_metric: string;
  /**
   * The revalidation triggers every trust decision must declare.
   */
  required_triggers: TrustTrigger[];
  /**
   * The store root itself, relative to the repository root.
   *
   * Repo-relative rather than absolute because this operation is handed no
   * repository: it states the layout, and the caller joins it to whichever
   * root it is asking about. The other four paths below are relative to
   * THIS one, not to the repository.
   */
  store_root_path: string;
  /**
   * The version stamped into every record envelope.
   */
  store_schema_version: number;
  /**
   * Store-relative path of the authored suite registry.
   */
  suites_path: string;
}

/**
 * What a participant decided about one sufficiency criterion.
 */
export interface SufficiencyDecision {
  /**
   * The authority claimed, checked against the participant's own.
   */
  authority: string;
  /**
   * The criterion, matched against the authored text verbatim.
   */
  criterion: string;
  /**
   * When it was decided. An instant.
   */
  decidedAt: string;
  /**
   * The participant id. Must be one the argument declares.
   */
  decidedBy: string;
  /**
   * `sha256:<64 lowercase hex>`.
   */
  evidenceDigest: string;
  /**
   * Evidence the decider relied on. Non-empty when the state is satisfied.
   */
  evidenceRefs: string[];
  /**
   * When it stops being current. An instant, and after `decided_at`.
   */
  expiresAt: string;
  /**
   * Why, when the decider gave a reason. Omitted when absent, never null.
   */
  rationale?: string | null;
  /**
   * The reasoning step whose criterion this decides.
   */
  reasoningId: string;
  /**
   * The revision the decision was made over.
   */
  sourceRevision: string;
  /**
   * Whether the decider called it satisfied.
   */
  state: DecisionState;
}

/**
 * A suite identity, as the suite registry declares it.
 */
export type SuiteId = string;

/**
 * The payload `semantic.sweep_corpus` writes to stdout.
 */
export interface SweepCorpusPayload {
  /**
   * The report, in the shape `sweep-report.schema.json` describes.
   */
  report: SweepReport;
}

/**
 * The request accepted by `semantic.sweep_corpus`.
 */
export interface SweepCorpusRequest {
  /**
   * RFC 3339 UTC, supplied by the caller.
   *
   * The clock is the caller's, not the boundary's: a report whose timestamp
   * came from inside this process could not be asserted, and
   * `quoin_semantic::sweep_corpus` takes the same parameter for the same
   * reason.
   */
  generated_at: string;
  /**
   * The semantic package identity the report is for.
   */
  package: string;
  /**
   * The roots to walk, in the order the report should list them.
   */
  roots: SweepRootRequest[];
  /**
   * The module version the report is for.
   */
  version: string;
}

/**
 * The per-form tallies.
 */
export interface SweepCounts {
  /**
   * How many artifacts were classified.
   */
  artifacts: number;
  /**
   * One count per form.
   */
  forms: Record<string, number>;
  /**
   * The two legacy forms, repeated so the schema's `legacy` block is filled.
   */
  legacy: Record<string, number>;
}

/**
 * The document `semantic.sweep_report` points at.
 */
export interface SweepReport {
  /**
   * The roots swept.
   */
  corpus: ReportCorpusRoot[];
  /**
   * The tallies.
   */
  counts: SweepCounts;
  /**
   * One entry per artifact.
   */
  findings: FormFinding[];
  /**
   * RFC 3339 UTC, as the schema's pattern requires.
   */
  generatedAt: string;
  /**
   * `<org>/<repo>`.
   */
  package: string;
  /**
   * The module version.
   */
  version: string;
}

/**
 * One corpus root a sweep should walk.
 */
export interface SweepRootRequest {
  /**
   * The repository name recorded against every finding under it.
   */
  repository: string;
  /**
   * The revision recorded in the report's `corpus` block.
   */
  revision: string;
  /**
   * The directory to walk.
   */
  root: string;
}

/**
 * A FR-051 stable symbol identity.
 */
export type SymbolId = string;

/**
 * The top claim, with the reasons it is not supported.
 */
export interface TopClaimView {
  /**
   * The claim's id.
   */
  id: string;
  /**
   * Every reason, in the retained order: argument status, criteria,
   * assumptions, challenges, then the two discharge reasons.
   */
  reasons: string[];
  /**
   * What is claimed.
   */
  statement: string;
  /**
   * Supported only when `reasons` is empty.
   */
  status: ViewStatus;
  /**
   * What it is about.
   */
  subject: string;
}

/**
 * The explainable result of comparing one decision against what is observed.
 */
export interface TrustAssessment {
  /**
   * The decision's id.
   */
  id: TrustDecisionId;
  /**
   * Stated limits on the reliance.
   */
  limitations: string[];
  /**
   * Who is accountable.
   */
  owner: string;
  /**
   * The decisions this use is permitted to support.
   */
  permittedDecisions: string[];
  /**
   * The producer's name.
   */
  producer: string;
  /**
   * The verdict.
   */
  status: TrustStatus;
  /**
   * The triggers that differed, empty unless the status is `invalidated`.
   */
  triggeredBy: TrustTrigger[];
  /**
   * The bounded use's id.
   */
  useId: string;
}

/**
 * The payload `evidence.trust_assessments` writes to stdout.
 */
export interface TrustAssessmentsPayload {
  /**
   * One assessment per readable decision, in file-name order.
   */
  assessments: TrustAssessment[];
  /**
   * Paths of decisions that would not parse or would not validate.
   *
   * Named, not counted, and not fatal: one malformed decision must not hide
   * every other reliance judgement in the store.
   */
  unreadable: string[];
}

/**
 * A trust decision id, `ETD-<digits>`.
 *
 * Validated at construction because it becomes a file name: the retained
 * `trustDecisionPath` refuses anything else rather than letting an id escape
 * the `trust/` directory.
 */
export type TrustDecisionId = string;

/**
 * The payload `evidence.trust_decision` writes to stdout.
 */
export interface TrustDecisionPayload {
  /**
   * Its effective state, as compared against the observed context.
   */
  assessment: TrustAssessment;
  /**
   * Where the decision was written, as an **absolute** path.
   */
  path: string;
}

/**
 * The request accepted by `evidence.trust_decision`.
 */
export interface TrustDecisionRequest {
  /**
   * The decision document, as the caller read it.
   */
  decision: unknown;
  /**
   * The repository root.
   */
  repo: string;
}

/**
 * What a decision says about the context currently observed.
 */
export type TrustStatus =
  | "accepted"
  | "accepted-with-limitations"
  | "invalidated"
  | "not-accepted"
  | "unobserved";

/**
 * A fact about a producer context whose change can invalidate a decision.
 */
export type TrustTrigger =
  | "producer-version"
  | "configuration"
  | "adapter"
  | "validation-corpus"
  | "input-contract"
  | "environment";

/**
 * A check that could not be run, separate from both findings and clean results.
 *
 * The distinction is the whole point: an absent mock inspection means "nobody
 * looked", never "nothing was mocked", and folding it into either bucket
 * would hand a caller a clean bill it never earned (agent-ix/quoin#204).
 */
export interface UnevaluatedCheck {
  /**
   * Which check could not run.
   *
   * A `String` for the same reason as `kind`: the retained
   * declaration is a one-member union, erased before it reaches the wire,
   * and a second member added upstream must read here rather than fail.
   * `MOCKED_CONFIRMATION` is the one it holds today.
   */
  check: string;
  /**
   * The obligation the check was about.
   */
  obligation: string;
  /**
   * Why, in one sentence, including what to run.
   */
  reason: string;
  /**
   * The suites that could not be answered for, sorted.
   */
  suites: string[];
}

/**
 * A document whose frontmatter could not be read, and why.
 */
export interface UnreadableDocument {
  /**
   * Path, relative to the bundle root, with `/` separators.
   */
  path: string;
  /**
   * Why it could not be read.
   */
  reason: string;
}

/**
 * A module root whose `manifest.yaml` could not be read or parsed.
 *
 * Reported rather than thrown: a catalog missing one module's entries is
 * still worth having, and the command that would have crashed is the one an
 * operator runs *to diagnose* the module (agent-ix/quoin#106).
 */
export interface UnreadableModule {
  /**
   * The resolved module root.
   */
  moduleRoot: string;
  /**
   * The reader's own message. See `DIVERGENCE.md` §4.
   */
  reason: string;
}

/**
 * One producer result the run-entry vocabulary cannot carry.
 *
 * Declared here rather than re-used from `quoin_evidence::adapters` because
 * that type is not `Serialize`: it is an internal shape, and a `derive` added
 * to it there would put a wire contract on a type nothing wires.
 */
export interface UnrepresentedView {
  /**
   * Why no run-entry outcome carries it.
   */
  reason: string;
  /**
   * The producer's own state name, verbatim.
   */
  state: string;
  /**
   * The producer's own identity for the result.
   */
  symbol: string;
}

/**
 * A declaration whose vocabulary could not be resolved, and why.
 */
export interface UnresolvedDeclaration {
  /**
   * The declaration's name.
   */
  name: VocabularyName;
  /**
   * Why it could not be resolved.
   */
  reason: string;
}

/**
 * The message shown when no source yielded an organization.
 *
 * Served over the boundary so the sentence has ONE home. It was a `const` in
 * `src/org.ts` and a `const` in `quoin_config::org`, and two copies of a
 * user-facing sentence is exactly the drift FR-101 retires.
 */
export interface UnresolvedOrgMessagePayload {
  /**
   * The sentence.
   */
  message: string;
}

/**
 * A decision that matched no authored criterion.
 *
 * Reported rather than dropped: a decision nobody asked for usually means the
 * criterion text was edited after the decision was recorded, and a view that
 * silently discarded it would read as a clean argument over a population the
 * decider did not think they were deciding.
 */
export interface UnusedDecision {
  /**
   * The criterion it named.
   */
  criterion: string;
  /**
   * The reasoning id the decision named.
   */
  reasoningId: string;
}

/**
 * A fact that was supplied and not spent.
 */
export interface UnusedDischargeFact {
  /**
   * The clause the fact named.
   */
  clauseId: string;
  /**
   * The fact's discriminant, without its payload.
   */
  kind: FactKind;
  /**
   * Why it was not spent.
   */
  reason: UnusedFactReason;
}

/**
 * Why a supplied fact was not spent.
 */
export type UnusedFactReason = "unknown_clause" | "not_binding" | "unresolved";

/**
 * `UNCHECKED` is not a fourth flavour of pass.
 *
 * A bundle whose module set declares no vocabulary has not been assessed, and
 * `PASS` over it is the green-matrix-over-dead-links result this program was
 * created to stop — the first draft of the TypeScript printed exactly that, and
 * the criterion written to forbid it caught it.
 */
export type Verdict = "PASS" | "CONDITIONAL" | "FAIL" | "UNCHECKED";

/**
 * One catalog entry, as the module declared it.
 */
export interface VerificationMethod {
  /**
   * Rule name → values.
   *
   * **Never interpreted structurally** — the advisor matches values, and
   * which axes exist is the declaring module's business (quire-rs
   * FR-054-CON-2). A rule naming an axis the advisor cannot observe is
   * skipped, not failed.
   */
  applicability?: Record<string, string[]>;
  /**
   * IADT in practice; a free string to the engine, so a free string here.
   */
  class: string;
  /**
   * What discharging the method means.
   */
  definition: string;
  /**
   * The evidence kind the method produces, when the module declares one.
   *
   * Its absence makes method conformance **unanswerable** rather than
   * failed: a run of an undeclared kind is not a mismatch
   * (agent-ix/quoin#105).
   */
  evidenceKind?: string | null;
  /**
   * The method id, unique in the merged catalog (first module wins).
   */
  id: string;
  /**
   * The module that contributed this entry.
   */
  moduleName: string;
  /**
   * The human name.
   */
  name: string;
  /**
   * Tools the module names for the method.
   */
  tooling?: string[];
}

/**
 * The payload `change_assurance.verify_receipt` writes to stdout.
 */
export interface VerifyReceiptPayload {
  /**
   * The receipt as re-read and re-verified, in full.
   */
  receipt: unknown;
}

/**
 * The request accepted by `change_assurance.verify_receipt`.
 */
export interface VerifyReceiptRequest {
  /**
   * The sealed receipt's exact bytes, as hex.
   */
  receipt_hex: string;
}

/**
 * The one value `schemaVersion` takes.
 */
export type ViewSchemaVersion = "authored-assurance-view-v1";

/**
 * The two states everything in this view reduces to.
 */
export type ViewStatus = "supported" | "open";

/**
 * A declaration's own name, e.g. `quality-characteristics`.
 */
export type VocabularyName = string;

/**
 * What the advisor's fact set can ever produce.
 *
 * # Why an engine fact is an operation at all
 *
 * Same shape as `evidence.store_facts`: not a function over a request, but
 * the boundary stating a constant OF ITSELF. `quoin catalog methods` renders
 * a catalog, and the only way to know whether an entry is reachable is to ask
 * the engine what values it can mint — the catalog declares the values that
 * trigger a method, and this is the set that can ever match them.
 *
 * Nothing compared the two once, and the failure was silent in both
 * directions: `match_rules` skips an unknown *axis* by design, an unknown
 * *value* on a known axis simply never matches, and `inconclusive` is already
 * a legitimate outcome. Measured then: 60 values declared, 20 producible, 7
 * methods no statement could ever reach (agent-ix/quoin#128).
 */
export interface VocabularyPayload {
  /**
   * Every characteristic value the fact set can mint, sorted.
   */
  mintableCharacteristics: string[];
}

/**
 * The per-vocabulary tally.
 */
export interface VocabularyRollup {
  /**
   * Values in the declared enum.
   */
  declared: number;
  /**
   * Values a document excuses, justified or not.
   */
  excused: number;
  /**
   * Values some document claims.
   */
  owned: number;
  /**
   * Values neither claimed nor excused.
   */
  unowned: number;
  /**
   * Which declaration.
   */
  vocabulary: VocabularyName;
}

/**
 * One value inside a declared vocabulary, e.g. `safety`.
 */
export type VocabularyValue = string;

/**
 * The payload `evidence.write_baseline` writes to stdout.
 */
export interface WriteBaselinePayload {
  /**
   * Where it was written, as an **absolute** path.
   */
  path: string;
}

/**
 * The request accepted by `evidence.write_baseline`.
 */
export interface WriteBaselineRequest {
  /**
   * The accepted findings as `<kind>:<obligation>`.
   */
  accepted: string[];
  /**
   * The commit the baseline is accepted at.
   */
  commit: string;
  /**
   * The repository root.
   */
  repo: string;
}

/** The IPC protocol revision this build of the boundary speaks. */
export const PROTOCOL_VERSION = 1;
