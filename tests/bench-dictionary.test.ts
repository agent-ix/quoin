/**
 * The quality benchmark's metric dictionary (agent-ix/quoin#198, FR-043-AC-1..AC-6).
 *
 * FR-043 declared ten acceptance criteria and shipped with none of them carrying
 * a tagged test, which SR-015 FND-002 recorded. Five of them are dictionary
 * criteria — each says the dictionary *defines* or *declares* something — and
 * this is those five plus the sentinel's declaration.
 */

import { readFileSync } from "node:fs";
import { join } from "node:path";

import {
  DictionaryError,
  loadMetrics,
  validateDictionary,
} from "../evals/lib/dictionary.mjs";

const DICTIONARY = join(__dirname, "..", "bench", "metrics.json");

describe("the metric dictionary", () => {
  const { families, metrics } = loadMetrics(DICTIONARY);

  it("TC-926 declares unit, population and method for every metric, and refuses one that does not", () => {
    // TC-926
    for (const [name, spec] of Object.entries(metrics)) {
      for (const field of ["unit", "population", "method", "direction"]) {
        expect(
          (spec as Record<string, string>)[field],
          `${name} declares no ${field}`,
        ).toBeTruthy();
      }
    }

    // Rejected at LOAD, not reported with a gap: a benchmark that silently
    // drops a malformed metric scores a smaller thing than it claims to.
    expect(() =>
      validateDictionary({
        families: ["f"],
        metrics: { bare: { unit: "thing", direction: "higher-is-better" } },
      }),
    ).toThrow(DictionaryError);
    expect(() =>
      validateDictionary({
        families: ["f"],
        metrics: {
          bad: {
            unit: "u",
            population: "p",
            method: "m",
            direction: "sideways",
          },
        },
      }),
    ).toThrow(/direction/);
  });

  it("TC-927 defines precision and recall per defect family, keyed on families the corpora label", () => {
    // TC-927
    for (const name of ["finding_precision", "finding_recall"]) {
      expect(metrics[name].per_family).toBe(true);
      expect(metrics[name].direction).toBe("higher-is-better");
    }
    expect(families.length).toBeGreaterThan(0);
    // Every family the answer key adjudicates is one the dictionary keys on,
    // so a score cannot be reported over an unlabelled population.
    for (const family of [
      "marker-form-mismatch",
      "vacuous-under-guard",
      "mocked-confirmation",
    ]) {
      expect(families).toContain(family);
    }
    // A per-family metric over no labelled families is refused.
    expect(() =>
      validateDictionary({
        families: [],
        metrics: {
          p: {
            unit: "u",
            population: "p",
            method: "m",
            direction: "higher-is-better",
            per_family: true,
          },
        },
      }),
    ).toThrow(/labels no families/);
  });

  it("TC-928 defines span_grounding_rate with the pass-2 figure as its baseline", () => {
    // TC-928
    const m = metrics.span_grounding_rate;
    expect(m.unit).toMatch(/specific-shape/);
    // 0 of 65 measured at pass 2. Zero is the measured starting point, and it
    // must be the number 0 rather than null — null would say "never measured".
    expect(m.baseline).toBe(0);
    expect(m.baseline_note).toMatch(/0 of 65/);
  });

  it("TC-1094 defines correctness and safe refusal independently of presence", () => {
    const correctness = metrics.span_correctness_rate;
    const refusal = metrics.span_safe_refusal_rate;
    expect(correctness.population).toMatch(/expected.*loci/i);
    expect(correctness.method).toMatch(/Presence without exact equality/);
    expect(correctness.per_family).toBe(true);
    expect(refusal.population).toMatch(/unsupported/);
    expect(refusal.method).toMatch(/expected structured refusal signal/);
    expect(refusal.per_family).toBe(true);
    expect(correctness.measurement_plan).not.toBe(refusal.measurement_plan);
  });

  it("TC-1120 gives labeled span v2 its own active MeasurementPlan", () => {
    const historical = metrics.span_grounding_rate;
    const labeled = metrics.span_grounding_v2_rate;
    expect(historical.measurement_plan).toMatch(/MP-204/);
    expect(labeled.measurement_plan).toMatch(/MP-215/);
    expect(labeled.measurement_plan).not.toBe(historical.measurement_plan);
    const plan = readFileSync(
      join(__dirname, "..", labeled.measurement_plan),
      "utf8",
    );
    expect(plan).toMatch(/status: active/);
    expect(plan).toMatch(/metric: span_grounding_v2_rate/);
    expect(plan).toMatch(/definition_version: property\.span-grounding-v2/);
  });

  it("TC-929 defines actionability_rate with the 15-of-496 baseline", () => {
    // TC-929
    const m = metrics.actionability_rate;
    expect(m.baseline).toBeCloseTo(3.02, 2);
    expect(m.baseline_note).toMatch(/15 of 496/);
    expect(m.method).toMatch(/row id/);
  });

  it("TC-930 defines cost per confirmed insight in tokens AND tool calls", () => {
    // TC-930
    const m = metrics.cost_per_confirmed_insight;
    expect(m.unit).toMatch(/tokens/);
    expect(m.unit).toMatch(/tool calls/);
    // Denominator is CONFIRMED findings; dividing by emitted output would
    // reward a run for producing more of it.
    expect(m.population).toMatch(/CONFIRMED|confirmed/);
    expect(m.direction).toBe("lower-is-better");
  });

  it("TC-931 declares the silent-zero sentinel as a gate with no tolerance", () => {
    // TC-931
    const m = metrics["sentinel.silent_zero"];
    expect(m.direction).toBe("gate-zero");
    expect(m.expected).toBe(0);
    expect(m.tolerance).toBe(0);
    // Count-shaped metrics are exempt — the CR-098 coupling with
    // agent-ix/quire-rs#229. Without it the gate fires on an honest zero.
    expect(m.method).toMatch(/count-shaped/);

    // A gate that carries tolerance is a score wearing a gate's name.
    expect(() =>
      validateDictionary({
        families: ["f"],
        metrics: {
          g: {
            unit: "u",
            population: "p",
            method: "m",
            direction: "gate-zero",
            expected: 0,
            tolerance: 1,
          },
        },
      }),
    ).toThrow(/tolerance/);
  });
});
