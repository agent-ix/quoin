/**
 * The tier-1 runner (quoin#199, FR-043-AC-2/AC-9/AC-10).
 *
 * `buildBenchCorpora` had no production caller until this runner: the corpora
 * were built by a unit test, into a temp directory, and scored by nobody. These
 * tests cover the pure parts — the label flattening that made the two halves
 * able to meet, the localisation rate, and the ratchet's one-way semantics.
 * Whether a given corpus actually produces a given finding is verified by
 * running the real engine and recorded per defect in `confirmed_at`; asserting
 * it here would make the unit suite depend on a `quire` binary.
 */

import { readFileSync } from "node:fs";
import { join } from "node:path";

import {
  compare,
  flattenLabels,
  localisationRate,
  ratchet,
} from "../scripts/bench-tier1.mjs";

describe("tier-1 label flattening", () => {
  test("TC-953 the builder's wrapper becomes the flat array scoring consumes", () => {
    // TC-953
    // The shape mismatch that kept the two halves from ever meeting:
    // `buildBenchCorpora` writes `{corpora:[{name, defects}]}` and
    // `scoreFindings` consumes a flat list of labels. Nothing converted, so
    // nothing scored. The corpus name rides along, because a label that cannot
    // say which corpus it came from cannot be argued with.
    const flat = flattenLabels({
      corpora: [
        { name: "a", family: "f1", defects: [{ id: "A-1", family: "f1" }] },
        { name: "b", family: "f2", defects: [{ id: "B-1", family: "f2" }] },
        { name: "clean", family: "none", defects: [] },
      ],
    });
    expect(flat).toEqual([
      { id: "A-1", family: "f1", corpus: "a" },
      { id: "B-1", family: "f2", corpus: "b" },
    ]);
  });
});

describe("finding localisation rate", () => {
  test("TC-954 it is positional pairings over true positives, null when there are none", () => {
    // TC-954
    // The metric that encodes the actual requirement: an alert must say WHERE.
    // Two of five is the first scored run — the three that did not name a place
    // are an aggregate metric, a diagnostic that names a metric rather than a
    // file, and one that names the LANGUAGE instead of the marker's line.
    expect(
      localisationRate({
        positional: 2,
        families: [
          { truePositives: 3 },
          { truePositives: 2 },
          { truePositives: 0 },
        ],
      }),
    ).toBe(0.4);

    // `null`, never 0. A run that confirmed nothing has no localisation rate,
    // and reporting 0 would claim every finding failed to name a place when no
    // finding was confirmed at all — 0/0 is not 0%.
    expect(
      localisationRate({ positional: 0, families: [{ truePositives: 0 }] }),
    ).toBeNull();
  });
});

describe("the ratchet", () => {
  test("TC-955 a regression keeps the OLD baseline, so a bad run cannot lower the bar", () => {
    // TC-955
    // quire-rs `scripts/bench.py`'s semantics, deliberately identical. The
    // one-way property is the whole point: if a regression proposed its own
    // value as the new baseline, `--update` after a bad run would quietly
    // ratify it and the gate would track whatever the tool last did.
    expect(compare("higher-is-better", 0.9, 0.5)).toEqual(["improved", 0.9]);
    expect(compare("higher-is-better", 0.5, 0.5)).toEqual(["held", 0.5]);
    expect(compare("higher-is-better", 0.2, 0.5)).toEqual(["regressed", 0.5]);
    // Direction is honoured, not assumed.
    expect(compare("lower-is-better", 0.2, 0.5)).toEqual(["improved", 0.2]);
    expect(compare("lower-is-better", 0.9, 0.5)).toEqual(["regressed", 0.5]);
  });

  test("TC-956 a missing baseline is `new`, never a pass by default", () => {
    // TC-956
    // The first run of any metric. It must not read as a pass — nothing was
    // compared — and it must not read as a failure either. The observed value
    // is PROPOSED, and only `--update` writes it.
    expect(compare("higher-is-better", 0.7, null)).toEqual(["new", 0.7]);
    expect(compare("higher-is-better", 0.7, undefined)).toEqual(["new", 0.7]);
  });

  test("TC-957 gate-zero carries no baseline and no tolerance to spend", () => {
    // TC-957
    // A gate is not a score. Anything non-zero is a regression regardless of
    // history, and the proposed baseline is forced back to 0 so `--update`
    // cannot launder a non-zero value into the accepted state.
    expect(compare("gate-zero", 0, null)).toEqual(["held", 0]);
    expect(compare("gate-zero", 1, null)).toEqual(["regressed", 0]);
    expect(compare("gate-zero", 3, 3)).toEqual(["regressed", 0]);
  });
});

describe("a vanished family", () => {
  test("TC-960 a family the baseline scored and this run does not report is a regression", () => {
    // TC-960
    // Delete a corpus, drop a mapping, remove a label — and without this the
    // family simply stops appearing and the ratchet says nothing. An absent row
    // is indistinguishable from an absence of news, which is the same shape as
    // a check that cannot fail. Exercised through the runner's own ratchet
    // rather than by hand, so the wiring is covered too.
    const report = {
      families: [
        {
          family: "kept",
          truePositives: 1,
          falsePositives: 0,
          misses: 0,
          precision: 1,
          recall: 1,
        },
      ],
      finding_localisation_rate: null,
    };
    const previous = {
      families: [
        { family: "kept", precision: 1, recall: 1 },
        { family: "vanished", precision: 1, recall: 1 },
      ],
      finding_localisation_rate: null,
    };
    const verdicts = ratchet(report, previous, {
      metrics: {
        finding_precision: { direction: "higher-is-better" },
        finding_recall: { direction: "higher-is-better" },
      },
    });
    const gone = verdicts.find((v) => v.family === "vanished");
    expect(gone?.verdict).toBe("regressed");
    expect(gone?.observed).toBeNull();
    // The baseline is kept, so `--update` after a deletion cannot ratify it.
    expect(gone?.baseline).toBe(1);
    expect(gone?.why).toMatch(/did not report it at all/);
  });
});

describe("the committed mapping table", () => {
  const mapping = JSON.parse(
    readFileSync(join(__dirname, "..", "bench", "tier1-mapping.json"), "utf8"),
  );
  const dictionary = JSON.parse(
    readFileSync(join(__dirname, "..", "bench", "metrics.json"), "utf8"),
  );

  test("TC-958 every declared family is mapped, and every mapping names a real source", () => {
    // TC-958
    // The mapping is the contract between what a tool emits and what the
    // benchmark calls a finding. A family the dictionary declares but the table
    // does not map scores 0 forever and looks like a detector failure; a
    // mapping naming a payload section that does not exist is a rule that can
    // never fire.
    const sources = new Set([...Object.keys(mapping.sources), "none"]);
    for (const family of dictionary.families) {
      expect(
        mapping.families[family],
        `${family} is declared in bench/metrics.json and mapped nowhere`,
      ).toBeDefined();
      expect(sources).toContain(mapping.families[family].source);
      expect(mapping.families[family].key).toBeTruthy();
    }
    // And no mapping for a family nothing declares.
    for (const family of Object.keys(mapping.families)) {
      expect(dictionary.families).toContain(family);
    }
  });

  test("TC-959 a family with no detector declares `source: none` rather than being omitted", () => {
    // TC-959
    // The difference between "we looked and the tool said nothing" and "nothing
    // looks for this" is the whole substance of a recall of 0. An omitted
    // family reads as an oversight; `source: none` reads as a declared hole,
    // and it is the state `gate-that-gates-nothing` is genuinely in.
    const holes = Object.entries(mapping.families).filter(
      ([, m]: [string, { source: string }]) => m.source === "none",
    );
    expect(holes.length).toBeGreaterThan(0);
    for (const [, m] of holes as Array<[string, { $note?: string }]>) {
      expect(m.$note).toBeTruthy();
    }
  });
});
