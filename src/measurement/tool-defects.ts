/**
 * The tool-defect ledger (FR-089).
 *
 * A published rate is only honest if the reader can tell which part of it the
 * tools could not measure. Each entry cites a repository and an issue number,
 * declares the scope it covers and the effect it has, and the share of the
 * population it covers is published beside the aggregate rate — not folded
 * into it.
 *
 * Two rules keep the ledger from becoming an excuse:
 *
 *   - An entry with no citation is refused, by name. "The tool is flaky" is
 *     not a defect, it is a way of not investigating one.
 *   - A failure not covered by a declared entry is never classified
 *     `tool-defect`. The ledger explains failures it predicted; it does not
 *     absorb ones it did not.
 */

export type DefectEffect =
  | "rows-unbindable"
  | "documents-unmeasurable"
  | "attribution-unproven"
  | "check-never-runs";

export interface LedgerEntry {
  readonly id: string;
  readonly repository: string;
  readonly issue: number;
  /** What the defect does to the measurement, not what it does in the tool. */
  readonly effect: DefectEffect;
  /** Human-readable statement of which population the entry covers. */
  readonly scope: string;
  /** Predicate deciding whether a path falls inside the declared scope. */
  readonly covers: (path: string) => boolean;
  /** Checks this entry blocks; a covered document is could-not-run for these. */
  readonly blocks: readonly string[];
  readonly summary: string;
}

export class LedgerError extends Error {}

/** Refuses an entry that cites nothing, naming it. */
export function assertCited(entries: readonly LedgerEntry[]): void {
  for (const e of entries) {
    if (!e.repository || !Number.isInteger(e.issue) || e.issue <= 0) {
      throw new LedgerError(
        `ledger entry "${e.id}" cites no repository and issue number; ` +
          `an uncited entry is a way of not investigating a defect, not a record of one`,
      );
    }
  }
}

export interface Coverage {
  readonly entryId: string;
  readonly documents: number;
  readonly share: number;
}

/** The share of the population each entry covers, published beside the rate. */
export function coverage(
  entries: readonly LedgerEntry[],
  population: readonly string[],
): Coverage[] {
  assertCited(entries);
  const total = population.length;
  return entries.map((e) => {
    const documents = population.filter((p) => e.covers(p)).length;
    return {
      entryId: e.id,
      documents,
      share: total === 0 ? 0 : documents / total,
    };
  });
}

/**
 * Classifies a failure. Returns `tool-defect` with the citation only when a
 * declared entry covers both the path and the check; otherwise returns null so
 * the caller partitions it on its merits.
 */
export function classifyFailure(
  entries: readonly LedgerEntry[],
  path: string,
  check: string,
): { classification: "tool-defect"; citation: string; entryId: string } | null {
  assertCited(entries);
  for (const e of entries) {
    if (e.covers(path) && e.blocks.includes(check)) {
      return {
        classification: "tool-defect",
        citation: `${e.repository}#${e.issue}`,
        entryId: e.id,
      };
    }
  }
  return null;
}

/** The declared ledger for this measurement. */
export const LEDGER: readonly LedgerEntry[] = [
  {
    id: "range-ids-unresolvable",
    repository: "agent-ix/quire-rs",
    issue: 402,
    effect: "rows-unbindable",
    scope: "any matrix whose Traces To cell uses an A..B range",
    covers: (p) => p.endsWith("tests.md"),
    blocks: ["trace-resolution"],
    summary:
      "`quire coverage` does not expand `A..B` ranges, so an id appearing only inside a range resolves to no declared row. The row exists; the engine cannot see it.",
  },
  {
    id: "regex-brace-unbinds-file",
    repository: "agent-ix/quire-rs",
    issue: 403,
    effect: "rows-unbindable",
    scope: "any TypeScript test file containing a brace inside a regex literal",
    covers: (p) => p.endsWith(".test.ts"),
    blocks: ["trace-binding"],
    summary:
      "A brace inside a regex literal makes the whole file unreadable to the binder, so its tags mint nothing and its rows report as unbacked. Unreadable is not untested, and the two are indistinguishable in a total.",
  },
  {
    id: "status-lie-check-never-ran",
    repository: "agent-ix/spec-artifacts-process",
    issue: 81,
    effect: "check-never-runs",
    scope: "FR, StR and US coverage tables in every repository",
    covers: (p) => p.endsWith("tests.md"),
    blocks: ["status-lie"],
    summary:
      "`traceability.status.column` names `Status` while the coverage tables assert `Coverage Status`, so the complete-but-unbacked check has never run. Any existing 'no status lies' claim is unmeasured, not clean.",
  },
  {
    id: "artifact-module-install-refused",
    repository: "agent-ix/quoin",
    issue: 347,
    effect: "documents-unmeasurable",
    scope: "install-path verification for artifact-type semantic modules",
    covers: () => false,
    blocks: ["module-install"],
    summary:
      "Quoin cannot install any artifact-type semantic module, so the install path cannot be exercised for them.",
  },
  {
    id: "pinned-module-set-not-closed",
    repository: "agent-ix/quire-rs",
    issue: 405,
    effect: "attribution-unproven",
    scope: "every document in this measurement",
    covers: () => true,
    blocks: ["contract-attribution"],
    summary:
      "IX_FILAMENT_MODULES_PATH adds to the default install root instead of replacing it, and resolution is first-wins. Every rate this run publishes is therefore not provably attributable to the pinned contract revisions, even though the verdicts themselves stand.",
  },
];
