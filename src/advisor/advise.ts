/**
 * Catalog-driven verification-method recommendation (FR-031).
 *
 * The proto-advisor was a **skill-local prose table** — `test | analysis |
 * inspection | demonstration` plus a handful of evidence kinds, declared in no
 * manifest, read by no code. The result was that `Verification` columns
 * defaulted to `Test` by habit, and nothing ever advised DAST for an attack
 * surface, monitors for a temporal property, or fault injection for a
 * reliability NFR.
 *
 * This module is the deterministic half. Rules match or they do not; where they
 * are inconclusive it says so and stops, rather than guessing. An LLM may then
 * judge the residue — labelled as judgement, never as a verdict (the FR-042 /
 * ADR-0010 discipline: no verdict-by-LLM).
 */

import type { MethodCatalog, VerificationMethod } from "./methods.js";
import { parseSpace } from "../auditor/combinatorial.js";

/** What the advisor knows about one obligation before it recommends anything. */
export interface ObligationFacts {
  id: string;
  statement: string;
  /** The authored `Verification` cell, when the row has one. */
  authoredMethod?: string | null;
  /** The FR-052 property shape quire classified the criterion as. */
  propertyShape?: string | null;
  /** The owning document's archetype (`FR`, `NFR`, `StR`). */
  archetype?: string | null;
  /** Object types the spec declares near this requirement, if any. */
  objectTypes?: string[];
  /**
   * The obligation's declared criticality, verbatim (`P0`, `high`, …).
   *
   * Carried rather than interpreted. `mutation-testing` is keyed on
   * `high-criticality`, and the engine does not get to decide which values
   * count as high — CR-008 deleted a hardcoded `["P0"]` for exactly that
   * reason. So the characteristic is minted when the value **says so itself**,
   * which is reading it, not judging it.
   */
  criticality?: string | null;
}

/** Why a method was recommended — the rule and the value that matched. */
export interface MatchReason {
  rule: string;
  value: string;
}

/** One recommendation for one obligation. */
export interface Recommendation {
  method: string;
  class: string;
  evidenceKind?: string;
  reasons: MatchReason[];
}

/** The advisor's verdict for one obligation. */
export interface Advice {
  obligation: string;
  /** Deterministic recommendations, strongest (most rules matched) first. */
  recommended: Recommendation[];
  /** The authored method, normalized. */
  authored?: string | null;
  /**
   * Set when the authored method is not among the recommendations AND the
   * advisor had rules to go on. Advisory: the human confirms, and the confirmed
   * method is what the auditor later checks conformance against.
   */
  mismatch: boolean;
  /**
   * True when no rule matched anything. The honest outcome — an advisor that
   * recommends `Test` because it found nothing is the habit this replaces.
   */
  inconclusive: boolean;
}

/**
 * Characteristics the advisor can read off a statement deterministically.
 *
 * Deliberately small and lexical. Every entry is a phrase whose presence is a
 * fact about the text, not an inference about intent — the CR-014 lesson, where
 * an open set whose membership had to be *judged* reached ~13% precision.
 */
const STATEMENT_CHARACTERISTICS: Array<[string, RegExp]> = [
  ["temporal", /\b(always|never|eventually|while|until|continuously)\b/i],
  ["liveness", /\b(eventually|makes progress|terminates)\b/i],
  ["invariance", /\b(invariant|holds for every|at all times)\b/i],
  ["concurrent", /\b(concurrent|parallel|race|thread|simultaneous)\b/i],
  ["reliability", /\b(tolerat|degrad|retry|failover|resilien|recover)/i],
  // No trailing \b: the alternatives already end at their own boundary, and a
  // trailing one made `within 5ms` fail — there is no word boundary between the
  // digit and the unit, which is the form a threshold is actually written in.
  ["latency", /\b(latency|response time|within\s+\d|\d\s*ms\b|p9\d)/i],
  ["throughput", /\b(throughput|per second|requests\/s|rps)\b/i],
  ["quantified-threshold", /[<>≤≥]\s*\d|\b\d+\s*(ms|s|%|MB|GB)\b/i],
  [
    "security",
    /\b(authenticat|authoriz|permission|credential|secret|token|attack|exploit)/i,
  ],
  [
    "untrusted-input",
    /\b(untrusted|user-supplied|external input|malformed)\b/i,
  ],
  ["input-validation", /\b(reject|validate|malformed|invalid input)\b/i],
  ["parser", /\b(pars|deserializ|decode|lex)/i],
  [
    "configuration-matrix",
    /\b(configuration|feature flag|combination of|matrix of)\b/i,
  ],
  [
    "third-party-dependency",
    /\b(dependenc|third-party|vendored|licence|license)\b/i,
  ],
  ["layering", /\b(depend on|layering|must not import)\b/i],
  // Distinct from `layering`, which is directional — who may depend on whom.
  // This is about the surface itself: what is exported, what is internal, what
  // may cross. The catalog keys `architecture-conformance` on **both**
  // (spec-artifacts-process `verification_catalog`), and until this existed the
  // second half of that rule could never match, because no code path minted the
  // value. `module boundary` moved here from `layering` rather than appearing in
  // both: one phrase, one characteristic, or the two names stop meaning
  // different things.
  [
    "module-boundary",
    /\b(module boundary|public (api|interface|surface)|encapsulat|internal(s)? (of|to)|exported? (from|by))\b/i,
  ],
  ["user-visible", /\b(user|operator|the UI|displays|screen)\b/i],
  ["stable-output", /\b(byte-identical|identical output|serializ|snapshot)\b/i],
  ["agent-behaviour", /\b(agent|transcript|prompt)\b/i],
  ["no-executable-oracle", /\b(review|judgement|readable|documented)\b/i],

  // ── agent-ix/quoin#128 ───────────────────────────────────────────────────
  //
  // The catalog declared 60 characteristics and this table produced 20, so 40
  // were asked for and never minted — and an unmatched VALUE on a known axis
  // fails silently, unlike an unknown axis, which `matchRules` skips by design.
  // Seven methods were unreachable by any statement ever written, including
  // `integration-testing` and `mutation-testing`.
  //
  // Only values whose fact genuinely lives IN THE SENTENCE are added here. A
  // characteristic that is a property of the code (`path-sensitive`), of the
  // evidence store (`suite-quality-unknown`) or of an obligation field
  // (`high-criticality`) gets a fact source or gets retired — writing a regex
  // for a phrase no author ever types is the CR-014 failure, where an open set
  // whose membership had to be judged reached ~13% precision.
  //
  // Overlap with the entries above is expected and harmless: a statement may
  // carry several characteristics, and two methods keyed on different ones are
  // both worth surfacing.
  ["complexity", /\b(complexity|cyclomatic|nesting depth|deeply nested)\b/i],
  ["consistency", /\b(consistent|consistency|contradict|mutually exclusive)/i],
  [
    "cross-component",
    /\b(cross-component|between (two |the )?(components|services|processes)|across (the|a|its) [a-z-]* ?boundary|two or more components|end-to-end)\b/i,
  ],
  ["degradation", /\b(degrad|graceful(ly)?\s+(fail|fall)|fall back)/i],
  ["deserializer", /\b(deserializ|unmarshal|decoder|from_json|from_yaml)/i],
  [
    "distributed",
    /\b(distributed|cluster|replica|consensus|quorum|multi-node|across nodes)\b/i,
  ],
  [
    "fault-tolerance",
    /\b(fault[- ]toleran|tolerates? (a |an )?fail|survives? [a-z ]*fail|crash recovery)/i,
  ],
  [
    "feature-flags",
    /\b(feature flag|feature toggle|cargo feature|--features?\b|opt-in flag)\b/i,
  ],
  [
    "injection-risk",
    /\b(injection|\bXSS\b|\bSQL\b|shell escape|escap(e|ing) [a-z]* ?input)\b/i,
  ],
  [
    "io-boundary",
    /\b(filesystem|file system|socket|database|to disk|from disk|over the network|subprocess|stdin|stdout)\b/i,
  ],
  ["licence", /\b(licence|license|SPDX|copyleft|allow-?list of licen)/i],
  [
    "maintainability",
    /\b(maintainab|technical debt|code smell|dead code|duplicat(e|ion) of)/i,
  ],
  [
    "memory-safety",
    /\b(memory safety|use[- ]after[- ]free|buffer overflow|double free|dangling pointer|\bunsafe\b)/i,
  ],
  [
    "network-exposed",
    /\b(endpoint|publicly (reachable|exposed|accessible)|listens? on|inbound request|remote client|HTTP request)\b/i,
  ],
  [
    "non-deterministic-subject",
    /\b(non-?deterministic|stochastic|temperature|sampled output|model output|\bLLM\b)\b/i,
  ],
  [
    "precondition-bearing",
    /\b(precondition|postcondition|requires that|provided that|assumes that|only when)\b/i,
  ],
  [
    "protocol",
    /\b(protocol|handshake|wire format|message (format|sequence|order))\b/i,
  ],
  [
    "published-interface",
    /\b(published interface|public (api|surface)|exported (api|surface)|semver|breaking change|backward[- ]compatib)/i,
  ],
  [
    "requirement-set",
    /\b(set of requirements|across (all )?requirements|no two requirements|every requirement)\b/i,
  ],
  // `safety` as a bare word is NOT here. Measured: of 11 matches across the
  // five repos, 10 were `path-safety` and none were the 25010 characteristic
  // — this ecosystem uses the word as a suffix for memory/path/type safety,
  // which is a different concept from freedom from harm. The words that name
  // the characteristic itself are kept.
  ["safety", /\b(hazard|harm to|injur|fail-safe|mitigat(e|ion) of risk)/i],
  ["serialization", /\b(serializ|round-?trip|encode[sd]? (to|as)|to_json)/i],
  [
    "single-component",
    /\b(single (function|component|module|unit)|in isolation|pure function)\b/i,
  ],
  [
    "stakeholder-acceptance",
    /\b(sign-?off|accepted by|acceptance by|demonstrat(e|ed|ion) to)\b/i,
  ],
  [
    "stakeholder-facing",
    // `business` is not here: it matched the repo name `spec-objects-business`.
    /\b(stakeholder|end user|customer|operator-facing)\b/i,
  ],
  [
    "state-machine",
    /\b(state machine|state transition|transitions? (from|to|into)|finite state)\b/i,
  ],
  [
    "structured-input",
    /\b(grammar|structured input|well-formed|schema-valid|malformed (document|input|payload))\b/i,
  ],
  [
    "supply-chain",
    /\b(supply chain|\bSBOM\b|vendored|transitive dependenc|provenance)\b/i,
  ],
  [
    "undefined-behaviour",
    /\b(undefined behaviou?r|\bUB\b|data race|uninitializ|out[- ]of[- ]bounds)/i,
  ],
  [
    "universally-quantified",
    /\b(for (every|all) [a-z]|every input|any input|universally|holds for)\b/i,
  ],
  [
    "workflow",
    // `scenario` dropped: in this corpus it names a test case or an example,
    // not a user-facing sequence of steps.
    /\b(workflow|user journey|step \d|then the (user|operator))\b/i,
  ],
];

/**
 * Characteristics readable from an obligation's statement.
 *
 * Mostly prose regexes, plus one **structural** signal: a statement that parses
 * as a declared configuration space (quire-rs FR-061) is a configuration matrix
 * by construction, whatever words it happens to contain.
 *
 * The regex alone would miss it. A minted space reads
 * `2-way over features(default|python|wasm) target(linux|wasm32)` — no
 * "configuration", no "feature flag", no "combination of" — so the very
 * obligations that most need the combinatorial method would be the ones it was
 * never advised for. A structural test cannot false-positive on prose either,
 * which a widened regex would.
 */
/**
 * The prose of a statement, with markdown link **targets** removed.
 *
 * A criterion cites its neighbours constantly —
 * `([StR-005-AC-4](../stakeholder/StR-005-no-runtime.md))` — and the path is
 * not something the author said about the requirement. Measured across the five
 * repos in this ecosystem when this was added: of 13 statements matching
 * `stakeholder`, **12 matched the directory name `../stakeholder/` inside a
 * link target** and one was the word in prose. Every characteristic reading the
 * raw string inherited that noise, the twenty that predate #128 included.
 *
 * Link *text* is kept: `[the evidence store](./FR-030.md)` is prose the author
 * wrote. Only the destination goes.
 */
function prose(statement: string): string {
  return statement.replace(/\]\([^)]*\)/g, "]");
}

export function characteristicsOf(
  statement: string,
  criticality?: string | null,
): string[] {
  const text = prose(statement);
  const matched = STATEMENT_CHARACTERISTICS.filter(([, re]) =>
    re.test(text),
  ).map(([name]) => name);
  if (parseSpace(statement) !== null) matched.push("configuration-matrix");
  // Read from the value, never inferred from a threshold this code chose.
  // Worth stating plainly: 2,304 of 2,304 `Acceptance Criteria` tables in the
  // ecosystem carry no criticality column, so this mints for nothing today.
  // That is the honest state — `mutation-testing` becomes reachable in
  // principle and stays unreached in practice until a spec declares one, and
  // the join check now shows that rather than hiding it.
  if (criticality && /^(p0|high|critical)$/i.test(criticality.trim())) {
    matched.push("high-criticality");
  }
  return [...new Set(matched)].sort();
}

/**
 * Every `characteristics` value any code path in quoin can produce.
 *
 * This exists so the **join** between the catalog and the fact set is
 * checkable. The catalog declares the values that trigger a method; this is the
 * set that can ever be produced to match them. Nothing compared the two, and
 * the consequence was silent: `matchRules` skips an unknown *axis* by design
 * (FR-054-CON-2 leaves the axis set open) but an unknown *value* on a known
 * axis simply never matches, and `inconclusive` is already a legitimate
 * outcome. A systematically under-advising advisor was indistinguishable from a
 * quiet one.
 *
 * Measured when this was added: the installed catalog declared **60** values
 * and 20 were producible, leaving **7 methods** — `integration-testing` and
 * `mutation-testing` among them — that no statement could ever reach
 * (agent-ix/quoin#128).
 *
 * Derived from the regex table rather than hand-listed, because a hand-listed
 * copy is the failure mode this whole check exists to catch.
 */
export function mintableCharacteristics(): Set<string> {
  return new Set([
    ...STATEMENT_CHARACTERISTICS.map(([name]) => name),
    // Not lexical: read from the obligation's own criticality field.
    "high-criticality",
  ]);
}

/**
 * Recommend methods for one obligation.
 *
 * A method is recommended when **any** of its applicability rules matches a
 * fact about the obligation. Ranking is by number of matching rules — a method
 * two axes agree on outranks one a single axis suggested — with the method id
 * as a deterministic tiebreak, so the same input always yields the same order.
 */
export function advise(catalog: MethodCatalog, facts: ObligationFacts): Advice {
  const characteristics = new Set(
    characteristicsOf(facts.statement, facts.criticality),
  );
  const shapes = new Set(facts.propertyShape ? [facts.propertyShape] : []);
  const objects = new Set(facts.objectTypes ?? []);
  const archetypes = new Set(facts.archetype ? [facts.archetype] : []);

  const recommended: Recommendation[] = [];
  for (const method of catalog.methods) {
    const reasons = matchRules(method, {
      characteristics,
      property_shapes: shapes,
      object_types: objects,
      archetypes,
    });
    if (reasons.length === 0) continue;
    recommended.push({
      method: method.id,
      class: method.class,
      evidenceKind: method.evidenceKind,
      reasons,
    });
  }

  recommended.sort(
    (a, b) =>
      b.reasons.length - a.reasons.length || compare(a.method, b.method),
  );

  const authored = normalizeAuthored(facts.authoredMethod);
  const inconclusive = recommended.length === 0;
  // A mismatch is only meaningful when the advisor had something to say. With
  // no rules matched, "the author chose Test and we recommend nothing" is not a
  // disagreement — it is silence, and reporting it as a mismatch would bury the
  // real ones.
  const mismatch =
    !inconclusive &&
    authored !== null &&
    authored !== undefined &&
    !recommended.some(
      (r) =>
        r.method.toLowerCase() === authored.toLowerCase() ||
        r.class.toLowerCase() === authored.toLowerCase(),
    );

  return {
    obligation: facts.id,
    recommended,
    authored,
    mismatch,
    inconclusive,
  };
}

function matchRules(
  method: VerificationMethod,
  facts: Record<string, Set<string>>,
): MatchReason[] {
  const reasons: MatchReason[] = [];
  for (const [rule, values] of Object.entries(method.applicability)) {
    const known = facts[rule];
    // A rule naming an axis the advisor cannot observe is skipped, not failed.
    // The engine deliberately leaves the axis set open (FR-054-CON-2), so a
    // module may declare rules this advisor has no facts for — that is a gap in
    // what can be observed, not a reason to reject the method.
    if (!known) continue;
    for (const value of values) {
      if (known.has(value)) reasons.push({ rule, value });
    }
  }
  return reasons.sort(
    (a, b) => compare(a.rule, b.rule) || compare(a.value, b.value),
  );
}

/** `Test (TC-707)` → `Test`; an empty cell → `null`. */
function normalizeAuthored(cell: string | null | undefined): string | null {
  if (!cell) return null;
  const head = cell.includes("(") ? cell.slice(0, cell.indexOf("(")) : cell;
  const trimmed = head.trim();
  return trimmed.length > 0 ? trimmed : null;
}

/** Locale-independent string order (agent-ix/quoin#106). */
function compare(a: string, b: string): number {
  return a === b ? 0 : a < b ? -1 : 1;
}
