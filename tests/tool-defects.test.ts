import { describe, expect, it } from "vitest";

import {
  LEDGER,
  LedgerError,
  assertCited,
  classifyFailure,
  coverage,
  type LedgerEntry,
} from "../src/measurement/tool-defects.js";

const uncited: LedgerEntry = {
  id: "vague",
  repository: "",
  issue: 0,
  effect: "documents-unmeasurable",
  scope: "sometimes it just fails",
  covers: () => true,
  blocks: ["anything"],
  summary: "flaky",
};

describe("TC-1530..1536 the ledger explains only what it declared", () => {
  // TC-1530
  it("refuses an entry that cites no repository and issue, by name", () => {
    expect(() => assertCited([uncited])).toThrow(LedgerError);
    expect(() => assertCited([uncited])).toThrow(/"vague"/);
  });

  // TC-1531
  it("accepts the declared ledger", () => {
    expect(() => assertCited(LEDGER)).not.toThrow();
    for (const e of LEDGER) {
      expect(e.repository).toMatch(/^agent-ix\//);
      expect(e.issue).toBeGreaterThan(0);
    }
  });

  // TC-1532 — the rule that stops the ledger becoming an excuse.
  it("never classifies an undeclared failure as a tool defect", () => {
    expect(classifyFailure(LEDGER, "spec/FR-001.md", "frontmatter")).toBeNull();
    expect(classifyFailure(LEDGER, "spec/tests.md", "frontmatter")).toBeNull();
  });

  // TC-1533
  it("classifies a covered failure with its citation", () => {
    const hit = classifyFailure(LEDGER, "spec/tests.md", "trace-resolution");
    expect(hit?.classification).toBe("tool-defect");
    expect(hit?.citation).toBe("agent-ix/quire-rs#402");
  });

  // TC-1534
  it("publishes the share of the population each entry covers", () => {
    const population = [
      "a/spec/tests.md",
      "b/spec/FR-001.md",
      "c/test/x.test.ts",
      "d/spec/NFR-001.md",
    ];
    const shares = coverage(LEDGER, population);
    const ranges = shares.find((s) => s.entryId === "range-ids-unresolvable");
    expect(ranges?.documents).toBe(1);
    expect(ranges?.share).toBeCloseTo(0.25);

    // The attribution entry covers everything, and says so rather than being
    // silently folded into the aggregate.
    const attribution = shares.find(
      (s) => s.entryId === "pinned-module-set-not-closed",
    );
    expect(attribution?.share).toBe(1);
  });

  // TC-1535
  it("declares an effect on the measurement, not on the tool", () => {
    for (const e of LEDGER) {
      expect([
        "rows-unbindable",
        "documents-unmeasurable",
        "attribution-unproven",
        "check-never-runs",
      ]).toContain(e.effect);
    }
  });
});
