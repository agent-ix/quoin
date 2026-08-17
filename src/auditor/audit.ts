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
import type { Binding, RunRecord } from "../evidence/index.js";
import type { MethodCatalog } from "../advisor/index.js";

/** Severity of one finding. Matches the SpecReview vocabulary. */
export type Severity = "low" | "medium" | "high";

/** One thing wrong with the evidence for one obligation. */
export interface Finding {
  kind:
    | "suspect-link"
    | "stale-evidence"
    | "vacuous-evidence"
    | "undischarged"
    | "method-conformance"
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

  const bindingByObligation = new Map(
    input.bindings.map((b) => [b.obligation, b]),
  );
  const runsBySuite = new Map(input.runs.map((r) => [r.suite, r]));

  for (const obligation of [...input.obligations].sort((a, b) =>
    a.id.localeCompare(b.id),
  )) {
    const binding = bindingByObligation.get(obligation.id);

    if (!binding) {
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
    if (binding.statementHashAtBinding !== obligation.statement_hash) {
      findings.push({
        kind: "suspect-link",
        obligation: obligation.id,
        // The highest-value single finding in the traceability design: the
        // requirement moved and the evidence did not follow, while the matrix
        // still reads as covered.
        severity: "high",
        summary:
          `${obligation.id} was reworded after its evidence was bound ` +
          `(bound against ${binding.statementHashAtBinding.slice(0, 12)}…, ` +
          `now ${obligation.statement_hash.slice(0, 12)}…). Re-affirm or re-verify.`,
      });
      continue;
    }

    const run = runsBySuite.get(binding.suite);

    // ── 2. Stale evidence ──
    if (!run) {
      findings.push({
        kind: "stale-evidence",
        obligation: obligation.id,
        severity: "high",
        summary:
          `${obligation.id} is bound to suite ${binding.suite}, which has no ` +
          `recorded run. The binding claims evidence that is not in the store.`,
      });
      continue;
    }
    if (input.headCommit && run.commit !== input.headCommit) {
      findings.push({
        kind: "stale-evidence",
        obligation: obligation.id,
        // Medium: an older run is normal between releases. It becomes
        // actionable at a gate, which is the consuming workflow's policy.
        severity: "medium",
        summary:
          `${obligation.id} rests on a ${binding.suite} run at ` +
          `${run.commit.slice(0, 12)}, not at HEAD (${input.headCommit.slice(0, 12)}).`,
      });
    }

    // ── 3. Vacuous evidence ──
    const vacuous = binding.symbols.filter((symbol) => {
      const entry = run.entries.find((e) => e.symbol === symbol);
      // A symbol the run never reported is vacuous in the strongest sense: the
      // binding names evidence the suite did not produce.
      if (!entry) return true;
      return entry.outcome === "skip";
    });
    if (vacuous.length > 0 && vacuous.length === binding.symbols.length) {
      findings.push({
        kind: "vacuous-evidence",
        obligation: obligation.id,
        severity: "high",
        summary:
          `every symbol bound to ${obligation.id} was skipped or absent from ` +
          `the ${binding.suite} run: ${vacuous.sort().join(", ")}. The row reads ` +
          `as covered and nothing was verified.`,
      });
      continue;
    }

    // ── Method conformance ──
    const conformance = methodConformance(
      obligation,
      binding,
      run,
      input.catalog,
    );
    if (conformance) {
      findings.push(conformance);
      continue;
    }

    // ── Multiplicity ──
    const multiplicity = multiplicityFinding(obligation, binding, input);
    if (multiplicity) {
      findings.push(multiplicity);
      continue;
    }

    healthy.push(obligation.id);
  }

  findings.sort(
    (a, b) =>
      a.obligation.localeCompare(b.obligation) || a.kind.localeCompare(b.kind),
  );
  return { findings, healthy: healthy.sort() };
}

/**
 * An obligation whose declared method is `Analysis` is not discharged by a unit
 * test.
 *
 * Checked through the catalog rather than by name matching: the authored cell
 * carries a method or a class, and only the catalog knows which class a method
 * belongs to. Without a catalog the check is skipped — an absent catalog means
 * the question cannot be asked, which is different from the answer being yes.
 */
function methodConformance(
  obligation: Obligation,
  binding: Binding,
  run: RunRecord,
  catalog: MethodCatalog | undefined,
): Finding | null {
  if (!catalog || !obligation.method) return null;
  const declared = obligation.method.trim().toLowerCase();

  const declaredClasses = new Set<string>();
  for (const method of catalog.methods) {
    if (method.id.toLowerCase() === declared) declaredClasses.add(method.class);
    if (method.class.toLowerCase() === declared)
      declaredClasses.add(method.class);
  }
  if (declaredClasses.size === 0) return null;

  // The evidence kind the run actually produced, via the suite's tool is not
  // knowable here — what IS knowable is the class the declared method belongs
  // to. A non-Test class discharged by a test-shaped run is the conformance
  // failure worth reporting; the reverse (a Test method discharged by a
  // recorded inspection) is caught by the same comparison.
  const isTestClass = declaredClasses.has("Test");
  const ranAsTest = run.entries.length > 0;

  if (!isTestClass && ranAsTest) {
    return {
      kind: "method-conformance",
      obligation: obligation.id,
      severity: "medium",
      summary:
        `${obligation.id} declares \`${obligation.method}\` ` +
        `(${[...declaredClasses].sort().join("/")}) but is discharged by a test ` +
        `run in ${binding.suite}. A method that is not a test is not discharged ` +
        `by one.`,
    };
  }
  return null;
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
  binding: Binding,
  input: AuditInput,
): Finding | null {
  const demanding = input.multiplicityRequires ?? [];
  if (demanding.length === 0 || !obligation.criticality) return null;
  if (!demanding.includes(obligation.criticality)) return null;

  const suites = new Set(
    input.bindings
      .filter((b) => b.obligation === obligation.id)
      .map((b) => b.suite),
  );
  if (suites.size >= 2) return null;

  return {
    kind: "insufficient-multiplicity",
    obligation: obligation.id,
    severity: "medium",
    summary:
      `${obligation.id} is criticality ${obligation.criticality}, which requires ` +
      `two independent methods, but all its evidence comes from ${binding.suite}.`,
  };
}

/** Findings not present in the baseline — what a ratchet fails on. */
export function ratchet(
  report: AuditReport,
  baseline: { undischarged: string[]; suspect: string[] },
): Finding[] {
  const accepted = new Set([
    ...baseline.undischarged.map((id) => `undischarged:${id}`),
    ...baseline.suspect.map((id) => `suspect-link:${id}`),
  ]);
  // Ratchet mode exists because a gate that fails on the whole existing
  // backlog gets disabled within a week. Only NEW violations fail.
  return report.findings.filter(
    (f) => !accepted.has(`${f.kind}:${f.obligation}`),
  );
}

/** What changed between two audits — the per-PR delta. */
export function delta(
  before: AuditReport,
  after: AuditReport,
): { added: Finding[]; resolved: Finding[] } {
  const key = (f: Finding) => `${f.kind}:${f.obligation}`;
  const beforeKeys = new Set(before.findings.map(key));
  const afterKeys = new Set(after.findings.map(key));
  return {
    added: after.findings.filter((f) => !beforeKeys.has(key(f))),
    resolved: before.findings.filter((f) => !afterKeys.has(key(f))),
  };
}
