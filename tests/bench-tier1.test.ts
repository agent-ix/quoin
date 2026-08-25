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

import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  byLanguage,
  compare,
  flattenLabels,
  loadCorpus,
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

describe("a case whose ground truth maps to nothing", () => {
  // Torn down in `afterAll`: quoin#184 was `mkdtempSync` fixtures with no
  // teardown path, and a test suite that leaves temp trees behind is the same
  // class of defect as a worktree nobody removes.
  const roots: string[] = [];
  const corpusWith = (expectYaml: string, caseYaml: string) => {
    const root = mkdtempSync(join(tmpdir(), "quoin-tier1-"));
    roots.push(root);
    const dir = join(root, "cases", "minting", "a-case");
    mkdirSync(join(dir, "input"), { recursive: true });
    writeFileSync(join(dir, "case.yaml"), caseYaml);
    writeFileSync(join(dir, "expect.yaml"), expectYaml);
    return root;
  };
  const CASE =
    "id: a-case\nmode: minting\nlanguage: rust\nmodule: m\nkind: failure\n";
  const MAPPING = {
    families: {
      "known-family": { source: "coverage.diagnostics", key: "known-reason" },
    },
  };

  afterAll(() => {
    for (const root of roots) rmSync(root, { recursive: true, force: true });
  });

  test("TC-964 an expect.yaml reason no family claims fails the run, and is never skipped", () => {
    // TC-964
    // agent-ix/quoin#236. `defectsFrom` used to `continue` past a reason the
    // family table did not recognise, so a case whose ONLY expectation was
    // unmapped derived zero defects — and derived zero silently. That is how
    // `section-matches-nothing` reached the corpus, the engine emitted it on
    // every run, and no tier-1 number moved: quire-rs#270 is the fix for
    // 3,514 unminted TC ids across 88 repositories, and the benchmark could
    // not see it land.
    const root = corpusWith("diagnostic_reasons:\n  - unmapped-reason\n", CASE);
    expect(() => loadCorpus(MAPPING, root)).toThrow(
      /no family in bench\/tier1-mapping\.json claims/,
    );
    // And the message must name the escape hatch, or the next author deletes
    // the expectation to make the error go away.
    expect(() => loadCorpus(MAPPING, root)).toThrow(/source: none/);
  });

  test("TC-965 a recognised reason still loads, so the guard refuses only the hole", () => {
    // TC-965
    // The counterpart assertion. A guard that refuses everything is not a
    // guard, and this is what distinguishes "the table does not claim this"
    // from "the table is unreadable".
    const root = corpusWith("diagnostic_reasons:\n  - known-reason\n", CASE);
    const { corpora } = loadCorpus(MAPPING, root);
    expect(corpora).toHaveLength(1);
    expect(corpora[0].family).toBe("known-family");
    expect(corpora[0].defects[0].expect_reason).toBe("known-reason");
    // The language the case declares rides along, so a `held` verdict over a
    // single-language corpus cannot read as "verified in every language".
    expect(corpora[0].language).toBe("rust");
  });
});

describe("the score cut by language", () => {
  test("TC-966 per-language rows partition the same findings the headline used", () => {
    // TC-966
    // The corpus was 22 of 22 `language: rust` when Wave 3's before/after
    // reported every family `held`, and two of the six fixes were for the
    // other two languages. One table over one language reads as a statement
    // about the toolchain and is a statement about Rust (quoin#236).
    const corpora = [
      { name: "r1", language: "rust" },
      { name: "p1", language: "python" },
    ];
    const findings = [
      { family: "f", reason: "x", corpus: "r1", path: "src/lib.rs" },
      { family: "f", reason: "x", corpus: "p1", path: "src/lib.py" },
    ];
    const labels = [
      {
        id: "R-1",
        family: "f",
        corpus: "r1",
        location: "src/lib.rs",
        findable: true,
      },
      {
        id: "P-1",
        family: "f",
        corpus: "p1",
        location: "src/lib.py",
        findable: true,
      },
    ];
    const cut = byLanguage(corpora, findings, labels, {});
    expect(cut.map((l) => l.language)).toEqual(["python", "rust"]);
    for (const row of cut) {
      expect(row.corpora).toBe(1);
      expect(row.families).toEqual([
        expect.objectContaining({ family: "f", truePositives: 1, misses: 0 }),
      ]);
    }
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
