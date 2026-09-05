/**
 * Rates, the divergence list and the figure index (FR-091).
 *
 * Three rules decide whether a published rate can be trusted, and all three
 * are mechanical here rather than editorial:
 *
 *   - **A rate carries its unit, its population identifier and its method.**
 *     A bare percentage is not a measurement; it is a number that survived
 *     being copied.
 *   - **A zero-denominator partition is published with no rate value, not
 *     omitted.** Dropping empty partitions is how a breakdown comes to
 *     describe only the parts that had something to say.
 *   - **Every printed figure is bound to the artifact and field it came from,
 *     and recomputed from it.** Three headline figures in this program were
 *     wrong because prose drifted from the artifact it described.
 */

import { createHash } from "node:crypto";

export interface Rate {
  readonly id: string;
  readonly numerator: number;
  readonly denominator: number;
  readonly unit: string;
  readonly populationId: string;
  readonly methodId: string;
  /** null when the denominator is zero: no rate, rather than a rate of zero. */
  readonly value: number | null;
  /** Counts that left both sides, published beside the rate. */
  readonly excluded: Readonly<Record<string, number>>;
}

export interface Figure {
  /** The literal text printed in the prose report. */
  readonly printed: string;
  /** The artifact the figure came from. */
  readonly artifact: string;
  /** The field within that artifact. */
  readonly field: string;
}

export class RateError extends Error {}

/**
 * The population identifier binds a rate to exactly what was measured: the
 * corpus id, every repository commit, every module commit and the engine
 * revision. Two rates with the same identifier are comparable; two without it
 * are two numbers.
 */
export function populationId(input: {
  corpusId: string;
  repositoryCommits: readonly string[];
  moduleCommits: readonly string[];
  engineRevision: string | null;
}): string {
  const material = JSON.stringify({
    corpusId: input.corpusId,
    repositories: [...input.repositoryCommits].sort(),
    modules: [...input.moduleCommits].sort(),
    engine: input.engineRevision ?? "unknown",
  });
  return `pop:${createHash("sha256").update(material).digest("hex").slice(0, 16)}`;
}

export function rate(input: {
  id: string;
  numerator: number;
  denominator: number;
  unit: string;
  populationId: string;
  methodId: string;
  excluded?: Readonly<Record<string, number>>;
}): Rate {
  if (input.numerator > input.denominator) {
    throw new RateError(
      `rate ${input.id} has numerator ${input.numerator} above denominator ${input.denominator}`,
    );
  }
  return {
    ...input,
    excluded: input.excluded ?? {},
    value: input.denominator === 0 ? null : input.numerator / input.denominator,
  };
}

export interface Divergence {
  readonly partition: string;
  readonly value: number;
  readonly aggregate: number;
  readonly margin: number;
}

/**
 * Names every partition more than `margin` below the aggregate. The margin is
 * a parameter and is read on every call, so a test can change it and observe
 * the list change — a threshold nothing can move is a threshold nobody has
 * checked.
 */
export function divergences(
  aggregate: Rate,
  partitions: readonly Rate[],
  margin: number,
): Divergence[] {
  if (aggregate.value === null) return [];
  const out: Divergence[] = [];
  for (const p of partitions) {
    if (p.value === null) continue;
    if (aggregate.value - p.value > margin) {
      out.push({
        partition: p.id,
        value: p.value,
        aggregate: aggregate.value,
        margin,
      });
    }
  }
  return out;
}

/**
 * Recomputes every printed figure from the artifact and field it names.
 *
 * `lookup` resolves (artifact, field) to the current value. A figure whose
 * printed text no longer matches is reported rather than corrected: silently
 * rewriting prose to match data hides that the two had drifted.
 */
export function verifyFigures(
  figures: readonly Figure[],
  lookup: (artifact: string, field: string) => string | undefined,
): { verified: number; mismatched: Figure[]; unresolved: Figure[] } {
  const mismatched: Figure[] = [];
  const unresolved: Figure[] = [];
  let verified = 0;
  for (const figure of figures) {
    const actual = lookup(figure.artifact, figure.field);
    if (actual === undefined) {
      unresolved.push(figure);
      continue;
    }
    if (actual !== figure.printed) {
      mismatched.push(figure);
      continue;
    }
    verified += 1;
  }
  return { verified, mismatched, unresolved };
}

/**
 * The structural rate and the form census are returned as separate records and
 * never summed: they count different populations by different methods, and a
 * combined figure would describe neither.
 */
export interface Report {
  readonly populationId: string;
  readonly structural: Rate;
  readonly formCensus: Rate;
  readonly byModule: readonly Rate[];
  readonly byType: readonly Rate[];
  readonly byRepository: readonly Rate[];
  readonly divergences: readonly Divergence[];
  /** Published beside every corpus-level rate. */
  readonly unstableRepositories: number;
  readonly dirtyRepositories: number;
  readonly figures: readonly Figure[];
}

export function assertNotSummed(report: Report): void {
  if (report.structural.methodId === report.formCensus.methodId) {
    throw new RateError(
      "the structural rate and the form census declare the same method; " +
        "they count different populations and must not be reported as one figure",
    );
  }
}
