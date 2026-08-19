/**
 * t-way coverage over a declared configuration space (FR-035).
 *
 * quire mints the obligation — *"2-way over these dimensions"* — and states the
 * space in the obligation's own statement (quire-rs FR-061). This computes what
 * a run actually reached, and names what it did not.
 *
 * **quoin computes; it neither declares nor generates.** The dimensions come
 * from the spec, the executed configurations come from the consumer's CI, and
 * the covering array itself is nobody's job here (ADR-0011 invariant 1).
 */

/** One dimension parsed back out of an obligation statement. */
export interface Dimension {
  name: string;
  values: string[];
}

/** A forbidden combination: assignments that cannot co-occur. */
export interface Exclusion {
  assignments: Array<[string, string]>;
}

export interface ConfigurationSpace {
  strength: number;
  dimensions: Dimension[];
  exclusions: Exclusion[];
}

const HEADER = /^(\d+)-way over\s/;
const DIMENSION = /([^\s(]+)\(([^)]*)\)/g;
const EXCLUSION = /excluding\[([^\]]*)\]/g;

/**
 * Parse the space out of a combinatorial obligation's statement.
 *
 * Returns `null` for any statement that is not one, which is how a caller tells
 * a combinatorial obligation from every other kind without a second flag to
 * keep in agreement with the first.
 */
export function parseSpace(statement: string): ConfigurationSpace | null {
  const header = HEADER.exec(statement);
  if (!header) return null;
  const strength = Number.parseInt(header[1], 10);
  if (!Number.isFinite(strength) || strength < 1) return null;

  // Dimensions are read from the part before any `excluding[...]`, so an
  // exclusion naming `dim=value` is never mistaken for a dimension.
  const head = statement.split("excluding[")[0];
  const dimensions: Dimension[] = [];
  for (const match of head.matchAll(DIMENSION)) {
    const values = match[2]
      .split("|")
      .map((v) => v.trim())
      .filter((v) => v !== "");
    if (values.length < 2) continue;
    dimensions.push({ name: match[1], values });
  }
  if (dimensions.length < 2) return null;

  const exclusions: Exclusion[] = [];
  for (const match of statement.matchAll(EXCLUSION)) {
    const assignments = match[1]
      .split(",")
      .map((clause) => clause.trim())
      .filter((clause) => clause !== "")
      .map((clause) => {
        const at = clause.indexOf("=");
        return at === -1
          ? null
          : ([clause.slice(0, at).trim(), clause.slice(at + 1).trim()] as [
              string,
              string,
            ]);
      })
      .filter((a): a is [string, string] => a !== null);
    if (assignments.length >= 2) exclusions.push({ assignments });
  }
  return { strength, dimensions, exclusions };
}

/** Every `size`-element subset of `0..n`, lexicographic. */
function combinations(n: number, size: number): number[][] {
  if (size > n || size < 1) return [];
  const out: number[][] = [];
  const current: number[] = [];
  const walk = (start: number): void => {
    if (current.length === size) {
      out.push([...current]);
      return;
    }
    for (let i = start; i < n; i += 1) {
      current.push(i);
      walk(i + 1);
      current.pop();
    }
  };
  walk(0);
  return out;
}

/** A stable key for one t-way tuple, so covered and demanded compare directly. */
function tupleKey(pairs: Array<[string, string]>): string {
  return [...pairs]
    .sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0))
    .map(([d, v]) => `${d}=${v}`)
    .join(" & ");
}

function forbidden(
  space: ConfigurationSpace,
  pairs: Array<[string, string]>,
): boolean {
  return space.exclusions.some((exclusion) =>
    exclusion.assignments.every(([d, v]) =>
      pairs.some(([pd, pv]) => pd === d && pv === v),
    ),
  );
}

/** Every t-way tuple the space demands, as stable keys. */
export function demandedTuples(space: ConfigurationSpace): Set<string> {
  const out = new Set<string>();
  for (const dims of combinations(space.dimensions.length, space.strength)) {
    const walk = (slot: number, acc: Array<[string, string]>): void => {
      if (slot === dims.length) {
        if (!forbidden(space, acc)) out.add(tupleKey(acc));
        return;
      }
      const dimension = space.dimensions[dims[slot]];
      for (const value of dimension.values) {
        acc.push([dimension.name, value]);
        walk(slot + 1, acc);
        acc.pop();
      }
    };
    walk(0, []);
  }
  return out;
}

/** Every t-way tuple one executed configuration covers. */
export function coveredBy(
  space: ConfigurationSpace,
  config: Record<string, string>,
): Set<string> {
  const present = space.dimensions.filter(
    (d) => config[d.name] !== undefined && d.values.includes(config[d.name]),
  );
  const out = new Set<string>();
  for (const dims of combinations(present.length, space.strength)) {
    out.add(
      tupleKey(dims.map((i) => [present[i].name, config[present[i].name]])),
    );
  }
  return out;
}

export interface CoverageResult {
  strength: number;
  demanded: number;
  covered: number;
  /** Tuples no executed configuration reached, in stable order. */
  gaps: string[];
}

/**
 * What a set of executed configurations reached, and what it did not.
 *
 * The gap list is the point. A percentage tells someone how much is missing;
 * the list tells them *which combinations to run*, which is the difference
 * between a number and an action.
 */
export function twayCoverage(
  space: ConfigurationSpace,
  configs: Array<Record<string, string>>,
): CoverageResult {
  const demanded = demandedTuples(space);
  const covered = new Set<string>();
  for (const config of configs) {
    for (const key of coveredBy(space, config)) {
      // Only tuples the space actually demands count. A run exercising a value
      // the spec never declared covers nothing it was asked for — and counting
      // it would let a coverage number rise by testing something else.
      if (demanded.has(key)) covered.add(key);
    }
  }
  const gaps = [...demanded].filter((k) => !covered.has(k)).sort();
  return {
    strength: space.strength,
    demanded: demanded.size,
    covered: covered.size,
    gaps,
  };
}
