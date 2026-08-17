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
  ["layering", /\b(depend on|layering|module boundary|must not import)\b/i],
  ["user-visible", /\b(user|operator|the UI|displays|screen)\b/i],
  ["stable-output", /\b(byte-identical|identical output|serializ|snapshot)\b/i],
  ["agent-behaviour", /\b(agent|transcript|prompt)\b/i],
  ["no-executable-oracle", /\b(review|judgement|readable|documented)\b/i],
];

/** Characteristics readable from an obligation's statement. */
export function characteristicsOf(statement: string): string[] {
  return STATEMENT_CHARACTERISTICS.filter(([, re]) => re.test(statement))
    .map(([name]) => name)
    .sort();
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
  const characteristics = new Set([
    ...characteristicsOf(facts.statement),
    ...(facts.archetype === "NFR" ? [] : []),
  ]);
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
      b.reasons.length - a.reasons.length || a.method.localeCompare(b.method),
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
    (a, b) => a.rule.localeCompare(b.rule) || a.value.localeCompare(b.value),
  );
}

/** `Test (TC-707)` → `Test`; an empty cell → `null`. */
function normalizeAuthored(cell: string | null | undefined): string | null {
  if (!cell) return null;
  const head = cell.includes("(") ? cell.slice(0, cell.indexOf("(")) : cell;
  const trimmed = head.trim();
  return trimmed.length > 0 ? trimmed : null;
}
