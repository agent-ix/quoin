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
  existsSync,
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
  canonicalCorpusInventory,
  comparability,
  compare,
  declarationProvenance,
  flattenLabels,
  loadCorpus,
  localisationRate,
  measurementRecord,
  ratchet,
  silentZeros,
  validateCanonicalInventory,
} from "../scripts/bench-tier1.mjs";

describe("the silent-zero sentinel", () => {
  const ratio = (over: Record<string, unknown>) => ({
    name: "coverage.backed",
    shape: "ratio",
    state: "measured",
    value: 0,
    population: 2389,
    examined: 2389,
    matched: 0,
    ...over,
  });

  test("TC-988 a ratio reading none of a non-zero population, unaccompanied, is a violation", () => {
    // TC-988
    // `555/2389 (23%)` — arithmetic over a corpus the binder could not read,
    // published with nothing saying so. The number that made three reviews
    // wrong and opened agent-ix/quire-rs#264. The dictionary has declared this
    // gate since it was written; nothing computed it (agent-ix/quoin#243).
    const { violations } = silentZeros([
      { name: "a-case", metrics: [ratio({})], diagnostics: [] },
    ]);
    expect(violations).toHaveLength(1);
    expect(violations[0]).toMatchObject({
      corpus: "a-case",
      metric: "coverage.backed",
      population: 2389,
    });
  });

  test("TC-989 a COUNT-shaped metric reading zero is exempt", () => {
    // TC-989
    // CR-098. For a count, `matched` and the value are the same fact: a zero
    // reports that none was found, not that none was read. Failing on it would
    // make every clean corpus a gate violation.
    const { violations, unread } = silentZeros([
      {
        name: "a-case",
        metrics: [ratio({ name: "coverage.implements", shape: "count" })],
        diagnostics: [],
      },
    ]);
    expect(violations).toEqual([]);
    expect(unread).toEqual([]);
  });

  test("TC-990 accompanied means a diagnostic NAMES the metric, not that one exists", () => {
    // TC-990
    // Every ecosystem-bound fixture in this corpus emits four
    // `archetype-matches-nothing`, so "the payload carried a diagnostic" is
    // true of every case here and would excuse every hollow ratio in it. The
    // accompanying diagnostic has to name the metric — in `value`, the way
    // `catch-all-universal` names `coverage.specific_shaped`, or in the
    // message, the way `hollow-denominator` names `coverage.backed`.
    const unrelated = silentZeros([
      {
        name: "a-case",
        metrics: [ratio({})],
        diagnostics: [
          {
            reason: "archetype-matches-nothing",
            message: "declaration 'suite' declares archetype 'SuiteRegistry'",
          },
        ],
      },
    ]);
    expect(unrelated.violations).toHaveLength(1);

    const byMessage = silentZeros([
      {
        name: "a-case",
        metrics: [ratio({})],
        diagnostics: [
          {
            reason: "hollow-denominator",
            message: "`coverage.backed` published a ratio over 4 matrix rows",
          },
        ],
      },
    ]);
    expect(byMessage.violations).toEqual([]);

    const byValue = silentZeros([
      {
        name: "a-case",
        metrics: [ratio({})],
        diagnostics: [
          { reason: "catch-all-universal", value: "coverage.backed" },
        ],
      },
    ]);
    expect(byValue.violations).toEqual([]);
  });

  test("TC-991 an unmeasured metric is an absence, not a silent zero", () => {
    // TC-991
    // quire-rs FR-063 already reports a metric it could not arrive at as
    // `state != measured`. Counting that as a hollow ratio would fail the gate
    // for the engine correctly saying it did not know.
    expect(
      silentZeros([
        {
          name: "a-case",
          metrics: [ratio({ state: "unavailable" })],
          diagnostics: [],
        },
      ]),
    ).toEqual({ violations: [], unread: [] });
    // A ratio over a ZERO population is 0/0 — nothing was there to read, which
    // is a different claim from reading none of what was.
    expect(
      silentZeros([
        {
          name: "a-case",
          metrics: [ratio({ population: 0, examined: 0 })],
          diagnostics: [],
        },
      ]),
    ).toEqual({ violations: [], unread: [] });
  });

  test("TC-993 walking NONE of a population is reported, not gated — it is a different claim", () => {
    // TC-993
    // THE FIRST DRAFT OF THIS GATE FIRED ON THREE CORPUS CASES, and every one
    // was correct input: `catch-all-properties`, `greenfield-no-symbols` and
    // `gate-that-gates-nothing` each seed a tree with no evidence symbols at
    // all, so `coverage.backed` honestly backs 0 of its rows and no reading was
    // missed. Bad rule, not bad corpus.
    //
    // The engine's own envelope draws the line: `coverage.backed`'s `method`
    // says "matched 0 of a NON-ZERO examined is a ratio computed over a corpus
    // the binder could not read", and `hollow-denominator` fires on exactly
    // that. `examined == 0` is authoring absence, not instrument failure.
    //
    // Still counted, because the engine says nothing about it either — which is
    // `agent-ix/quire-rs#271`. Narrowing a gate must not be the same act as
    // ceasing to look.
    const { violations, unread } = silentZeros([
      {
        name: "greenfield",
        metrics: [ratio({ population: 2, examined: 0 })],
        diagnostics: [],
      },
      {
        name: "read-none-of-two",
        metrics: [ratio({ population: 4, examined: 2 })],
        diagnostics: [],
      },
    ]);
    expect(violations.map((v) => v.corpus)).toEqual(["read-none-of-two"]);
    expect(unread.map((v) => v.corpus)).toEqual(["greenfield"]);
  });

  test("TC-992 the gate reads against 0 and ignores the baseline, so --update cannot launder it", () => {
    // TC-992
    // A gate is not a score: `compare`'s `gate-zero` branch never consults the
    // previous value, so a run that wrote a baseline with three violations in
    // it still fails the next run.
    const verdicts = ratchet(
      {
        families: [],
        finding_localisation_rate: null,
        "sentinel.silent_zero": {
          count: 3,
          instances: [
            { corpus: "a", metric: "coverage.backed" },
            { corpus: "b", metric: "coverage.backed" },
            { corpus: "c", metric: "coverage.property_shaped" },
          ],
        },
      },
      { families: [], "sentinel.silent_zero": { count: 3, instances: [] } },
      {
        metrics: {
          finding_precision: { direction: "higher-is-better" },
          finding_recall: { direction: "higher-is-better" },
          "sentinel.silent_zero": { direction: "gate-zero" },
        },
      },
    );
    const v = verdicts.find((x) => x.metric === "sentinel.silent_zero");
    expect(v?.verdict).toBe("regressed");
    expect(v?.baseline).toBe(0);
    // Named, never a bare count: a gate that says "3" and not which three is a
    // gate nobody can discharge.
    expect(v?.why).toMatch(/a\/coverage\.backed/);
  });
});

describe("the measurement series", () => {
  const report = {
    corpora: 80,
    findings: 417,
    by_language: [
      { language: "rust", corpora: 42 },
      { language: "python", corpora: 18 },
    ],
    provenance: {
      engine: "quire 0.30.2 (engine e5a6ccc)",
      corpus: "a6ce22cb2a6bb207cf8afa0ce98949bdbb3cdd98",
      declaration: { digest: "sha256:819807f757c7" },
    },
    families: [{ family: "hollow-denominator", precision: 1, recall: 1 }],
  };

  test("TC-997 tool version and config digest come from the payload, not from an operator", () => {
    // TC-997
    // The two fields that would have PREVENTED the defect this EPIC exists to
    // fix. The installed `quire` was CLI 0.29.0 pinning engine v0.42.0, sixteen
    // releases behind and unable to emit `binding_census` at all, and every
    // recent ecosystem figure came from it. `quire --version` reports the CLI
    // crate version, so a current CLI linking a stale engine is invisible to
    // anything that trusts a typed string (agent-ix/quoin#228).
    const record = measurementRecord(report, "2026-08-25T00:00:00.000Z");
    expect(record.toolVersion).toBe("quire 0.30.2 (engine e5a6ccc)");
    expect(record.configDigest).toBe("sha256:819807f757c7");
    expect(record.corpusRevision).toBe(report.provenance.corpus);
    // The POPULATION, because a delta across a moved population is not a delta:
    // quire-rs#272 adds ~3,514 rows to the denominator, and a record that does
    // not carry its population lets that read as a regression.
    expect(record.scope).toMatchObject({ corpora: 80, findings: 417 });
    expect(record.scope.by_language).toEqual({ rust: 42, python: 18 });
  });

  test("TC-998 the raw report is ATTACHED, never transcribed into the record's own fields", () => {
    // TC-998
    // Three published SpecReviews cited hand-typed figures from a binary whose
    // self-reported version was wrong. The whole payload rides along so a later
    // reader re-derives rather than re-types (agent-ix/quoin#228).
    const record = measurementRecord(report, "2026-08-25T00:00:00.000Z");
    expect(record.rawEvidence).toBe(report);
    expect(record.timestamp).toBe("2026-08-25T00:00:00.000Z");
    expect(record.observations.length).toBeGreaterThan(0);
  });

  test("TC-1000 the superseded JSONL series remains readable as legacy evidence", () => {
    // These observations predate MeasurementPlans. Preserve them, but do not
    // silently treat them as active plan-governed collections.
    const path = join(__dirname, "..", "bench", "measurements.jsonl");
    if (!existsSync(path)) return;
    const records = readFileSync(path, "utf8")
      .split("\n")
      .filter(Boolean)
      .map((l) => JSON.parse(l));
    expect(records.length).toBeGreaterThan(0);
    for (const r of records) {
      expect(r.tool_version).toBeTruthy();
      expect(r.config_digest).toBeTruthy();
      expect(r.corpus_revision).toBeTruthy();
      expect(r.definition_version).toBeTruthy();
      expect(r.at).toBeTruthy();
      expect(r.schemaVersion).toBeUndefined();
    }
  });
});

describe("the committed baseline and the committed scorer", () => {
  test("TC-995 the real baseline compared against itself holds on every verdict", () => {
    // TC-995
    // `make bench-tier1` is invoked by nothing (agent-ix/quoin#244) and the
    // ratchet tests all ran against hand-built synthetic objects, so
    // `bench/tier1-baseline.json` was a committed file no automated run ever
    // read. A hand-edit into a shape the scorer cannot parse would have been
    // contradicted by nothing until the next time a human typed the command.
    //
    // This needs no `quire` binary: a baseline compared against ITSELF must
    // produce no `regressed` and no `new`. `new` means the scorer looked for a
    // field the baseline does not carry, which is the drift worth catching.
    const baseline = JSON.parse(
      readFileSync(
        join(__dirname, "..", "bench", "tier1-baseline.json"),
        "utf8",
      ),
    );
    const dictionary = JSON.parse(
      readFileSync(join(__dirname, "..", "bench", "metrics.json"), "utf8"),
    );
    const verdicts = ratchet(baseline, baseline, dictionary);
    expect(verdicts.length).toBeGreaterThan(0);
    const notHeld = verdicts.filter((v) => v.verdict !== "held");
    expect(
      notHeld.map((v) => `${v.metric}[${v.family ?? "-"}]=${v.verdict}`),
    ).toEqual([]);
  });

  test("TC-996 every family the baseline scored is one the mapping still claims", () => {
    // TC-996
    // A family dropped from `bench/tier1-mapping.json` while its row stays in
    // the baseline is a score against a contract nobody holds. `ratchet` already
    // fails a family that vanishes from a RUN; nothing held the two committed
    // files to each other.
    const baseline = JSON.parse(
      readFileSync(
        join(__dirname, "..", "bench", "tier1-baseline.json"),
        "utf8",
      ),
    );
    const mapping = JSON.parse(
      readFileSync(
        join(__dirname, "..", "bench", "tier1-mapping.json"),
        "utf8",
      ),
    );
    const claimed = new Set(Object.keys(mapping.families));
    const orphans = baseline.families
      .map((f: { family: string }) => f.family)
      .filter((f: string) => !claimed.has(f));
    expect(orphans).toEqual([]);
  });
});

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

  test("TC-984 a family the baseline measured and this run reports null is regressed, not skipped", () => {
    // TC-984
    // THE ESCAPE HATCH. `shape: advisory` in `bench/tier1-mapping.json` turns a
    // family's precision to null, and the old `if (family[metric] === null)
    // continue` then emitted no verdict at all — so a bad number could be
    // deleted, in silence, for the cost of editing one string in one JSON file.
    // It was used twice: #234 on `catch-all-universal` at 0.167, and CR-102
    // (`be60e57`) on `archetype-matches-nothing` at 3 TP / 296 FP = 0.01
    // (agent-ix/quoin#245).
    const verdicts = ratchet(
      {
        families: [
          {
            family: "reclassified",
            truePositives: 3,
            falsePositives: 296,
            misses: 0,
            precision: null,
            recall: 1,
          },
        ],
        finding_localisation_rate: null,
      },
      {
        families: [{ family: "reclassified", precision: 0.01, recall: 1 }],
        finding_localisation_rate: null,
      },
      {
        metrics: {
          finding_precision: { direction: "higher-is-better" },
          finding_recall: { direction: "higher-is-better" },
        },
      },
    );
    const v = verdicts.find((x) => x.metric === "finding_precision");
    expect(v?.verdict).toBe("regressed");
    expect(v?.observed).toBeNull();
    // The baseline is KEPT, so `--update` cannot ratify the deletion either.
    expect(v?.baseline).toBe(0.01);
    expect(v?.why).toMatch(/stopped being measured/);
  });

  test("TC-985 a family that never had a precision is still skipped, not failed", () => {
    // TC-985
    // The other half of TC-984, and the reason the old `continue` existed:
    // `gate-that-gates-nothing` has no detector anywhere, so it reports null
    // every run. Failing the build for a family nothing ever fired on would
    // make the gate permanently red and therefore permanently ignored.
    const verdicts = ratchet(
      {
        families: [
          {
            family: "never-measured",
            truePositives: 0,
            falsePositives: 0,
            misses: 1,
            precision: null,
            recall: 0,
          },
        ],
        finding_localisation_rate: null,
      },
      {
        families: [{ family: "never-measured", precision: null, recall: 0 }],
        finding_localisation_rate: null,
      },
      {
        metrics: {
          finding_precision: { direction: "higher-is-better" },
          finding_recall: { direction: "higher-is-better" },
        },
      },
    );
    expect(verdicts.some((v) => v.metric === "finding_precision")).toBe(false);
    expect(verdicts.find((v) => v.metric === "finding_recall")?.verdict).toBe(
      "held",
    );
  });

  test("TC-986 an advisory's unadjudicated count is ratcheted lower-is-better", () => {
    // TC-986
    // An advisory's precision RATE cannot fall independently of qa-corpus's own
    // `make ci` — a firing on a case that declared the reason absent turns that
    // gate red first. So the rate is not the evidence; the count of firings
    // nobody has ruled on is, and it must cost something to grow.
    const verdicts = ratchet(
      {
        families: [
          {
            family: "advisory-family",
            truePositives: 3,
            falsePositives: 316,
            misses: 0,
            precision: 1,
            recall: 1,
            precision_basis: {
              precision: 1,
              truePositives: 3,
              falsePositives: 0,
              unadjudicated: 400,
              rulings: 6,
            },
          },
        ],
        finding_localisation_rate: null,
      },
      {
        families: [
          {
            family: "advisory-family",
            precision: 1,
            recall: 1,
            precision_basis: { unadjudicated: 313 },
          },
        ],
        finding_localisation_rate: null,
      },
      {
        metrics: {
          finding_precision: { direction: "higher-is-better" },
          finding_recall: { direction: "higher-is-better" },
        },
      },
    );
    const v = verdicts.find(
      (x) => x.metric === "finding_precision.unadjudicated",
    );
    expect(v?.verdict).toBe("regressed");
    expect(v?.observed).toBe(400);
    expect(v?.baseline).toBe(313);
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
    // The declaration the case binds has to EXIST (quoin#240): a module id
    // resolving to nothing would leave the engine registering no archetype and
    // every case reporting an empty payload, so `loadCorpus` refuses it.
    mkdirSync(join(root, "modules", "m"), { recursive: true });
    writeFileSync(
      join(root, "modules", "m", "manifest.yaml"),
      "archetypes: []\n",
    );
    // The case schema comes from the DECLARATION (quoin#242). A reader that
    // enforces nothing when its rule is missing is indistinguishable from one
    // that enforced it and found nothing, so `loadCorpus` refuses a corpus
    // root carrying no `corpus.yaml`.
    writeFileSync(
      join(root, "corpus.yaml"),
      "case_schema:\n  variant_forbidden:\n  - case\n  - mode\n  - module\n" +
        "  - kind\n  - pending\n",
    );
    return {
      root,
      inventory: {
        bounds: { gap_count: 0 },
        cases: [
          {
            id: "a-case",
            mode: "minting",
            language: "rust",
            module: "m",
            kind: "failure",
            dir: "cases/minting/a-case",
            expect: "cases/minting/a-case/expect.yaml",
          },
        ],
      },
    };
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
    const { root, inventory } = corpusWith(
      "diagnostic_reasons:\n  - unmapped-reason\n",
      CASE,
    );
    expect(() => loadCorpus(MAPPING, root, null, inventory)).toThrow(
      /no family in bench\/tier1-mapping\.json claims/,
    );
    // And the message must name the escape hatch, or the next author deletes
    // the expectation to make the error go away.
    expect(() => loadCorpus(MAPPING, root, null, inventory)).toThrow(
      /source: none/,
    );
  });

  test("TC-965 a recognised reason still loads, so the guard refuses only the hole", () => {
    // TC-965
    // The counterpart assertion. A guard that refuses everything is not a
    // guard, and this is what distinguishes "the table does not claim this"
    // from "the table is unreadable".
    const { root, inventory } = corpusWith(
      "diagnostic_reasons:\n  - known-reason\n",
      CASE,
    );
    const { corpora } = loadCorpus(MAPPING, root, null, inventory);
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

describe("the declaration axis", () => {
  // Torn down in `afterAll` for quoin#184's reason: a fixture tree with no
  // teardown path is the same class of defect as a worktree nobody removes.
  const roots: string[] = [];
  const declarationRoot = (manifestBody: string, vendored: string | null) => {
    const root = mkdtempSync(join(tmpdir(), "quoin-decl-"));
    roots.push(root);
    mkdirSync(join(root, "ecosystem", "a-module"), { recursive: true });
    writeFileSync(
      join(root, "ecosystem", "a-module", "manifest.yaml"),
      manifestBody,
    );
    // The modules the table NAMES have to be here. A row is recognised by its
    // first cell resolving to a directory in the same tree (quoin#240), which
    // is what separates a data row from the header and the `|---|` rule
    // without matching on their spelling — and additionally catches a
    // provenance file naming a module the copy does not carry.
    for (const m of ["spec-artifacts-process", "spec-artifacts-iso"]) {
      mkdirSync(join(root, "ecosystem", m), { recursive: true });
    }
    if (vendored !== null) {
      writeFileSync(join(root, "ecosystem", "VENDORED.md"), vendored);
    }
    return root;
  };
  const TABLE =
    "| Module | Source path | Pinned SHA |\n|---|---|---|\n" +
    "| `spec-artifacts-process` | `spec_artifacts_process/manifest.yaml` " +
    "| `c197b1c0a10148164620ca0626d82ca5edd032bd` |\n";
  const CASE =
    "id: a-case\nmode: minting\nlanguage: typescript\nmodule: ecosystem\n" +
    "kind: failure\n";

  afterAll(() => {
    for (const root of roots) rmSync(root, { recursive: true, force: true });
  });

  test("TC-968 the report records WHICH declaration it scored against, by content and by upstream SHA", () => {
    // TC-968
    // agent-ix/quoin#240. The report named the engine and the corpus and not
    // the third input — and two of Wave 3's six fixes changed nothing else, so
    // `spec-artifacts-process#68` scored `held` in the same word the runner
    // prints for a family that genuinely did not move.
    //
    // BOTH keys, because they answer different questions. The digest is
    // measured from the bytes and is what makes two reports comparable or not;
    // the SHA is the corpus's own record of where the copy came from and is the
    // only way a reader can fetch the diff. A vendored copy carries no git
    // identity of its own, so neither can be derived from the other.
    const before = declarationRoot("archetypes: []\n", TABLE);
    const after = declarationRoot("archetypes: []\n# one comment\n", TABLE);
    const same = declarationRoot("archetypes: []\n", TABLE);

    const p = declarationProvenance(before, ["ecosystem"]);
    expect(p.digest).toMatch(/^sha256:[0-9a-f]{64}$/);
    expect(p.sources).toEqual({
      "spec-artifacts-process": "c197b1c0a10148164620ca0626d82ca5edd032bd",
    });
    expect(p.modules).toHaveProperty("ecosystem");

    // One byte of one manifest moves the digest; identical bytes do not. A
    // digest that did not move on a manifest edit would make the refusal below
    // unable to fire on exactly the change it exists for.
    expect(declarationProvenance(after, []).digest).not.toBe(p.digest);
    expect(declarationProvenance(same, []).digest).toBe(p.digest);
  });

  test("TC-969 a VENDORED.md present and unreadable fails the run rather than reporting no source", () => {
    // TC-969
    // A provenance file that has silently stopped parsing is WORSE than none:
    // the report keeps printing a confident `sources` beside numbers nobody can
    // join to a commit, which is the defect quoin#229 records one file over.
    // Absent is a different claim and is allowed — a declaration built by hand
    // for a one-off comparison has no upstream SHA to record.
    const broken = declarationRoot("archetypes: []\n", "no table here\n");
    expect(() => declarationProvenance(broken, [])).toThrow(
      /records no .* row this runner can read/s,
    );
    expect(
      declarationProvenance(declarationRoot("a: 1\n", null), []).sources,
    ).toBeNull();
  });

  test("TC-981 an unreadable SHA on ONE row of a table fails the run, and an annotated SHA is read rather than dropped", () => {
    // TC-981
    // agent-ix/quoin#240, REOPENED. The guard was `if (!rows)` — all-or-nothing
    // — so a table of two dropped the unreadable row the moment the other one
    // parsed. Measured on `qa-corpus@41c6224` while re-pinning for #242: the
    // report printed `sources: {spec-artifacts-iso}` and `spec-artifacts-process`
    // was simply gone. That is the module carrying the traceability model,
    // whose five lines decide whether a TypeScript test's own title binds, and
    // it is exactly the "confident `sources` beside numbers nobody could join
    // to a commit" this function exists to refuse — arriving one row at a time.
    const twoRows = (second: string) =>
      "| Module | Source path | Pinned SHA |\n|---|---|---|\n" +
      "| `spec-artifacts-iso` | `iso/manifest.yaml` " +
      "| `3d871962b66db99a1854f40466e94ebabc7a6115` |\n" +
      `| \`spec-artifacts-process\` | \`process/manifest.yaml\` | ${second} |\n`;

    expect(() =>
      declarationProvenance(
        declarationRoot("archetypes: []\n", twoRows("`not-a-sha`")),
        [],
      ),
    ).toThrow(/records module `spec-artifacts-process` with `not-a-sha`/);

    // An ANNOTATED sha is read, not refused. The corpus deliberately records a
    // branch head as ``62d691f` (`feat/68-typescript-test-name-form`)` so a
    // reader knows the pin is not on `main`; the hash is the first token and
    // the rest is provenance prose.
    const annotated = declarationProvenance(
      declarationRoot(
        "archetypes: []\n",
        twoRows("`62d691f` (`feat/68-typescript-test-name-form`)"),
      ),
      [],
    );
    expect(annotated.sources).toEqual({
      "spec-artifacts-iso": "3d871962b66db99a1854f40466e94ebabc7a6115",
      "spec-artifacts-process": "62d691f",
    });
  });

  test("TC-970 cases resolve their module under an overridden root, and an id resolving to nothing fails the run", () => {
    // TC-970
    // The axis itself: the same cases, scored against another declaration with
    // the engine held fixed. That is the only way a declaration-side fix is
    // distinguishable from a fix that had no effect.
    //
    // And the failure mode it opens. `findingsFor` decides between `--module`
    // and `IX_FILAMENT_MODULES_PATH` by asking whether the directory holds a
    // `manifest.yaml`; a directory that does not exist answers "no", the engine
    // registers no archetype, every case reports an empty payload, and a
    // mistyped path reads as a total detection collapse.
    const corpusRoot = mkdtempSync(join(tmpdir(), "quoin-tier1-"));
    roots.push(corpusRoot);
    const dir = join(corpusRoot, "cases", "minting", "a-case");
    mkdirSync(join(dir, "input"), { recursive: true });
    writeFileSync(join(dir, "case.yaml"), CASE);
    writeFileSync(
      join(corpusRoot, "corpus.yaml"),
      "case_schema:\n  variant_forbidden:\n  - case\n",
    );
    writeFileSync(join(dir, "expect.yaml"), "{}\n");
    const inventory = {
      bounds: { gap_count: 0 },
      cases: [
        {
          id: "a-case",
          mode: "minting",
          language: "rust",
          module: "ecosystem",
          kind: "failure",
          dir: "cases/minting/a-case",
          expect: "cases/minting/a-case/expect.yaml",
        },
      ],
    };
    const declaration = declarationRoot("archetypes: []\n", TABLE);

    const { corpora, modulesRoot } = loadCorpus(
      { families: {} },
      corpusRoot,
      declaration,
      inventory,
    );
    expect(modulesRoot).toBe(declaration);
    expect(corpora[0].module).toBe(join(declaration, "ecosystem"));

    const empty = mkdtempSync(join(tmpdir(), "quoin-decl-empty-"));
    roots.push(empty);
    expect(() =>
      loadCorpus({ families: {} }, corpusRoot, empty, inventory),
    ).toThrow(/holds no `manifest\.yaml`/);
  });
});

describe("a delta across unlike inputs", () => {
  const report = {
    provenance: { engine: "e2", corpus: "c1", declaration: { digest: "d1" } },
    corpora: 34,
    by_language: [{ language: "rust", corpora: 34 }],
    families: [
      {
        family: "f",
        truePositives: 1,
        falsePositives: 0,
        misses: 0,
        precision: 0.5,
        recall: 1,
      },
    ],
    finding_localisation_rate: null,
  };
  const previous = {
    provenance: { engine: "e1", corpus: "c1", declaration: { digest: "d1" } },
    corpora: 34,
    by_language: [{ language: "rust", corpora: 34 }],
    families: [{ family: "f", precision: 1, recall: 1 }],
    finding_localisation_rate: null,
  };

  test("TC-971 the ENGINE may move and still be compared; the corpus, the declaration and the population may not", () => {
    // TC-971
    // agent-ix/quoin#231 and #240's second half. Varying the engine and
    // comparing is what this benchmark is FOR, so it is deliberately not a
    // reason. The other three are the inputs a delta is only meaningful while
    // they are held — and the last pass nearly published a spurious result on
    // exactly this: the `84740d4` leg read `regressed` on every family because
    // the scored population had gone 21 -> 34, and nothing in the output said
    // so.
    expect(comparability(report, previous).comparable).toBe(true);

    const moved = (patch: object) =>
      comparability({ ...report, ...patch }, previous);
    expect(
      moved({
        provenance: { ...report.provenance, declaration: { digest: "d2" } },
      }).reasons[0].field,
    ).toBe("provenance.declaration.digest");
    expect(
      moved({ provenance: { ...report.provenance, corpus: "c2" } }).reasons[0]
        .field,
    ).toBe("provenance.corpus");
    expect(moved({ corpora: 21 }).reasons[0].field).toBe("corpora");
    // The count can hold while the MIX changes — swap a rust case for a python
    // one and `corpora` says 34 either way.
    expect(
      moved({ by_language: [{ language: "python", corpora: 34 }] }).reasons[0]
        .field,
    ).toBe("by_language");
  });

  test("TC-972 an incomparable run withdraws the CLAIM, keeps both numbers, and is not called a regression", () => {
    // TC-972
    // The distinction that makes this usable: `improved` and `regressed` are
    // statements about a CHANGE, and a run over another declaration did not
    // observe one — the subject changed. Reporting it as a regression would
    // blame the toolchain for the operator moving an input, which is what
    // happened to the `84740d4` leg last pass. Both values stay printed,
    // because withholding them would make the axis unmeasurable.
    const verdicts = ratchet(
      {
        ...report,
        provenance: { ...report.provenance, declaration: { digest: "d2" } },
      },
      previous,
      {
        metrics: {
          finding_precision: { direction: "higher-is-better" },
          finding_recall: { direction: "higher-is-better" },
        },
      },
    );
    expect(verdicts).toHaveLength(2);
    for (const v of verdicts) expect(v.verdict).toBe("incomparable");
    const precision = verdicts.find((v) => v.metric === "finding_precision");
    expect(precision?.observed).toBe(0.5);
    expect(precision?.baseline).toBe(1);
    expect(precision?.why).toMatch(/provenance\.declaration\.digest moved/);
    const improved = ratchet(
      {
        ...report,
        provenance: { ...report.provenance, declaration: { digest: "d2" } },
        actionability: { rate: 0.188 },
      },
      { ...previous, actionability: { rate: 0.004 } },
      {
        metrics: {
          finding_precision: { direction: "higher-is-better" },
          finding_recall: { direction: "higher-is-better" },
          actionability_rate: { direction: "higher-is-better" },
        },
      },
    ).find((v) => v.metric === "actionability_rate");
    expect(improved).toMatchObject({
      verdict: "incomparable",
      observed: 0.188,
      baseline: 0.004,
    });
    // Without the guard this run reads `regressed` — the assertion that the
    // refusal is doing work rather than restating a verdict already reached.
    expect(
      ratchet(report, previous, {
        metrics: {
          finding_precision: { direction: "higher-is-better" },
          finding_recall: { direction: "higher-is-better" },
        },
      }).find((v) => v.metric === "finding_precision")?.verdict,
    ).toBe("regressed");
  });

  test("TC-973 a baseline that records nothing for a field is UNKNOWN, not a match", () => {
    // TC-973
    // The legacy baseline. It records no declaration, so a comparison against
    // it rests on an assumption nobody stated — and the two available answers,
    // refusing every old baseline or quietly assuming it matched, are both
    // wrong. It is reported as unknown and the run says so out loud.
    const legacy = {
      ...previous,
      provenance: { engine: "e1", corpus: "c1" },
    };
    const { comparable, unknown } = comparability(report, legacy);
    expect(comparable).toBe(true);
    expect(unknown).toContain("provenance.declaration.digest");
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

describe("the canonical qa-corpus inventory", () => {
  const roots: string[] = [];
  const SCHEMA =
    "case_schema:\n  variant_forbidden:\n  - case\n  - mode\n  - module\n" +
    "  - kind\n  - pending\n";
  const SHARED =
    "id: a-case\nmode: minting\nmodule: m\nkind: failure\nfindable: true\n";
  const inventory = (cases: Array<Record<string, unknown>>) => ({
    bounds: { gap_count: 0 },
    cases,
  });
  const entry = (overrides: Record<string, unknown> = {}) => ({
    id: "a-case",
    mode: "minting",
    language: "rust",
    module: "m",
    kind: "failure",
    dir: "cases/minting/a-case",
    expect: "cases/minting/a-case/expect.yaml",
    ...overrides,
  });

  /** A corpus root with one case directory the caller shapes. */
  const corpus = (shape: (caseDir: string) => void, schema = SCHEMA) => {
    const root = mkdtempSync(join(tmpdir(), "quoin-layout-"));
    roots.push(root);
    const dir = join(root, "cases", "minting", "a-case");
    mkdirSync(dir, { recursive: true });
    mkdirSync(join(root, "modules", "m"), { recursive: true });
    writeFileSync(
      join(root, "modules", "m", "manifest.yaml"),
      "archetypes: []\n",
    );
    writeFileSync(join(root, "corpus.yaml"), schema);
    shape(dir);
    return root;
  };

  afterAll(() => {
    for (const root of roots) rmSync(root, { recursive: true, force: true });
  });

  test("TC-974 a language set yields one scorable case per language, each with its own id and input tree", () => {
    // TC-974
    const root = corpus((dir) => {
      writeFileSync(join(dir, "case.yaml"), SHARED);
      for (const language of ["python", "rust", "typescript"]) {
        mkdirSync(join(dir, language, "input"), { recursive: true });
        writeFileSync(
          join(dir, language, "case.yaml"),
          `reproduce: quire coverage --scope ${language}\n`,
        );
        writeFileSync(join(dir, language, "expect.yaml"), "{}\n");
      }
    });
    const cases = ["python", "rust", "typescript"].map((language) =>
      entry({
        id: `a-case-${language}`,
        language,
        dir: `cases/minting/a-case/${language}`,
        expect: `cases/minting/a-case/${language}/expect.yaml`,
      }),
    );
    const { corpora } = loadCorpus(
      { families: {} },
      root,
      null,
      inventory(cases),
    );
    expect(corpora.map((c) => c.name).sort()).toEqual([
      "a-case-python",
      "a-case-rust",
      "a-case-typescript",
    ]);
    expect(corpora.map((c) => c.language).sort()).toEqual([
      "python",
      "rust",
      "typescript",
    ]);
    // The id is the JOIN KEY across runners — a pending marker, a baseline row.
    // Three variants sharing one id are indistinguishable in every one of them.
    expect(new Set(corpora.map((c) => c.name)).size).toBe(3);
    for (const c of corpora) {
      expect(existsSync(c.input)).toBe(true);
      expect(c.input.endsWith(join(c.language, "input"))).toBe(true);
    }
  });

  test("TC-975 the runner consumes the authoritative inventory instead of re-reading layouts", () => {
    // qa-corpus's Python and Rust readers own layout validation. This consumer
    // receives their resolved ids, languages, directories and expectations.
    const root = join(__dirname, "..", "corpus");
    const canonical = canonicalCorpusInventory(root);
    const loaded = loadCorpus(null, root, null, canonical);
    expect(loaded.corpora).toHaveLength(canonical.cases.length);
    expect(loaded.bounds).toEqual(canonical.bounds);
    expect(loaded.corpora.every((c) => c.language !== "unknown")).toBe(true);
  });

  test("TC-976 an unreadable canonical envelope fails instead of becoming an empty corpus", () => {
    expect(() => validateCanonicalInventory({ bounds: {} })).toThrow(
      /must carry cases and numeric bounds/,
    );
    expect(() =>
      validateCanonicalInventory({ bounds: { gap_count: 0 }, cases: [{}] }),
    ).toThrow(/case 0 lacks id, dir, expect, module, language/);
  });

  test("TC-977 raw case metadata is not interpreted by this third consumer", () => {
    const root = corpus((dir) => {
      mkdirSync(join(dir, "input"), { recursive: true });
      writeFileSync(join(dir, "expect.yaml"), "{}\n");
      // Deliberately no case.yaml: the validated inventory is the interface.
    });
    const loaded = loadCorpus(
      { families: {} },
      root,
      null,
      inventory([entry()]),
    );
    expect(loaded.corpora.map((c) => c.name)).toEqual(["a-case"]);
  });

  test("TC-978 a canonical entry resolving to no fixture fails by case id", () => {
    const root = corpus(() => {});
    expect(() =>
      loadCorpus({ families: {} }, root, null, inventory([entry()])),
    ).toThrow(/canonical case a-case resolves to missing input/);
  });

  test("TC-979 a pending case's expiry signal is read from `expect-pending.yaml`, never from the live block", () => {
    // TC-979
    // The old rule demanded the future reason under `diagnostic_reasons:` in
    // `expect.yaml` — where stating it would be FALSE, because the reason does
    // not fire today, which is the whole point of the marker. All ten pending
    // cases in the corpus state it correctly in the forward block and this
    // runner read neither.
    const root = corpus((dir) => {
      writeFileSync(
        join(dir, "case.yaml"),
        `${SHARED}language: rust\npending: agent-ix/quire-rs#312\n`,
      );
      mkdirSync(join(dir, "input"), { recursive: true });
      // The live block asserts the reason is ABSENT — true today.
      writeFileSync(
        join(dir, "expect.yaml"),
        "absent_diagnostic_reasons:\n  - tag-on-non-binding-symbol\n",
      );
      writeFileSync(
        join(dir, "expect-pending.yaml"),
        "diagnostic_reasons:\n  - tag-on-non-binding-symbol\n",
      );
    });
    const pending = "agent-ix/quire-rs#312";
    const [c] = loadCorpus(
      { families: {} },
      root,
      null,
      inventory([entry({ pending })]),
    ).corpora;
    expect(c.pending).toBe("agent-ix/quire-rs#312");
    expect(c.pendingReasons).toEqual(["tag-on-non-binding-symbol"]);
    expect(c.hasPendingBlock).toBe(true);
    // A reason no family claims must still reach the check: the token does not
    // exist in the engine yet and so can have no scoring family, and routing
    // staleness through the family mapping is how quoin#236 happened.
    expect(c.defects.map((d) => d.expect_reason).filter(Boolean)).toEqual([]);
  });

  test("TC-980 a pending case with no forward block at all is distinguishable from one this runner merely cannot evaluate", () => {
    // TC-980
    const noBlock = corpus((dir) => {
      writeFileSync(
        join(dir, "case.yaml"),
        `${SHARED}language: rust\npending: agent-ix/quire-rs#273\n`,
      );
      mkdirSync(join(dir, "input"), { recursive: true });
      writeFileSync(join(dir, "expect.yaml"), "total: 1\n");
    });
    const pending = "agent-ix/quire-rs#273";
    const [a] = loadCorpus(
      { families: {} },
      noBlock,
      null,
      inventory([entry({ pending })]),
    ).corpora;
    expect(a.hasPendingBlock).toBe(false);
    expect(a.pendingReasons).toEqual([]);

    // A forward block stating a PAYLOAD change rather than a diagnostic:
    // quire-rs#273 registers `describe()` as a Container so the tag starts
    // binding (`backed` 0 -> 2) and adds no reason token at all. Grading a
    // payload is what `verify.py` and the Rust harness already do over the same
    // file; a third implementation is the drift two readers exist to expose.
    const payloadOnly = corpus((dir) => {
      writeFileSync(
        join(dir, "case.yaml"),
        `${SHARED}language: rust\npending: agent-ix/quire-rs#273\n`,
      );
      mkdirSync(join(dir, "input"), { recursive: true });
      writeFileSync(join(dir, "expect.yaml"), "total: 1\n");
      writeFileSync(join(dir, "expect-pending.yaml"), "total: 4\nbacked: 2\n");
    });
    const [b] = loadCorpus(
      { families: {} },
      payloadOnly,
      null,
      inventory([entry({ pending })]),
    ).corpora;
    expect(b.hasPendingBlock).toBe(true);
    expect(b.pendingReasons).toEqual([]);
  });
});
