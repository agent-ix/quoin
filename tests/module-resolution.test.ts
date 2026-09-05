import { describe, expect, it } from "vitest";

import {
  resolveModuleSet,
  showAt,
  sha256,
} from "../src/measurement/modules.js";

const REQUIRED = [
  ["spec-objects-business", "d1840b8"],
  ["spec-artifacts-iso", "6686f11"],
  ["spec-objects-enterprise", "9c230e6"],
  ["spec-objects-safety", "0c8d42e"],
  ["spec-objects-architecture", "99e5f62"],
  ["spec-objects-operational", "95efb69"],
  ["spec-artifacts-app", "cbeaa52"],
  ["spec-objects-security", "f9e7f7b"],
  ["spec-artifacts-process", "ccc2bea"],
].map(([name, ref]) => ({
  name,
  repositoryPath: `/home/peter/dev/${name}`,
  ref,
}));

describe("TC-1506..1512 module-set resolution reads the object store", () => {
  // TC-1506
  it("resolves every required module and records ref beside commit", () => {
    const { modules } = resolveModuleSet(REQUIRED);
    expect(modules).toHaveLength(9);
    for (const m of modules) {
      expect(m.commit).toMatch(/^[0-9a-f]{40}$/);
      expect(m.commit.startsWith(m.ref)).toBe(true);
      expect(m.manifestVersion).toMatch(/^\d+\.\d+\.\d+$/);
    }
  });

  // TC-1507
  it("finds no data_schema digest mismatch across the merged wave", () => {
    const { findings } = resolveModuleSet(REQUIRED);
    expect(findings.filter((f) => f.kind === "data-schema-digest-mismatch")).toEqual(
      [],
    );
  });

  // TC-1508
  it("does not trim the bytes a declared digest was computed over", () => {
    // The regression this pins: trimming the trailing newline reported every
    // schema in the ecosystem as a mismatch. The digest must be over the file.
    const body = showAt(
      "/home/peter/dev/spec-objects-business",
      "d1840b8",
      "spec_objects_business/schemas/Domain.json",
    );
    expect(body).not.toBeNull();
    expect(body?.endsWith("\n")).toBe(true);
    expect(sha256(body as string)).toBe(
      "sha256:e59103a3718f19aa7b1c9fadabd409cd776776c2ec968b6a93e62d3db9da484c",
    );
    expect(sha256((body as string).trim())).not.toBe(
      "sha256:e59103a3718f19aa7b1c9fadabd409cd776776c2ec968b6a93e62d3db9da484c",
    );
  });

  // TC-1509
  it("records mappings as absent without dropping the module's types", () => {
    const { modules } = resolveModuleSet(REQUIRED);
    const business = modules.find((m) => m.name === "spec-objects-business");
    expect(business?.mappingsDigest).toBeNull();
    expect(business?.objectTypes).toHaveLength(10);
  });

  // TC-1510
  it("refuses the whole set when a required member does not resolve", () => {
    expect(() =>
      resolveModuleSet([
        ...REQUIRED,
        {
          name: "not-a-module",
          repositoryPath: "/home/peter/dev/spec-objects-business",
          ref: "0000000000000000000000000000000000000000",
        },
      ]),
    ).toThrow(/required modules did not resolve/);
  });
});
