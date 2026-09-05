import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

import { resolveModuleSet } from "../src/measurement/modules.js";
import {
  materializeModules,
  runBatch,
  tally,
} from "../src/measurement/engine-run.js";

const REQUIRED = [
  ["spec-objects-business", "d1840b8"],
  ["spec-artifacts-iso", "6686f11"],
  ["spec-artifacts-process", "ccc2bea"],
].map(([name, ref]) => ({
  name,
  repositoryPath: `/home/peter/dev/${name}`,
  ref,
}));

function fixture(body: string): string {
  const dir = mkdtempSync(join(tmpdir(), "engine-"));
  mkdirSync(join(dir, "spec"), { recursive: true });
  writeFileSync(join(dir, "spec", "doc.md"), body);
  return dir;
}

describe("TC-1520..1526 the engine decides, the measurement records", () => {
  const { modules } = resolveModuleSet(REQUIRED);
  const modulesPath = materializeModules(modules);

  // TC-1520
  it("materializes each pinned module from its object store", () => {
    expect(modules).toHaveLength(3);
    expect(modulesPath).toMatch(/pinned-modules-/);
  });

  // TC-1521 — the falsification: a runner that cannot fail measures nothing.
  it("fails a document the engine rejects, and names why", () => {
    const dir = fixture("---\ntype: FR\nid: FR-001\n---\n# no sections\n");
    const evaluations = runBatch({
      scope: dir,
      documents: ["spec/doc.md"],
      modulesPath,
    });
    expect(tally(evaluations)).toEqual({
      pass: 0,
      fail: 1,
      "could-not-run": 0,
    });
    expect(evaluations[0]?.diagnostics.length).toBeGreaterThan(0);
    expect(
      evaluations[0]?.diagnostics.some((d) => d.severity === "error"),
    ).toBe(true);
  });

  // TC-1522 — the perturbation: it must still pass what the engine accepts.
  it("passes a document the engine accepts", () => {
    const evaluations = runBatch({
      scope: "/home/peter/dev/spec-objects-business",
      documents: ["spec/spec.md"],
      modulesPath,
    });
    expect(tally(evaluations)).toEqual({
      pass: 1,
      fail: 0,
      "could-not-run": 0,
    });
  });

  // TC-1523
  it("attributes a diagnostic whose payload omits `path` via its message", () => {
    // The engine emits `path` as a field under some invocations and only as
    // the message prefix under others. Relying on the field alone reported
    // every batch clean.
    const dir = fixture("---\ntype: FR\nid: FR-002\n---\n# no sections\n");
    const [evaluation] = runBatch({
      scope: dir,
      documents: ["spec/doc.md"],
      modulesPath,
    });
    expect(evaluation?.outcome).toBe("fail");
    for (const d of evaluation?.diagnostics ?? []) {
      expect(d.path).not.toBeNull();
    }
  });

  // TC-1524
  it("returns nothing for an empty batch rather than inventing an outcome", () => {
    expect(
      runBatch({ scope: "/home/peter/dev", documents: [], modulesPath }),
    ).toEqual([]);
  });
});
