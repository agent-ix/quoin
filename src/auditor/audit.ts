/**
 * The evidence auditor (FR-032).
 *
 * A trace link is currently a string match that never expires. Evidence rots in
 * three ways, none detected before this:
 *
 *  1. **Suspect links** — the statement changed after the evidence was bound.
 *  2. **Stale evidence** — bound to an old commit, a failed run, or a run that
 *     never happened.
 *  3. **Vacuous evidence** — a tagged test that is skipped, xfail, or
 *     assertion-free. quire-rs#72's 1,014 dead tags are the extreme case; this
 *     owns the whole family.
 *
 * **The auditor runs nothing.** It reads the store and reports (ADR-0011
 * invariant 1); the consumer's CI refreshes. That separation is what lets the
 * report be trusted: an auditor that could re-run a suite could also make a
 * finding disappear by re-running it.
 */

import type { Obligation } from "../quire/index.js";
import { MUTATION_SCORE_METRIC, scanIsVacuous } from "../evidence/index.js";
import { parseSpace, twayCoverage } from "./combinatorial.js";
import type { Binding, FindingRecord, RunRecord } from "../evidence/index.js";
import type { MethodCatalog } from "../advisor/index.js";

/** Severity of one finding. Matches the SpecReview vocabulary. */
export type Severity = "low" | "medium" | "high";

/** One thing wrong with the evidence for one obligation. */
export interface Finding {
  kind:
    | "suspect-link"
    | "stale-evidence"
    | "vacuous-evidence"
    | "combinatorial-gap"
    | "undischarged"
    | "method-conformance"
    | "unknown-method"
    | "insufficient-multiplicity"
    | "insufficient-mutation-score"
    | "unmeasured-mutation-score"
    | "mocked-confirmation";
  obligation: string;
  severity: Severity;
  summary: string;
}

/**
 * A stand-in a suite injects for real behaviour.
 *
 * Pass 2 found `FR-017-AC-7`'s trusted-UI confirmation had NO implementation,
 * and its test passed by injecting `Confirmation::allow()` — mocking exactly
 * the behaviour the criterion verifies. The test was green, the AC was green,
 * and the behaviour did not exist.
 */
export interface MockInjection {
  /** Suite the injection was observed in. */
  suite: string;
  /** Symbol doing the injecting. */
  symbol: string;
  /** Identifiers substituted for real behaviour — `Confirmation::allow`. */
  injects: string[];
}

/** What the auditor was given to read. */
export interface AuditInput {
  /** Obligations as quire derives them today. */
  obligations: Obligation[];
  /** The binding graph from the store. */
  bindings: Binding[];
  /** Every run record the store holds, newest per suite. */
  runs: RunRecord[];
  /**
   * What each suite's tests INJECT in place of real behaviour (#204).
   *
   * Supplied by the caller because the auditor reads the store, not source —
   * and #204 asked to extend `evidence audit`, not to build a second system.
   * An absent list means "nobody looked", never "nothing was mocked": the
   * check stays silent, the same posture `scanIsVacuous` takes.
   */
  injections?: MockInjection[];
  /**
   * Every finding-shaped scan record the store holds, newest per suite
   * (FR-034).
   *
   * Separate from `runs` because the two answer different questions. A scan
   * has no symbols and no pass/fail, so every check that reasons over
   * `entries` is meaningless against it — and silently folding them together
   * is how a scan would start reading as a run that skipped everything.
   */
  scans?: FindingRecord[];
  /** The merged verification-method catalog, for method conformance. */
  catalog?: MethodCatalog;
  /** Commit being audited, for freshness. */
  headCommit?: string;
  /** Criticality values that demand two independent methods. */
  multiplicityRequires?: string[];
  /**
   * Criticality value → the mutation score its obligations must reach.
   *
   * **Unset by default**, for the reason CR-008 removed the hardcoded
   * `multiplicityRequires: ["P0"]`: a built-in floor is a rule that fires on
   * everything the moment a criticality column appears, and nobody chose it.
   *
   * A score is a ratio in `[0, 1]` — killed mutants over viable ones, as the
   * `cargo-mutants` adapter computes it. Unviable mutants are in neither side.
   */
  mutationFloor?: Record<string, number>;
}

/** The audit result, ordered so the same input yields the same report. */
export interface AuditReport {
  findings: Finding[];
  /** Obligations with a binding whose hash still matches. */
  healthy: string[];
}

/**
 * Audit the store against the obligations of the day.
 *
 * Every check is deterministic and reads only what it was handed. There is no
 * clock, no filesystem walk and no subprocess in here — the caller assembles
 * the inputs, which is what makes the whole thing testable without a repository.
 */
export function audit(input: AuditInput): AuditReport {
  const findings: Finding[] = [];
  const healthy: string[] = [];

  // An obligation can be discharged by more than one suite — a unit suite and
  // a mutation suite, say — so the graph is grouped, not indexed. Keying on the
  // obligation alone was what let the second suite's binding overwrite the
  // first (agent-ix/quoin#102).
  const bindingsByObligation = new Map<string, Binding[]>();
  for (const binding of input.bindings) {
    const group = bindingsByObligation.get(binding.obligation) ?? [];
    group.push(binding);
    bindingsByObligation.set(binding.obligation, group);
  }
  const runsBySuite = new Map(input.runs.map((r) => [r.suite, r]));
  const scansBySuite = new Map((input.scans ?? []).map((s) => [s.suite, s]));
  // Absent means "nobody looked", never "nothing was mocked" — the check stays
  // silent rather than reporting a clean bill it did not earn (#204).
  const injections = input.injections ?? [];

  for (const obligation of [...input.obligations].sort(byId)) {
    const findingsBefore = findings.length;
    const bindings = (bindingsByObligation.get(obligation.id) ?? [])
      .slice()
      .sort((a, b) => compare(a.suite, b.suite));

    // ── 0. Unknown method ──
    // A pure statement-vs-catalog comparison: it needs no bindings, no runs
    // and no evidence store, so it is asked BEFORE the binding guard. Behind
    // it, the one check that pays off on day one of adoption — "your
    // Verification column names methods no catalog declares" — could never
    // fire on an unadopted repository, whose every obligation is unbound
    // (agent-ix/quoin#165: 1,107 findings, all `undischarged`, zero
    // `unknown-method`, against 90+ uncatalogued Verification values).
    //
    // It does not `continue`: the evidence ladder below stays
    // one-finding-per-obligation, and this statement-level finding coexists
    // with whichever evidence finding the ladder reaches — an unbound
    // obligation with an uncatalogued method is BOTH undischarged AND
    // unknown-method, and hiding either behind the other cost a reader the
    // ability to see both facts.
    const unknown = unknownMethodFinding(obligation, input.catalog);
    if (unknown) findings.push(unknown);

    if (bindings.length === 0) {
      findings.push({
        kind: "undischarged",
        obligation: obligation.id,
        // Medium, not high: an unwritten test is ordinary work in progress.
        // What is high is evidence that *claims* to exist and does not hold.
        severity: "medium",
        summary: `no evidence is bound to ${obligation.id}`,
      });
      continue;
    }

    // ── 1. Suspect link ──
    // Reported when ANY binding predates the current statement. A sibling suite
    // that re-bound after the reword does not absolve the one that did not:
    // that binding still claims to discharge a statement it never saw.
    const suspect = bindings.filter(
      (b) => b.statementHashAtBinding !== obligation.statement_hash,
    );
    if (suspect.length > 0) {
      findings.push({
        kind: "suspect-link",
        obligation: obligation.id,
        // The highest-value single finding in the traceability design: the
        // requirement moved and the evidence did not follow, while the matrix
        // still reads as covered.
        severity: "high",
        summary:
          `${obligation.id} was reworded after its evidence was bound in ` +
          `${suspect.map((b) => b.suite).join(", ")} ` +
          `(bound against ${suspect[0].statementHashAtBinding.slice(0, 12)}…, ` +
          `now ${obligation.statement_hash.slice(0, 12)}…). Re-affirm or re-verify.`,
      });
      continue;
    }

    // ── 2. Stale evidence ──
    // A suite that recorded a SCAN has evidence; it simply is not run-shaped.
    // Before FR-034 the store held no scans, so this reduces to the previous
    // behaviour for every existing consumer.
    const unrecorded = bindings.filter(
      (b) => !runsBySuite.has(b.suite) && !scansBySuite.has(b.suite),
    );
    if (unrecorded.length > 0) {
      findings.push({
        kind: "stale-evidence",
        obligation: obligation.id,
        severity: "high",
        summary:
          `${obligation.id} is bound to ${unrecorded.map((b) => b.suite).join(", ")}, ` +
          `which ${unrecorded.length === 1 ? "has" : "have"} no recorded run. ` +
          `The binding claims evidence that is not in the store.`,
      });
      continue;
    }

    // ── Mocked confirmation (#204) ──
    // The behaviour the criterion verifies, substituted by the test that
    // verifies it. Pass 2's case: FR-017-AC-7's trusted-UI confirmation had NO
    // implementation, and its test passed by injecting `Confirmation::allow()`.
    // Green test, green AC, absent behaviour.
    //
    // Reported only when EVERY binding is mocked. One suite standing in a
    // dependency while another exercises the real path is ordinary test
    // design, and flagging it would fire on most of the corpus for a reason
    // that has nothing to do with this defect.
    //
    // `medium`, not `high`: this is a heuristic over identifiers, and a
    // legitimate mock can share a noun with the statement it appears under.
    const mocked = mockedBindings(obligation, bindings, injections);
    if (mocked.length > 0 && mocked.length === bindings.length) {
      findings.push({
        kind: "mocked-confirmation",
        obligation: obligation.id,
        severity: "medium",
        summary:
          `${obligation.id} is discharged only by tests that inject a stand-in ` +
          `for the behaviour it verifies: ` +
          mocked
            .map((m) => `${m.symbol} injects ${m.injects.join(", ")}`)
            .sort(compare)
            .join("; ") +
          `. A test that substitutes the behaviour under verification passes ` +
          `whether or not that behaviour exists.`,
      });
      continue;
    }

    // ── Finding-shaped vacuity ──
    // A scan that ran every rule and reported nothing is a CLEAN RESULT, and
    // is precisely the evidence FR-034 exists to preserve. What proves nothing
    // is a scan that evaluated NO RULES: it reports zero findings too, and from
    // the findings list alone the two are identical.
    //
    // A tool that does not say how many rules it evaluated leaves the question
    // unanswerable, and `scanIsVacuous` returns null — the check stays silent
    // rather than guessing, the same posture method conformance takes when no
    // evidence kind is declared (agent-ix/quoin#105).
    const emptyScans = bindings
      .map((b) => scansBySuite.get(b.suite))
      .filter((s): s is FindingRecord => s !== undefined)
      .filter((s) => scanIsVacuous(s) === true);
    if (emptyScans.length > 0) {
      findings.push({
        kind: "vacuous-evidence",
        obligation: obligation.id,
        severity: "high",
        summary:
          `${obligation.id} rests on a scan that evaluated no rules: ` +
          emptyScans
            .map((s) => `${s.suite} (${s.tool})`)
            .sort(compare)
            .join(", ") +
          `. It reported no finding because it looked for nothing.`,
      });
      continue;
    }

    // Every remaining check reasons over run entries, so a binding backed only
    // by a scan has nothing more to answer here.
    const runBindings = bindings.filter((b) => runsBySuite.has(b.suite));
    if (runBindings.length === 0) continue;
    const runs = runBindings.map((b) => runsBySuite.get(b.suite)!);
    if (input.headCommit) {
      const behind = bindings.filter(
        (b, i) => runs[i].commit !== input.headCommit,
      );
      if (behind.length > 0) {
        findings.push({
          kind: "stale-evidence",
          obligation: obligation.id,
          // Medium: an older run is normal between releases. It becomes
          // actionable at a gate, which is the consuming workflow's policy.
          severity: "medium",
          summary:
            `${obligation.id} rests on ` +
            behind
              .map(
                (b) =>
                  `a ${b.suite} run at ${runsBySuite.get(b.suite)!.commit.slice(0, 12)}`,
              )
              .join(" and ") +
            `, not at HEAD (${input.headCommit.slice(0, 12)}).`,
        });
      }
    }

    // ── 3. Vacuous evidence ──
    // Vacuous only when EVERY symbol in EVERY suite was skipped or absent. One
    // suite that genuinely ran is evidence; reporting the obligation as vacuous
    // because a second suite skipped would be the false alarm that gets the
    // check switched off.
    // `runBindings`, not `bindings`: `runs` was built from the former, so
    // indexing the latter would pair a binding with another suite's run the
    // moment any binding is scan-backed.
    const vacuous = runBindings.flatMap((b, i) =>
      b.symbols
        .filter((symbol) => {
          const entry = runs[i].entries.find((e) => e.symbol === symbol);
          // A symbol the run never reported is vacuous in the strongest sense:
          // the binding names evidence the suite did not produce.
          if (!entry) return true;
          return entry.outcome === "skip";
        })
        .map((symbol) => `${b.suite}:${symbol}`),
    );
    const boundSymbols = runBindings.reduce((n, b) => n + b.symbols.length, 0);
    if (vacuous.length > 0 && vacuous.length === boundSymbols) {
      findings.push({
        kind: "vacuous-evidence",
        obligation: obligation.id,
        severity: "high",
        summary:
          `every symbol bound to ${obligation.id} was skipped or absent from ` +
          `its run: ${vacuous.sort(compare).join(", ")}. The row reads as ` +
          `covered and nothing was verified.`,
      });
      continue;
    }

    // ── Combinatorial coverage ──
    // Only for an obligation whose statement declares a configuration space
    // (quire-rs FR-061). `parseSpace` returning null IS the test for that, so
    // there is no second flag to keep in agreement with the first.
    const space = parseSpace(obligation.statement ?? "");
    if (space) {
      const configs = runs.flatMap((r) =>
        r.entries
          .map((e) => e.config)
          .filter((c): c is Record<string, string> => !!c),
      );
      const coverage = twayCoverage(space, configs);
      if (coverage.covered < coverage.demanded) {
        findings.push({
          kind: "combinatorial-gap",
          obligation: obligation.id,
          // Medium: an incomplete covering array is ordinary work in progress.
          // What would be high is a run claiming completeness it does not have,
          // which is what the gap list makes impossible to do quietly.
          severity: "medium",
          summary:
            `${obligation.id} demands ${coverage.demanded} ${coverage.strength}-way ` +
            `combinations and its runs reached ${coverage.covered}. ` +
            // Named, not counted. A percentage says how much is missing; the
            // list says which combinations to run, which is the difference
            // between a number and an action.
            `Never run: ${coverage.gaps.slice(0, 10).join("; ")}` +
            (coverage.gaps.length > 10
              ? ` (and ${coverage.gaps.length - 10} more)`
              : ""),
        });
        continue;
      }
    }

    // ── Method conformance ──
    const conformance = methodConformance(
      obligation,
      runBindings,
      runs,
      input.catalog,
    );
    if (conformance) {
      findings.push(conformance);
      continue;
    }

    // ── Multiplicity ──
    const multiplicity = multiplicityFinding(obligation, bindings, input);
    if (multiplicity) {
      findings.push(multiplicity);
      continue;
    }

    // ── Mutation score ──
    const mutation = mutationFinding(obligation, runBindings, runs, input);
    if (mutation) {
      findings.push(mutation);
      continue;
    }

    // Healthy means NOTHING was found for this obligation — including the
    // pre-guard unknown-method check, which does not `continue`.
    if (findings.length === findingsBefore) healthy.push(obligation.id);
  }

  // Plain comparison, not `localeCompare`: the report is meant to be
  // reproducible, and `localeCompare` without an explicit locale depends on the
  // runtime's ICU data (agent-ix/quoin#106).
  findings.sort(
    (a, b) => compare(a.obligation, b.obligation) || compare(a.kind, b.kind),
  );
  return { findings, healthy: healthy.sort(compare) };
}

/** Locale-independent string order. */
function compare(a: string, b: string): number {
  return a === b ? 0 : a < b ? -1 : 1;
}

function byId(a: Obligation, b: Obligation): number {
  return compare(a.id, b.id);
}

/**
 * Is the declared method declared at all?
 *
 * A `Verification` cell naming neither a catalog method id nor a catalog class
 * used to return `null` from `methodConformance` — a silent skip — so the
 * requirements whose verification is *least* well defined were exactly the
 * ones nothing questioned. Measured across the ecosystem when this landed:
 * 55 of 577 obligations (agent-ix/quoin#105, quire-rs#152).
 *
 * Separate from `methodConformance` and asked before the binding guard,
 * because this comparison needs no evidence at all — the statement and the
 * catalog are the whole input (agent-ix/quoin#165).
 */
function unknownMethodFinding(
  obligation: Obligation,
  catalog: MethodCatalog | undefined,
): Finding | null {
  if (!catalog || !obligation.method) return null;
  if (catalogMethodsMatching(obligation.method, catalog).length > 0) {
    return null;
  }
  return {
    kind: "unknown-method",
    obligation: obligation.id,
    severity: "medium",
    summary:
      `${obligation.id} declares \`${obligation.method}\`, which is neither a ` +
      `catalog method id nor a catalog class. Nothing can say what discharging ` +
      `it means, so no conformance check can run against it.`,
  };
}

/** Catalog methods whose id or class matches the declared method. */
function catalogMethodsMatching(method: string, catalog: MethodCatalog) {
  const declared = method.trim().toLowerCase();
  return catalog.methods.filter(
    (m) =>
      m.id.toLowerCase() === declared || m.class.toLowerCase() === declared,
  );
}

/**
 * Did the evidence match the declared method's kind? Compared **kind to
 * kind**. The old test was `run.entries.length > 0` — read as "this was a test
 * run", but true of a transcribed inspection too, so every `Inspection` or
 * `Analysis` obligation recorded through `quoin evidence record` was flagged.
 * A run now declares its own `evidenceKind`, and when it declares none the
 * check says nothing: an undeclared kind means the question cannot be asked,
 * which is a different answer from "it conformed".
 *
 * An uncatalogued method returns `null` here without a finding — that is not
 * the old silent skip: `unknownMethodFinding` has already reported it, before
 * the binding guard.
 */
function methodConformance(
  obligation: Obligation,
  bindings: Binding[],
  runs: RunRecord[],
  catalog: MethodCatalog | undefined,
): Finding | null {
  if (!catalog || !obligation.method) return null;
  const matched = catalogMethodsMatching(obligation.method, catalog);
  if (matched.length === 0) return null;

  // The kinds the catalog says this method produces. An entry declaring none
  // makes the comparison unanswerable rather than failed.
  const expected = new Set(
    matched
      .map((m) => m.evidenceKind)
      .filter((k): k is string => typeof k === "string" && k.length > 0),
  );
  if (expected.size === 0) return null;

  const mismatched = bindings.filter((b, i) => {
    const kind = runs[i].evidenceKind;
    return typeof kind === "string" && kind.length > 0 && !expected.has(kind);
  });
  if (mismatched.length === 0) return null;

  return {
    kind: "method-conformance",
    obligation: obligation.id,
    severity: "medium",
    summary:
      `${obligation.id} declares \`${obligation.method}\`, whose evidence kind is ` +
      `${[...expected].sort(compare).join("/")}, but is discharged by ` +
      mismatched
        .map(
          (b) =>
            `a ${runs[bindings.indexOf(b)].evidenceKind} run in ${b.suite}`,
        )
        .join(" and ") +
      `. A method is not discharged by evidence of another kind.`,
  };
}

/**
 * Criticality can demand two independent methods.
 *
 * "Independent" means two different suites: two symbols in one suite share a
 * harness, a fixture set and a failure mode, so counting them as two methods
 * would let one broken assumption look like corroboration.
 */
function multiplicityFinding(
  obligation: Obligation,
  bindings: Binding[],
  input: AuditInput,
): Finding | null {
  const demanding = input.multiplicityRequires ?? [];
  if (demanding.length === 0 || !obligation.criticality) return null;
  if (!demanding.includes(obligation.criticality)) return null;

  // Now a real measurement. While `bind()` keyed on the obligation alone this
  // set could never hold more than one suite, so the finding fired on every
  // demanding obligation and no amount of evidence could clear it (#102).
  const suites = new Set(bindings.map((b) => b.suite));
  if (suites.size >= 2) return null;

  return {
    kind: "insufficient-multiplicity",
    obligation: obligation.id,
    severity: "medium",
    summary:
      `${obligation.id} is criticality ${obligation.criticality}, which requires ` +
      `two independent methods, but all its evidence comes from ` +
      `${[...suites].join(", ")}.`,
  };
}

/**
 * Criticality can demand that the tests have been shown to detect faults.
 *
 * **Why this check exists at all.** Every other quality signal in this program
 * is a proxy — does the criterion use a vague verb, does it name a concrete
 * object, is it shaped as an assertion. Those are word lists over an open
 * vocabulary, and the corpus already showed what that is worth: 1,201 distinct
 * verb stems, of which a built-in list of 13 covered 14.5%.
 *
 * A mutation score is the direct answer to the question those proxies
 * approximate — *does the test discriminate the behaviour the criterion
 * describes?* A surviving mutant means either the tests do not check that
 * behaviour or the criterion never demanded it, and both are findings.
 *
 * **quoin does not run the mutation tool** (ADR-0011: the consumer's CI does).
 * This reads a score somebody else recorded, which is why the absence of one is
 * reported rather than filled in.
 */
function mutationFinding(
  obligation: Obligation,
  bindings: Binding[],
  runs: RunRecord[],
  input: AuditInput,
): Finding | null {
  const floors = input.mutationFloor ?? {};
  if (!obligation.criticality) return null;
  const floor = floors[obligation.criticality];
  if (floor === undefined) return null;

  const scores = scoresFor(bindings, runs);
  if (scores.length === 0) {
    // Distinct from `undischarged`: this obligation may be thoroughly tested and
    // still have nothing saying the tests detect anything. A demanded threshold
    // that cannot be evaluated is not a threshold met.
    return {
      kind: "unmeasured-mutation-score",
      obligation: obligation.id,
      severity: "medium",
      summary:
        `${obligation.id} is criticality ${obligation.criticality}, which demands a ` +
        `mutation score of at least ${floor}, and no run bound to it records one.`,
    };
  }

  // The WORST score, not the mean. Averaging lets a well-tested symbol carry a
  // symbol whose mutants all survive, which is the case the threshold exists to
  // find.
  const worst = Math.min(...scores);
  if (worst >= floor) return null;
  return {
    kind: "insufficient-mutation-score",
    obligation: obligation.id,
    severity: "medium",
    summary:
      `${obligation.id} is criticality ${obligation.criticality}, which demands a ` +
      `mutation score of at least ${floor}; its weakest bound symbol scores ${worst}.`,
  };
}

/**
 * Fault-detection scores among the runs bound to this obligation.
 *
 * **Filtered on the entry's own `metric`, never on the tool name.**
 * `RunEntry.score` is deliberately generic — its own contract says "a mutation
 * score, a coverage percentage, a measured latency" — so reading every scored
 * entry would compare a p95 latency in milliseconds against a floor of `0.8`
 * and report the obligation as failing. The first draft did exactly that; the
 * first fix scoped by tool name against the catalog's
 * `mutation-testing.tooling`, which named a method id in the engine, was a
 * tool allowlist by another name, and did not generalize. The adapter now
 * names what it measured at the point of recording (agent-ix/quoin#138), and
 * an entry without a `metric` is not a mutation score — no fallback read of
 * the tool string.
 *
 * Exported for the advisor (agent-ix/quoin#158), which needs the same answer
 * to decide whether anything has measured that the tests discriminate. One
 * definition, so the auditor's finding and the advisor's recommendation cannot
 * disagree about what a score is.
 */
/**
 * How much of an injected identifier must overlap the statement's own words
 * before the injection is read as standing in for the verified behaviour.
 *
 * Deliberately high. A test legitimately mocks a clock, a filesystem, a
 * network — all of which share nothing with the statement. What this is
 * looking for is the narrow case where the mock's NAME is the statement's
 * subject, which is what `Confirmation::allow` against a trusted-UI
 * confirmation criterion looks like.
 */
export const MOCK_SUBJECT_FLOOR = 0.5;

/** Bindings whose suite injects a stand-in for this obligation's subject. */
function mockedBindings(
  obligation: Obligation,
  bindings: Binding[],
  injections: MockInjection[],
): MockInjection[] {
  if (injections.length === 0) return [];
  const subject = words(obligation.statement);
  if (subject.size === 0) return [];

  const out: MockInjection[] = [];
  for (const binding of bindings) {
    const hit = injections
      .filter((i) => i.suite === binding.suite)
      .find((i) =>
        i.injects.some((identifier) => {
          const tokens = words(identifier);
          if (tokens.size === 0) return false;
          let shared = 0;
          for (const token of tokens) if (subject.has(token)) shared += 1;
          return shared / tokens.size >= MOCK_SUBJECT_FLOOR;
        }),
      );
    if (hit) out.push(hit);
  }
  return out;
}

/** Lowercased word-ish tokens, minus the words every sentence shares. */
function words(text: string): Set<string> {
  const noise = new Set([
    "the",
    "a",
    "an",
    "and",
    "or",
    "of",
    "to",
    "for",
    "in",
    "is",
    "be",
    "shall",
    "with",
    "that",
    "it",
    "its",
    "on",
    "by",
    "as",
    "not",
    "new",
  ]);
  return new Set(
    text
      .split(/[^A-Za-z0-9]+/)
      .map((t) => t.toLowerCase())
      .filter((t) => t.length > 2 && !noise.has(t)),
  );
}

export function scoresFor(bindings: Binding[], runs: RunRecord[]): number[] {
  const bound = new Set(bindings.map((b) => b.suite));
  const out: number[] = [];
  for (const run of runs) {
    if (!bound.has(run.suite)) continue;
    for (const entry of run.entries) {
      if (entry.metric !== MUTATION_SCORE_METRIC) continue;
      // `skip` carries no measurement: a skipped symbol's absent score is not a
      // zero, and treating it as one would fail an obligation for a test nobody
      // ran rather than for a test that failed to discriminate.
      if (entry.score !== undefined && entry.outcome !== "skip") {
        out.push(entry.score);
      }
    }
  }
  return out;
}

/** The baseline key for one finding. */
export function findingKey(finding: Finding): string {
  return `${finding.kind}:${finding.obligation}`;
}

/**
 * Findings not present in the baseline — what a ratchet fails on.
 *
 * The baseline is a flat set of `<kind>:<obligation>` keys covering **every**
 * finding kind. It used to be two named buckets, `undischarged` and `suspect`,
 * so `stale-evidence`, `vacuous-evidence`, `method-conformance`,
 * `unknown-method` and `insufficient-multiplicity` could never be baselined and
 * `--ratchet` reported the whole existing backlog for all five — which is the
 * outcome this function's own comment says ratchet mode exists to prevent
 * (agent-ix/quoin#105).
 */
export function ratchet(
  report: AuditReport,
  baseline: { accepted: string[] },
): Finding[] {
  const accepted = new Set(baseline.accepted);
  // Ratchet mode exists because a gate that fails on the whole existing
  // backlog gets disabled within a week. Only NEW violations fail.
  return report.findings.filter((f) => !accepted.has(findingKey(f)));
}

/** What changed between two audits — the per-PR delta. */
export function delta(
  before: AuditReport,
  after: AuditReport,
): { added: Finding[]; resolved: Finding[] } {
  const beforeKeys = new Set(before.findings.map(findingKey));
  const afterKeys = new Set(after.findings.map(findingKey));
  return {
    added: after.findings.filter((f) => !beforeKeys.has(findingKey(f))),
    resolved: before.findings.filter((f) => !afterKeys.has(findingKey(f))),
  };
}
