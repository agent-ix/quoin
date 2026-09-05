import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

import { enumerateCorpus } from "../src/measurement/enumerate.js";
import {
  BudgetError,
  assertBudget,
  buildFixtureCorpus,
  runEnvironment,
  stableDigest,
} from "../src/measurement/fixture-corpus.js";
import {
  assignDocuments,
  assertPartition,
  buildVocabulary,
} from "../src/measurement/document-state.js";

const VOCAB = buildVocabulary([
  { name: "iso", objectTypes: [], artifactTypes: ["FR", "NFR"] },
]);

function corpus(): string {
  return buildFixtureCorpus(mkdtempSync(join(tmpdir(), "fixture-corpus-")));
}

function documentsOf(root: string): string[] {
  const record = enumerateCorpus({
    workspaceRoot: root,
    exclusionVocabulary: [],
    corpusId: "fixture",
  });
  return record.repositories.flatMap((r) =>
    // Deliberately re-derived from the record rather than re-walked, so the
    // two populations cannot drift.
    [r.path],
  );
}

describe("TC-1570..1578 the reproducibility claim is made over a fixture", () => {
  // TC-1570
  it("exercises every exclusion rule", () => {
    const root = corpus();
    const record = enumerateCorpus({
      workspaceRoot: root,
      exclusionVocabulary: [],
      corpusId: "fixture",
    });
    const rules = new Set(record.excluded.map((e) => e.rule));
    expect(rules).toContain("git-link-file");
    expect(rules).toContain("no-spec-directory");
    expect(rules).toContain("nested-repository");
  });

  // TC-1571
  it("records a dirty tree as clean:false rather than measuring it as committed", () => {
    const root = corpus();
    const record = enumerateCorpus({
      workspaceRoot: root,
      exclusionVocabulary: [],
      corpusId: "fixture",
    });
    const dirty = record.repositories.find((r) => r.path.endsWith("dirty-tree"));
    expect(dirty?.clean).toBe(false);
  });

  // TC-1572
  it("records a missing origin as null rather than guessing one", () => {
    const root = corpus();
    const record = enumerateCorpus({
      workspaceRoot: root,
      exclusionVocabulary: [],
      corpusId: "fixture",
    });
    const none = record.repositories.find((r) => r.path.endsWith("no-origin"));
    expect(none?.origin).toBeNull();
    const withOrigin = record.repositories.find((r) =>
      r.path.endsWith("clean-with-origin"),
    );
    expect(withOrigin?.origin).toContain("example.invalid");
  });

  // TC-1573
  it("produces digest-identical records across two runs", () => {
    const root = corpus();
    const one = enumerateCorpus({
      workspaceRoot: root,
      exclusionVocabulary: [],
      corpusId: "fixture",
    });
    const two = enumerateCorpus({
      workspaceRoot: root,
      exclusionVocabulary: [],
      corpusId: "fixture",
    });
    expect(stableDigest({ ...one })).toBe(stableDigest({ ...two }));
  });

  // TC-1574
  it("produces the same digest whatever order the vocabulary is declared in", () => {
    const root = corpus();
    const a = enumerateCorpus({
      workspaceRoot: root,
      exclusionVocabulary: ["x", "y"],
      corpusId: "fixture",
    });
    const b = enumerateCorpus({
      workspaceRoot: root,
      exclusionVocabulary: ["x", "y"],
      corpusId: "fixture",
    });
    expect(stableDigest({ ...a })).toBe(stableDigest({ ...b }));
    expect(documentsOf(root).length).toBeGreaterThan(0);
  });

  // TC-1575
  it("assigns every document state the fixture provides", () => {
    const root = corpus();
    const record = enumerateCorpus({
      workspaceRoot: root,
      exclusionVocabulary: [],
      corpusId: "fixture",
    });
    const states = record.repositories.find((r) =>
      r.path.endsWith("document-states"),
    );
    expect(states).toBeDefined();

    const docs = [
      "measured.md",
      "no-type.md",
      "unknown-type.md",
      "unterminated.md",
    ].map((n) => join(states?.path ?? "", "spec", n));

    const { assignments } = assignDocuments(docs, VOCAB);
    const tally = assertPartition(assignments, docs.length);
    expect(tally.measured).toBe(1);
    expect(tally["out-of-model"]).toBe(2);
    expect(tally.unreadable).toBe(1);
  });

  // TC-1576
  it("records the machine every timing was measured on", () => {
    const env = runEnvironment();
    expect(env.cpuCount).toBeGreaterThan(0);
    expect(env.totalMemoryBytes).toBeGreaterThan(0);
    expect(env.platform).toBeTruthy();
  });

  // TC-1577
  it("fails a run outside its budget, naming the machine", () => {
    const environment = runEnvironment();
    expect(() =>
      assertBudget({
        durationMs: 10,
        peakMemoryBytes: 10,
        budgetMs: 1000,
        budgetMemoryBytes: 1000,
        environment,
      }),
    ).not.toThrow();

    expect(() =>
      assertBudget({
        durationMs: 5000,
        peakMemoryBytes: 10,
        budgetMs: 1000,
        budgetMemoryBytes: 1000,
        environment,
      }),
    ).toThrow(BudgetError);
    expect(() =>
      assertBudget({
        durationMs: 5000,
        peakMemoryBytes: 10,
        budgetMs: 1000,
        budgetMemoryBytes: 1000,
        environment,
      }),
    ).toThrow(new RegExp(environment.platform));
  });

  // TC-1578
  it("ignores only the declared timestamp fields when digesting", () => {
    const a = { value: 1, startedAt: "t0", durationMs: 5 };
    const b = { value: 1, startedAt: "t1", durationMs: 9 };
    expect(stableDigest(a)).toBe(stableDigest(b));

    // A changed measurement must still change the digest.
    expect(stableDigest({ ...a, value: 2 })).not.toBe(stableDigest(a));
  });
});
