// The quality benchmark's metric dictionary (agent-ix/quoin#198, FR-043-AC-1).
//
// The dictionary is the contract, and it is enforced at LOAD. A metric that
// does not say what one of its values is, what its denominator is drawn from,
// and how it was arrived at cannot be interrogated by the person reading the
// score — and a number nobody can interrogate is the defect this whole
// programme exists to end. So an incomplete entry is refused here rather than
// reported as a gap downstream, where it would be one warning among many.

import { readFileSync } from "node:fs";

/** Fields every metric must declare. */
const REQUIRED = ["unit", "population", "method", "direction"];

/** Directions a metric may declare. `gate-zero` never ratchets. */
const DIRECTIONS = new Set([
  "higher-is-better",
  "lower-is-better",
  "gate-zero",
]);

export class DictionaryError extends Error {}

/**
 * Read and validate the dictionary, or throw (FR-043-AC-1).
 *
 * Throws rather than returning a partial set: a benchmark that silently drops
 * a malformed metric scores a smaller thing than it claims to, which is the
 * same class as the answer-key entry that scored `missed` forever because its
 * `expect_value` was absent.
 */
export function loadMetrics(path) {
  let doc;
  try {
    doc = JSON.parse(readFileSync(path, "utf8"));
  } catch (cause) {
    throw new DictionaryError(
      `cannot read the metric dictionary at ${path}: ${cause.message}`,
    );
  }
  return validateDictionary(doc, path);
}

/** The validation half, separated so it can be exercised without a file. */
export function validateDictionary(doc, path = "<inline>") {
  const metrics = doc?.metrics;
  if (!metrics || typeof metrics !== "object" || Array.isArray(metrics)) {
    throw new DictionaryError(`${path}: no \`metrics\` object`);
  }
  const families = Array.isArray(doc.families) ? doc.families : [];

  for (const [name, spec] of Object.entries(metrics)) {
    for (const field of REQUIRED) {
      const value = spec?.[field];
      if (typeof value !== "string" || !value.trim()) {
        throw new DictionaryError(
          `${path}: metric \`${name}\` declares no ${field}. A metric that does ` +
            `not say what it counts is not one this benchmark emits.`,
        );
      }
    }
    if (!DIRECTIONS.has(spec.direction)) {
      throw new DictionaryError(
        `${path}: metric \`${name}\` declares direction \`${spec.direction}\`, ` +
          `not one of ${[...DIRECTIONS].join(", ")}`,
      );
    }
    // A gate is not a score: it carries no tolerance to spend, and the
    // expected value is exact (FR-043-AC-6).
    if (
      spec.direction === "gate-zero" &&
      (spec.expected !== 0 || spec.tolerance !== 0)
    ) {
      throw new DictionaryError(
        `${path}: gate \`${name}\` must declare expected 0 and tolerance 0; a gate ` +
          `with tolerance is a score wearing a gate's name`,
      );
    }
    // A per-family metric is keyed on families the corpora actually label, so
    // a score cannot be reported over an unlabelled population (FR-043-AC-2).
    if (spec.per_family && families.length === 0) {
      throw new DictionaryError(
        `${path}: metric \`${name}\` is per-family but the dictionary labels no families`,
      );
    }
  }
  return { families, metrics };
}
