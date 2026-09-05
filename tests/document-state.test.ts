import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

import {
  assertPartition,
  assignDocuments,
  buildVocabulary,
  declaredType,
} from "../src/measurement/document-state.js";

const vocab = buildVocabulary([
  { name: "mod-a", objectTypes: ["Entity"], artifactTypes: ["FR"] },
  { name: "mod-b", objectTypes: [], artifactTypes: ["NFR"] },
]);

function doc(dir: string, name: string, body: string): string {
  const path = join(dir, name);
  writeFileSync(path, body);
  return path;
}

describe("TC-1513..1519 document-state assignment is a total partition", () => {
  // TC-1513
  it("keeps the two out-of-model reasons apart", () => {
    const dir = mkdtempSync(join(tmpdir(), "docs-"));
    const none = doc(dir, "none.md", "# no frontmatter\n");
    const unknown = doc(dir, "unknown.md", "---\ntype: Nope\n---\n# x\n");

    const { assignments } = assignDocuments([none, unknown], vocab);
    expect(assignments[0]?.reason).toBe("no-declared-type");
    expect(assignments[1]?.reason).toBe("type-not-declared-by-any-module");
  });

  // TC-1514
  it("resolves the type case-sensitively", () => {
    const dir = mkdtempSync(join(tmpdir(), "docs-"));
    const lower = doc(dir, "lower.md", "---\ntype: entity\n---\n");
    const exact = doc(dir, "exact.md", "---\ntype: Entity\n---\n");

    const { assignments } = assignDocuments([lower, exact], vocab);
    expect(assignments[0]?.state).toBe("out-of-model");
    expect(assignments[1]?.state).toBe("measured");
  });

  // TC-1515
  it("marks a type two modules declare as contested and raises one finding", () => {
    const shared = buildVocabulary([
      { name: "mod-a", objectTypes: ["Thing"], artifactTypes: [] },
      { name: "mod-b", objectTypes: ["Thing"], artifactTypes: [] },
    ]);
    const dir = mkdtempSync(join(tmpdir(), "docs-"));
    const one = doc(dir, "a.md", "---\ntype: Thing\n---\n");
    const two = doc(dir, "b.md", "---\ntype: Thing\n---\n");

    const { assignments, findings } = assignDocuments([one, two], shared);
    expect(assignments.every((a) => a.state === "contested")).toBe(true);
    expect(findings).toHaveLength(1);
    expect(findings[0]?.detail).toContain("mod-a and mod-b");
  });

  // TC-1516
  it("records an unterminated fence as unreadable and keeps going", () => {
    const dir = mkdtempSync(join(tmpdir(), "docs-"));
    const broken = doc(dir, "broken.md", "---\ntype: FR\nno end fence\n");
    const good = doc(dir, "good.md", "---\ntype: FR\n---\n");

    const { assignments } = assignDocuments([broken, good], vocab);
    expect(assignments[0]?.state).toBe("unreadable");
    expect(assignments[1]?.state).toBe("measured");
  });

  // TC-1517
  it("throws when the states do not sum to the enumerated count", () => {
    const dir = mkdtempSync(join(tmpdir(), "docs-"));
    const one = doc(dir, "a.md", "---\ntype: FR\n---\n");
    const { assignments } = assignDocuments([one], vocab);

    expect(() => assertPartition(assignments, 1)).not.toThrow();
    expect(() => assertPartition(assignments, 2)).toThrow(
      /fell between states and would have left the denominator/,
    );
  });

  // TC-1518
  it("reads no type from a document without frontmatter", () => {
    expect(declaredType("# plain\n")).toBeNull();
    expect(declaredType("---\ntype: FR\n---\n")).toBe("FR");
  });
});
