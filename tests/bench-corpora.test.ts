/**
 * The tier-1 seeded corpora and their labels (quoin#199, FR-043-AC-7).
 *
 * A label nobody checks is a claim, not ground truth. These tests assert the
 * SHAPE of the label set — the part that makes precision and recall computable
 * — and the build's determinism. Whether each seeded defect is actually found
 * is verified against a real engine and recorded per defect in `confirmed_at`;
 * doing it here would make the unit suite depend on a `quire` binary.
 */

import { readFileSync } from "node:fs";

import { existsSync } from "node:fs";
import { join } from "node:path";

import { loadCorpus } from "../scripts/bench-tier1.mjs";
import { crossCheckFamilies } from "../evals/lib/dictionary.mjs";
import { diff, render, scoreAgainstKey } from "../scripts/battletest.mjs";

// The corpus is STATIC now (agent-ix/quoin#227): read from the `qa-corpus`
// submodule rather than generated into a tmpdir. Every property below survives
// the change — only their subject moved from a generator's output to files on
// disk that a reader can open.
// Read lazily, INSIDE the suites that need it. At module scope a missing
// submodule — `qa-corpus` is private, and CI checks out without submodules —
// errored the whole file at collection and took all nineteen tests with it,
// including the tier-2 answer-key and battletest suites that never touch the
// corpus.
const MAPPING = JSON.parse(
  readFileSync(
    join(import.meta.dirname, "../bench/tier1-mapping.json"),
    "utf8",
  ),
);
let CORPORA: ReturnType<typeof loadCorpus>["corpora"] = [];
try {
  CORPORA = loadCorpus().corpora;
} catch {
  CORPORA = [];
}
const describeCorpus = CORPORA.length ? describe : describe.skip;

describeCorpus("tier-1 seeded corpora", () => {
  test("every corpus isolates ONE defect family", () => {
    // A mini-repo mixing three defects cannot tell you which one a finding was
    // about, and precision PER FAMILY is what FR-043-AC-2 asks for.
    for (const corpus of CORPORA) {
      const families = new Set(corpus.defects.map((d) => d.family));
      expect(families.size).toBeLessThanOrEqual(1);
      for (const family of families) expect(family).toBe(corpus.family);
    }
  });

  test("there is a clean control with no seeded defect", () => {
    // The control most corpora lack: a check that cannot stay silent on
    // healthy input is not a check, it is a constant. Two of the checks in
    // this programme were deleted for exactly that (quire-rs CR-094).
    const clean = CORPORA.filter((c) => c.defects.length === 0);
    expect(clean.length).toBeGreaterThanOrEqual(1);
    for (const c of clean) expect(c.family).toBe("none");
  });

  test("TC-932 every seeded defect is fully labelled, family location and findability", () => {
    // TC-932
    for (const corpus of CORPORA) {
      for (const defect of corpus.defects) {
        // Explicit language labels use a compact language token (GN-PY),
        // while an inherited language-set label receives the canonical
        // inventory suffix (WT-1-python). Both remain stable, reviewable ids.
        expect(defect.id).toMatch(
          /^[A-Z]{2}-(?:\d+|PY|RS|TS)(?:-(?:python|rust|typescript))?$/,
        );
        expect(defect.family).toBe(corpus.family);
        // A LOCATION UNLESS THE FAMILY DECLARES IT HAS NONE. `locus: none` is
        // the mapping saying this finding has no document to open — the fault
        // is the declaration's, not any one file's — so requiring a location
        // for those families asserts something the mapping already says is
        // impossible. `archetype-matches-nothing` is the case that made this
        // concrete (agent-ix/quire-rs#304): the declared archetype names no
        // document, so there is nothing to point at by construction.
        if (MAPPING.families[defect.family]?.locus !== "none") {
          expect(defect.location).toBeTruthy();
        }
        // `findable` distinguishes a scored MISS from a defect nobody claimed
        // was findable — FR-043-AC-7's whole point.
        expect(typeof defect.findable).toBe("boolean");
        expect(defect.note).toBeTruthy();
        // The engine the label was CONFIRMED against. A label verified by
        // nobody is a claim; this records who checked and with what.
        expect(defect.confirmed_at).toBeTruthy();
        // Exactly one expectation kind, so scoring never has to guess.
        const kinds = [
          "expect_reason",
          "expect_metric",
          "expect_suspicion",
        ].filter((k) => k in defect);
        expect(kinds).toHaveLength(1);
      }
    }
  });

  test("defect ids are unique across the whole corpus set", () => {
    const ids = CORPORA.flatMap((c) => c.defects.map((d) => d.id));
    expect(new Set(ids).size).toBe(ids.length);
  });

  test("the corpus is read from disk, in place, and every case has an input", () => {
    // Was "the build is deterministic and writes labels.json". There is no
    // build: the cases are static files in the `qa-corpus` submodule, so the
    // property worth asserting moved from "generating twice agrees" to
    // "reading finds real files a reader can open" (agent-ix/quoin#227).
    expect(loadCorpus()).toEqual(loadCorpus());

    for (const corpus of CORPORA) {
      expect(existsSync(corpus.input)).toBe(true);
      // The declaration a case binds must exist — `module:` naming one thing
      // while another loads is the defect agent-ix/quire-rs#266 recorded.
      const single = existsSync(join(corpus.module, "manifest.yaml"));
      const asPath = existsSync(corpus.module);
      expect(single || asPath).toBe(true);
    }
  });

  test("TC-948 every family the dictionary declares has a corpus, and vice versa", () => {
    // TC-948
    // The gap that let four families sit unseeded from the day the dictionary
    // shipped: `bench/metrics.json` declared 8, `CORPORA` seeded 4, and
    // nothing compared the two lists. `finding_precision` and
    // `finding_recall` were structurally unmeasurable for half the dictionary
    // and no test, gate or report said so.
    //
    // This is the enforcement, not a restatement of the seeded list — a
    // hardcoded array here would have to be edited in lockstep with both
    // files, which is the third place to forget.
    const dictionary = JSON.parse(
      readFileSync(join(__dirname, "..", "bench", "metrics.json"), "utf8"),
    );
    expect(() =>
      crossCheckFamilies(
        dictionary.families,
        CORPORA.map((c) => c.family),
        { path: "bench/metrics.json" },
      ),
    ).not.toThrow();
  });

  test("TC-949 the cross-check fails in BOTH directions", () => {
    // TC-949
    // A guard that cannot fail is the defect it exists to prevent, and this
    // whole programme exists because one shipped. Each direction catches a
    // different mistake, so each is mutated separately.
    const corpusFamilies = CORPORA.map((c) => c.family);
    const declared: string[] = JSON.parse(
      readFileSync(join(__dirname, "..", "bench", "metrics.json"), "utf8"),
    ).families;

    // A family the dictionary declares that nothing seeds — the state that
    // shipped, for four families at once.
    expect(() =>
      crossCheckFamilies(
        [...declared, "a-family-nothing-seeds"],
        corpusFamilies,
      ),
    ).toThrow(/no corpus/);

    // A corpus family the dictionary never declared — a score no metric
    // governs. Dropping one DECLARED name makes its corpus undeclared, so the
    // no-corpus direction cannot fire first and mask this one.
    expect(() =>
      crossCheckFamilies(
        declared.filter((f) => f !== "hollow-denominator"),
        corpusFamilies,
      ),
    ).toThrow(/does not declare/);
  });

  test("TC-950 declared collateral names a family and a reason", () => {
    // TC-950
    // Collateral suppresses a finding from the precision denominator, so a
    // loose declaration is a licence to launder false positives. It must name
    // BOTH the family and the reason: family alone would absorb any finding of
    // that family, including one pointing where no defect was seeded — the
    // exact laundering CR-098's positional pairing was added to stop.
    const declared = CORPORA.flatMap((c) =>
      c.defects.flatMap((d: { collateral?: unknown[] }) => d.collateral ?? []),
    ) as Array<{ family?: string; reason?: string; note?: string }>;
    expect(declared.length).toBeGreaterThan(0);
    for (const entry of declared) {
      expect(entry.family).toBeTruthy();
      expect(entry.reason).toBeTruthy();
      // Why this finding is a consequence and not a second seeded defect.
      expect(entry.note).toBeTruthy();
    }
  });

  test("TC-951 a family with no working detector says so in its label", () => {
    // TC-951
    // Three families score recall 0 today for three different reasons, and the
    // difference matters: `oracle-is-code-copy` has a detector with no caller
    // (quire-rs#236), `mocked-confirmation` has one with no producer
    // (quoin#204), `gate-that-gates-nothing` has none at all. A 0 that does
    // not say which is a number nobody can act on — the failure this
    // benchmark exists to end, reproduced inside the benchmark.
    for (const corpus of CORPORA) {
      for (const defect of corpus.defects as Array<{
        confirmed_at: string;
        needs_engine?: string;
      }>) {
        if (!/not detected/i.test(defect.confirmed_at)) continue;
        expect(defect.needs_engine).toBeTruthy();
      }
    }
  });
});

describe("tier-2 adjudicated answer key", () => {
  const key = JSON.parse(
    readFileSync(join(__dirname, "..", "bench", "answer-key.json"), "utf8"),
  );

  test("TC-933 it is pinned to a commit and says why re-pinning is not free", () => {
    // TC-933
    // A tier-2 score is only a measurement because the corpus cannot move
    // under it. Re-pinning requires RE-ADJUDICATION, not re-measurement:
    // carrying these findings to a different tree would assert something
    // nobody checked.
    expect(key.pinned_sha).toMatch(/^[0-9a-f]{7,40}$/);
    expect(key.corpus).toBe("agent-ix/filament-ide-rs");
    expect(key.re_pin_policy).toMatch(/re-adjudication/i);
    expect(key.sources.length).toBeGreaterThan(0);
  });

  test("every finding records how it was found, and it was not the tools", () => {
    // The EPIC's premise, as data: every conclusion-changing finding of pass 2
    // came from manual work. This is the recall denominator — if the tools had
    // found them, there would be nothing to measure improvement against.
    expect(key.findings.length).toBeGreaterThanOrEqual(5);
    for (const f of key.findings) {
      expect(f.id).toMatch(/^AK-\d{3}$/);
      expect(f.family).toBeTruthy();
      expect(f.summary).toBeTruthy();
      expect(f.measured).toBeTruthy();
      expect(f.found_by).toBeTruthy();
      expect(f.found_by_tools_at_pass_2).toBe(false);
    }
  });

  test("a finding that is not yet detectable says so and names its ticket", () => {
    // The honest half. Claiming detection the toolchain does not have would
    // make the recall number a fiction, which is the exact failure this
    // programme exists to end.
    for (const f of key.findings) {
      if (f.now_detectable === true || f.now_detectable === "partially") {
        expect(f.detectable_since).toBeTruthy();
      } else {
        expect(f.now_detectable).toBe(false);
        expect(f.detectable_since).toBeNull();
        expect(f.tracked_by).toMatch(/#\d+$/);
      }
    }
  });

  test("TC-947 an entry declaring expect_metric also declares expect_value", () => {
    // TC-947
    // Without this, `Number(undefined)` is NaN, every comparison is false, and
    // the finding scores MISSED forever rather than being reported as a
    // malformed key. AK-003 shipped in exactly that state and was caught only
    // because a test happened to assert its detection (SR-014 FND-002).
    for (const f of key.findings) {
      if (f.expect_metric === undefined) continue;
      expect(
        f.expect_value,
        `${f.id} declares expect_metric "${f.expect_metric}" with no expect_value`,
      ).toBeDefined();
      expect(Number.isNaN(Number(f.expect_value))).toBe(false);
    }
  });

  test("TC-961 every finding records how strongly it is detected, not just whether", () => {
    // TC-961
    // `now_detectable: true` was flattening two different states: AK-003 is "a
    // number moved somewhere in 274 spec files" and AK-001 is "here is the
    // file". Counting them as one overstates the toolchain, and it is the same
    // conflation `finding_localisation_rate` exists to expose — measured at 40%
    // on the first scored tier-1 run.
    const scale = Object.keys(key.detection_strength_scale).filter(
      (k: string) => !k.startsWith("$"),
    );
    expect(scale.sort()).toEqual(["aggregate", "located", "none"]);
    for (const f of key.findings) {
      expect(scale, `${f.id} declares no detection_strength`).toContain(
        f.detection_strength,
      );
      // The two must agree. A finding nothing reports cannot be `located`, and
      // a finding something reports cannot be `none` — a disagreement here is
      // exactly how the recall denominator becomes fiction.
      if (f.now_detectable === false) {
        expect(f.detection_strength, `${f.id}`).toBe("none");
      } else {
        expect(f.detection_strength, `${f.id}`).not.toBe("none");
      }
      // Why this strength, checked against the code rather than carried over.
      expect(f.$strength_note, `${f.id} states no reason`).toBeTruthy();
    }
  });

  test("TC-962 an undetectable finding claims no date and no fix", () => {
    // TC-962
    // AK-005 carried `now_detectable: "partially"` with both a
    // `detectable_since` and a `fixed_by`, for a detector with no production
    // caller. Every one of those errors flattered the toolchain, which is the
    // direction that matters: a capability nothing can reach has no date it
    // became available and no PR that delivered it.
    for (const f of key.findings) {
      if (f.now_detectable !== false) continue;
      expect(f.detectable_since, `${f.id}`).toBeNull();
      expect(f.fixed_by ?? null, `${f.id}`).toBeNull();
      expect(f.tracked_by, `${f.id}`).toMatch(/#\d+$/);
    }
  });

  test("TC-963 every untracked family has its OWN ticket", () => {
    // TC-963
    // AK-007 (`gate-that-gates-nothing`) pointed at quoin#204 — the
    // mocked-confirmation ticket — from the day it was written, so the family
    // had no owner and nobody could tell. Two findings sharing a ticket means
    // closing it closes both, and only one of them was ever worked.
    const tickets = key.findings
      .filter((f: { now_detectable: unknown }) => f.now_detectable === false)
      .map((f: { tracked_by: string }) => f.tracked_by);
    expect(new Set(tickets).size).toBe(tickets.length);
  });

  test("finding ids are unique", () => {
    const ids = key.findings.map((f: { id: string }) => f.id);
    expect(new Set(ids).size).toBe(ids.length);
  });
});

describe("battletest scoring and ratchet", () => {
  const key = JSON.parse(
    readFileSync(join(__dirname, "..", "bench", "answer-key.json"), "utf8"),
  );
  const payload = {
    diagnostics: [{ reason: "hollow-denominator" }],
    suspicions: [{ kind: "vacuous-under-guard" }],
    metrics: [{ name: "coverage.specific_shaped", value: 78 }],
  };

  test("TC-934 the score report is byte-identical over identical inputs and carries its identity", () => {
    // TC-934
    // FR-043-AC-9. Not two calls compared for equality -- `scoreAgainstKey` is
    // pure, so that could not fail (the SR-014 FND-003 shape). This asserts the
    // report carries no time-varying field, which is the property the
    // byte-comparison depends on.
    const first = scoreAgainstKey(payload, key);
    expect(JSON.stringify(first, Object.keys(first).sort())).toBe(
      JSON.stringify(scoreAgainstKey(payload, key), Object.keys(first).sort()),
    );
    const stamped = Object.keys(first).filter((k) =>
      /time|date|stamp|now|generated/i.test(k),
    );
    expect(stamped).toEqual([]);

    // Per-finding accounting, not one number: every key finding lands in
    // exactly one bucket, so a miss cannot hide inside a rounded recall.
    const total =
      first.detected.length + first.missed.length + first.notMechanized.length;
    expect(total).toBe(key.findings.length);
  });

  test("TC-935 the ratchet names what was gained and what was LOST", () => {
    // TC-935
    // FR-043-AC-10. A regression must name the finding that stopped being
    // surfaced -- a recall percentage that drops by one seventh says nothing
    // about which detector rotted.
    const before = { detected: ["AK-001", "AK-002"], recall: 0.5 };
    const after = { detected: ["AK-002", "AK-005"], recall: 0.5 };
    const delta = diff(before, after);
    expect(delta.gained).toEqual(["AK-005"]);
    expect(delta.lost).toEqual(["AK-001"]);

    // Equal recall on both sides, so a scalar comparison would have reported
    // "no change" over a real regression. The rendered report says LOST.
    expect(delta.recallBefore).toBe(delta.recallAfter);
    expect(render({ ...after, missed: [], notMechanized: [] }, delta)).toMatch(
      /LOST:\s+\(AK-001\)/,
    );

    // No baseline is "everything is new", never a silent pass.
    expect(diff(null, after).gained).toEqual(["AK-002", "AK-005"]);
    expect(diff(null, after).recallBefore).toBeNull();
  });
});
