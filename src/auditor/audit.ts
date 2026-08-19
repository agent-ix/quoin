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
import { scanIsVacuous } from "../evidence/index.js";
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
    | "insufficient-multiplicity";
  obligation: string;
  severity: Severity;
  summary: string;
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

  for (const obligation of [...input.obligations].sort(byId)) {
    const bindings = (bindingsByObligation.get(obligation.id) ?? [])
      .slice()
      .sort((a, b) => compare(a.suite, b.suite));

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

    healthy.push(obligation.id);
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
 * Two questions about the declared method, both answered from the catalog.
 *
 * **1. Is the method declared at all?** A `Verification` cell naming neither a
 * catalog method id nor a catalog class used to return `null` here — a silent
 * skip — so the requirements whose verification is *least* well defined were
 * exactly the ones nothing questioned. Measured across the ecosystem when this
 * landed: 55 of 577 obligations (agent-ix/quoin#105, quire-rs#152).
 *
 * **2. Did the evidence match the method's kind?** Compared **kind to kind**.
 * The old test was `run.entries.length > 0` — read as "this was a test run",
 * but true of a transcribed inspection too, so every `Inspection` or `Analysis`
 * obligation recorded through `quoin evidence record` was flagged. A run now
 * declares its own `evidenceKind`, and when it declares none the check says
 * nothing: an undeclared kind means the question cannot be asked, which is a
 * different answer from "it conformed".
 */
function methodConformance(
  obligation: Obligation,
  bindings: Binding[],
  runs: RunRecord[],
  catalog: MethodCatalog | undefined,
): Finding | null {
  if (!catalog || !obligation.method) return null;
  const declared = obligation.method.trim().toLowerCase();

  const matched = catalog.methods.filter(
    (m) =>
      m.id.toLowerCase() === declared || m.class.toLowerCase() === declared,
  );

  if (matched.length === 0) {
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
