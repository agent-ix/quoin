/**
 * Document-state assignment (FR-086).
 *
 * Every enumerated document gets exactly one of four states. The states are
 * asserted exhaustive and mutually exclusive, and their sum is asserted equal
 * to the FR-084 document count — because the failure this guards against is a
 * document falling between two states and quietly leaving the denominator. A
 * rate whose denominator shrank is not a better rate.
 *
 * `out-of-model` keeps its two reasons apart. "This document declares no type"
 * and "this document declares a type no module knows" look the same in a total
 * and mean opposite things: the first is a document outside the model, the
 * second is very often a module gap.
 */

import { readFileSync } from "node:fs";

export type DocumentState =
  | "measured"
  | "out-of-model"
  | "unreadable"
  | "contested";

export type OutOfModelReason =
  | "no-declared-type"
  | "type-not-declared-by-any-module";

export interface DocumentAssignment {
  readonly path: string;
  readonly state: DocumentState;
  readonly declaredType: string | null;
  /** Set only when `state` is `out-of-model`; the two reasons never merge. */
  readonly reason?: OutOfModelReason;
  /** Set when `state` is `unreadable`. */
  readonly failure?: string;
  /** The modules declaring the type; more than one is `contested`. */
  readonly modules?: readonly string[];
}

export interface ContractDefectFinding {
  readonly kind: "contract-defect";
  readonly detail: string;
}

/** Frontmatter `type:`, or null. Case-sensitive by contract. */
export function declaredType(text: string): string | null {
  if (!text.startsWith("---")) return null;
  const end = text.indexOf("\n---", 3);
  if (end < 0) throw new Error("unterminated frontmatter fence");
  const front = text.slice(3, end);
  const m = /^type:\s*["']?([A-Za-z0-9_.-]+)["']?\s*$/m.exec(front);
  return m ? m[1] : null;
}

/**
 * Builds the type vocabulary keyed on the pair (module, type). A type two
 * resolved modules both declare is contested — it is not resolvable to one
 * contract, and picking either silently would attribute every document of that
 * type to a module that may not own it.
 */
export function buildVocabulary(
  modules: readonly {
    name: string;
    objectTypes: readonly string[];
    artifactTypes: readonly string[];
  }[],
): Map<string, string[]> {
  const byType = new Map<string, string[]>();
  for (const m of modules) {
    for (const t of [...m.objectTypes, ...m.artifactTypes]) {
      const owners = byType.get(t) ?? [];
      if (!owners.includes(m.name)) owners.push(m.name);
      byType.set(t, owners);
    }
  }
  return byType;
}

export function assignDocuments(
  paths: readonly string[],
  vocabulary: Map<string, string[]>,
): {
  assignments: DocumentAssignment[];
  findings: ContractDefectFinding[];
} {
  const assignments: DocumentAssignment[] = [];
  const findings: ContractDefectFinding[] = [];
  const contestedReported = new Set<string>();

  for (const path of paths) {
    let text: string;
    try {
      text = readFileSync(path, "utf8");
    } catch (error) {
      // Keep enumerating siblings: one unreadable file must not truncate the
      // population and silently shrink every denominator after it.
      assignments.push({
        path,
        state: "unreadable",
        declaredType: null,
        failure: String((error as Error).message),
      });
      continue;
    }

    let type: string | null;
    try {
      type = declaredType(text);
    } catch (error) {
      assignments.push({
        path,
        state: "unreadable",
        declaredType: null,
        failure: String((error as Error).message),
      });
      continue;
    }

    if (type === null) {
      assignments.push({
        path,
        state: "out-of-model",
        declaredType: null,
        reason: "no-declared-type",
      });
      continue;
    }

    const owners = vocabulary.get(type);
    if (!owners || owners.length === 0) {
      assignments.push({
        path,
        state: "out-of-model",
        declaredType: type,
        reason: "type-not-declared-by-any-module",
      });
      continue;
    }

    if (owners.length > 1) {
      if (!contestedReported.has(type)) {
        contestedReported.add(type);
        findings.push({
          kind: "contract-defect",
          detail: `type "${type}" is declared by ${owners.join(" and ")}; it resolves to no single contract`,
        });
      }
      assignments.push({
        path,
        state: "contested",
        declaredType: type,
        modules: owners,
      });
      continue;
    }

    assignments.push({
      path,
      state: "measured",
      declaredType: type,
      modules: owners,
    });
  }

  return { assignments, findings };
}

/**
 * Asserts the partition is total. Throws rather than returning a flag: a census
 * that has lost documents must not be able to publish a rate.
 */
export function assertPartition(
  assignments: readonly DocumentAssignment[],
  enumeratedCount: number,
): Record<DocumentState, number> {
  const tally: Record<DocumentState, number> = {
    measured: 0,
    "out-of-model": 0,
    unreadable: 0,
    contested: 0,
  };
  for (const a of assignments) tally[a.state] += 1;
  const sum = Object.values(tally).reduce((n, x) => n + x, 0);
  if (sum !== enumeratedCount) {
    throw new Error(
      `document states sum to ${sum} but enumeration counted ${enumeratedCount}; ` +
        `${enumeratedCount - sum} document(s) fell between states and would have left the denominator`,
    );
  }
  return tally;
}
