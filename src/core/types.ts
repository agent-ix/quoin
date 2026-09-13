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
  generatorVersion: "0.1.0",
  sourceSchemaSha256:
    "23b299fc1170709d85286283ad9a04b084c81dab639d57dd9644c8964ef93df9",
} as const;

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
  clauseSetDigest: string;
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
 * A resolved git commit id, as forty lowercase hex characters.
 */
export type CommitSha = string;

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
  kind: FindingKind;
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
  kind: FindingKind2;
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
 * `src/quire/types.ts` declares `EngineProvenance` with three concrete
 * fields — `cli`, `engine`, `capabilities` — so a struct is the honest
 * model: it says what the format is, and the crate's rule is that a reader
 * spells quire's field names exactly. A `Value` would have been the honest
 * choice only if the shape were open or undeclared, and it is neither.
 *
 * The field is `Option` on `ClauseBindingReport` rather than required
 * because the retained type declares it `engine?:`. That is the one
 * documented exception to the crate header's "fields quoin reads are
 * required": quoin does not read it at all — `buildDischargeReport` never
 * looks at it — so there is no blank row for an absence to produce.
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
 * Which of the two fact shapes this is.
 *
 * Separate from `DischargeFact` because the retained
 * `UnusedDischargeFact.kind` is declared `DischargeFact["kind"]` — the
 * discriminant without the payload — and an unused fact reports only that.
 */
export type FactKind = "direct" | "disposition";

/**
 * What kind of gap a finding records.
 */
export type FindingKind =
  "unowned" | "unjustified-exclusion" | "undeclared-exclusion";

/**
 * The class of defect a finding reports.
 *
 * One variant today. It is an enum and not a `&'static str` because the kind is
 * the payload's discriminant: a second validator adds a variant here and every
 * `match` on it becomes a compiler-checked edit site.
 */
export type FindingKind2 = "gate-that-gates-nothing";

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
 * A requirement obligation as it was written in the gate comment, e.g.
 * `FR-001-AC-1`.
 *
 * The claim regex is case-insensitive, so this deliberately preserves the
 * author's spelling rather than normalising it: the finding must point at what
 * the file actually says.
 */
export type ObligationId = string;

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

/** The IPC protocol revision this build of the boundary speaks. */
export const PROTOCOL_VERSION = 1;
