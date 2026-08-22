/**
 * The tier-1 seeded corpora and their labels (quoin#199, FR-043-AC-7).
 *
 * A label nobody checks is a claim, not ground truth. These tests assert the
 * SHAPE of the label set — the part that makes precision and recall computable
 * — and the build's determinism. Whether each seeded defect is actually found
 * is verified against a real engine and recorded per defect in `confirmed_at`;
 * doing it here would make the unit suite depend on a `quire` binary.
 */

import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { buildBenchCorpora, CORPORA } from "../evals/fixtures/bench/build.mjs";

function build() {
  const root = mkdtempSync(join(tmpdir(), "quoin-bench-"));
  const labels = buildBenchCorpora(root);
  return { root, labels };
}

describe("tier-1 seeded corpora", () => {
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

  test("every seeded defect is fully labelled", () => {
    for (const corpus of CORPORA) {
      for (const defect of corpus.defects) {
        expect(defect.id).toMatch(/^[A-Z]{2}-\d+$/);
        expect(defect.family).toBe(corpus.family);
        expect(defect.location).toBeTruthy();
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

  test("the build is deterministic and writes labels.json", () => {
    const a = build();
    const b = build();
    try {
      expect(a.labels).toEqual(b.labels);
      const onDisk = JSON.parse(
        readFileSync(join(a.root, "labels.json"), "utf8"),
      );
      expect(onDisk).toEqual(a.labels);
      // Every corpus materializes its module, spec and source.
      for (const corpus of CORPORA) {
        for (const rel of Object.keys(corpus.files)) {
          expect(
            readFileSync(join(a.root, corpus.name, rel), "utf8").length,
          ).toBeGreaterThan(0);
        }
      }
    } finally {
      rmSync(a.root, { recursive: true, force: true });
      rmSync(b.root, { recursive: true, force: true });
    }
  });

  test("every named battletest failure family has a corpus", () => {
    // The families #197 enumerates. `stale-name-correct-trace` and
    // `oracle=code-copy` are deliberately absent here and covered as
    // declarative cases in quire-rs (tests/fixtures/corpus_cases), where the
    // engine can be driven directly — duplicating them would give two places
    // to update and one to forget.
    const covered = new Set(CORPORA.map((c) => c.family));
    for (const family of [
      "marker-form-mismatch",
      "undeclared-type-value",
      "catch-all-universal",
      "vacuous-under-guard",
    ]) {
      expect(covered).toContain(family);
    }
  });
});

describe("tier-2 adjudicated answer key", () => {
  const key = JSON.parse(
    readFileSync(join(__dirname, "..", "bench", "answer-key.json"), "utf8"),
  );

  test("it is pinned to a commit and says why re-pinning is not free", () => {
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

  test("finding ids are unique", () => {
    const ids = key.findings.map((f: { id: string }) => f.id);
    expect(new Set(ids).size).toBe(ids.length);
  });
});
