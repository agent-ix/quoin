/**
 * The finding partition (FR-090).
 *
 * Every finding takes exactly one of eight classes, with a named owner and a
 * disposition. Three properties are load-bearing:
 *
 *   - **Identity excludes the line number.** A finding is identified by
 *     (repository, path, check, code). Including the line would let a moved
 *     line silently lose its classification and reappear as `unknown`, which
 *     reads as new work rather than the same work relocated.
 *   - **`unknown` and `undispositioned` are headline figures.** They are the
 *     honest measure of how far the partition got. Burying them in a total is
 *     how a partition reports completeness it does not have.
 *   - **A ledger entry matching nothing is reported.** An entry covering no
 *     finding is either fixed, mis-scoped, or was never real. All three are
 *     worth knowing, and none is visible from the classified side.
 */

import { classifyFailure, type LedgerEntry } from "./tool-defects.js";

export type FindingClass =
  | "contract-defect"
  | "legitimate-undeclared-value"
  | "malformed-document"
  | "missing-structure"
  | "unsupported-representation"
  | "stale-module"
  | "tool-defect"
  | "unknown";

export interface Finding {
  readonly repository: string;
  readonly path: string;
  readonly check: string;
  readonly code: string | null;
  readonly message: string;
}

export interface Disposition {
  /** A person or a repository. A bare role is refused. */
  readonly owner: string;
  readonly kind:
    | "fix-in-this-campaign"
    | "deferred-to-later-campaign"
    | "accepted"
    | "undispositioned";
  /** Required by kind: the campaign, the module repository, or the human. */
  readonly nomination?: string;
}

export interface Classified {
  readonly identity: string;
  readonly finding: Finding;
  readonly classification: FindingClass;
  readonly disposition: Disposition;
  readonly citation?: string;
}

export class PartitionError extends Error {}

const BARE_ROLES = new Set([
  "owner",
  "maintainer",
  "the team",
  "team",
  "someone",
  "tbd",
  "unassigned",
]);

/** (repository, path, check, code). Never the line. */
export function identity(f: Finding): string {
  return [f.repository, f.path, f.check, f.code ?? "-"].join(" ");
}

export function assertOwner(d: Disposition, identityKey: string): void {
  const owner = d.owner.trim();
  if (owner === "") {
    throw new PartitionError(
      `finding ${identityKey} has no owner; an unowned finding is not dispositioned, it is deferred silently`,
    );
  }
  if (BARE_ROLES.has(owner.toLowerCase())) {
    throw new PartitionError(
      `finding ${identityKey} names the bare role "${owner}"; a role cannot answer a question, a person or a repository can`,
    );
  }
}

export function assertNomination(d: Disposition, identityKey: string): void {
  const needs: Record<Disposition["kind"], string | null> = {
    "fix-in-this-campaign": null,
    "deferred-to-later-campaign": "the campaign it is deferred to",
    accepted: "the human accepting it",
    undispositioned: null,
  };
  const required = needs[d.kind];
  if (required && !d.nomination?.trim()) {
    throw new PartitionError(
      `finding ${identityKey} is "${d.kind}" but does not name ${required}`,
    );
  }
}

export function partition(options: {
  findings: readonly Finding[];
  ledger: readonly LedgerEntry[];
  /** Classifications the operator has authored, keyed by identity. */
  authored?: ReadonlyMap<
    string,
    { classification: FindingClass; disposition: Disposition }
  >;
}): {
  classified: Classified[];
  tally: Record<FindingClass, number>;
  unknown: number;
  undispositioned: number;
  unmatchedLedgerEntries: string[];
} {
  const { findings, ledger, authored = new Map() } = options;
  const classified: Classified[] = [];
  const matchedEntries = new Set<string>();

  for (const finding of findings) {
    const key = identity(finding);

    const tool = classifyFailure(ledger, finding.path, finding.check);
    if (tool) {
      matchedEntries.add(tool.entryId);
      const disposition: Disposition = {
        owner: tool.citation,
        kind: "deferred-to-later-campaign",
        nomination: tool.citation,
      };
      assertOwner(disposition, key);
      assertNomination(disposition, key);
      classified.push({
        identity: key,
        finding,
        classification: "tool-defect",
        disposition,
        citation: tool.citation,
      });
      continue;
    }

    const hand = authored.get(key);
    if (hand) {
      assertOwner(hand.disposition, key);
      assertNomination(hand.disposition, key);
      classified.push({
        identity: key,
        finding,
        classification: hand.classification,
        disposition: hand.disposition,
      });
      continue;
    }

    // Not covered by the ledger and not authored. `unknown` is the truthful
    // answer, and it is published rather than absorbed into a neighbour.
    classified.push({
      identity: key,
      finding,
      classification: "unknown",
      disposition: {
        owner: "unassigned-pending-review",
        kind: "undispositioned",
      },
    });
  }

  const tally: Record<FindingClass, number> = {
    "contract-defect": 0,
    "legitimate-undeclared-value": 0,
    "malformed-document": 0,
    "missing-structure": 0,
    "unsupported-representation": 0,
    "stale-module": 0,
    "tool-defect": 0,
    unknown: 0,
  };
  for (const c of classified) tally[c.classification] += 1;

  const sum = Object.values(tally).reduce((n, x) => n + x, 0);
  if (sum !== findings.length) {
    throw new PartitionError(
      `classes sum to ${sum} but there are ${findings.length} findings; ` +
        `${findings.length - sum} finding(s) were dropped by classification`,
    );
  }

  return {
    classified,
    tally,
    unknown: tally.unknown,
    undispositioned: classified.filter(
      (c) => c.disposition.kind === "undispositioned",
    ).length,
    unmatchedLedgerEntries: ledger
      .map((e) => e.id)
      .filter((id) => !matchedEntries.has(id)),
  };
}
