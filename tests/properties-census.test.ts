import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

import {
  CensusError,
  assertSamePopulation,
  conformingShare,
  propertiesCensus,
} from "../src/measurement/properties-census.js";

function doc(dir: string, name: string, body: string): string {
  const path = join(dir, name);
  writeFileSync(path, body);
  return path;
}

const TYPED = [
  "## Properties",
  "",
  "| Field | Type | Multiplicity | Constraints |",
  "|---|---|---|---|",
  "| id | UUID | 1 | |",
  "",
].join("\n");

const LEGACY = [
  "## Properties",
  "",
  "| Name | Notes |",
  "|---|---|",
  "| id | the identifier |",
  "",
].join("\n");

describe("TC-1550..1556 the Properties census states its unit and denominator", () => {
  // TC-1550
  it("counts in documents and says so", () => {
    const dir = mkdtempSync(join(tmpdir(), "census-"));
    const census = propertiesCensus([
      doc(dir, "typed.md", TYPED),
      doc(dir, "legacy.md", LEGACY),
      doc(dir, "plain.md", "# no properties\n"),
    ]);
    expect(census.unit).toBe("documents");
    expect(census.total).toBe(3);
  });

  // TC-1551 — a legacy form is advisory, never a failure.
  it("raises one advisory per legacy-form document and no failure", () => {
    const dir = mkdtempSync(join(tmpdir(), "census-"));
    const census = propertiesCensus([
      doc(dir, "a.md", LEGACY),
      doc(dir, "b.md", LEGACY),
      doc(dir, "c.md", TYPED),
    ]);
    expect(census.advisoryFindings).toBe(2);
    expect(census.rows.filter((r) => r.advisory)).toHaveLength(2);
  });

  // TC-1552 — the denominator excludes not-applicable on both sides.
  it("keeps a document with no Properties heading out of the rate", () => {
    const dir = mkdtempSync(join(tmpdir(), "census-"));
    const census = propertiesCensus([
      doc(dir, "typed.md", TYPED),
      doc(dir, "legacy.md", LEGACY),
      doc(dir, "plain1.md", "# nothing\n"),
      doc(dir, "plain2.md", "# nothing\n"),
    ]);
    expect(census.notApplicable).toBe(2);

    const share = conformingShare(census);
    expect(share.applicable).toBe(2);
    expect(share.conforming).toBe(1);
    expect(share.share).toBeCloseTo(0.5);
    // Not 1/4: counting documents the rule does not reach would halve the rate
    // without measuring anything.
    expect(share.share).not.toBeCloseTo(0.25);
  });

  // TC-1553
  it("records field-level conformance as could-not-run with its citation", () => {
    const dir = mkdtempSync(join(tmpdir(), "census-"));
    const census = propertiesCensus([doc(dir, "typed.md", TYPED)]);
    expect(census.fieldLevelCitation).toBe("agent-ix/quire-rs#392");
    for (const row of census.rows) {
      expect(row.fieldLevel).toBe("could-not-run");
      expect(row.citation).toBe("agent-ix/quire-rs#392");
    }
  });

  // TC-1554
  it("refuses a census population that is not the enumerated one", () => {
    expect(() => assertSamePopulation(["a", "b"], ["a", "b"])).not.toThrow();
    expect(() => assertSamePopulation(["a"], ["a", "b"])).toThrow(CensusError);
    expect(() => assertSamePopulation(["a", "c"], ["a", "b"])).toThrow(
      /diverge at index 1/,
    );
  });

  // TC-1555
  it("returns no share when nothing is applicable, rather than zero", () => {
    const dir = mkdtempSync(join(tmpdir(), "census-"));
    const census = propertiesCensus([doc(dir, "plain.md", "# nothing\n")]);
    // A share of 0 would read as "nothing conforms"; null reads as "the rule
    // reached no document", which is what happened.
    expect(conformingShare(census).share).toBeNull();
  });
});
