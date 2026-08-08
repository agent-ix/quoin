// Isolate the "package.json version is missing" guard in packageVersion().
// Mock node:fs so readFileSync returns a package.json without a string version.
vi.mock("node:fs", async () => {
  const actual = await vi.importActual<typeof import("node:fs")>("node:fs");
  return {
    ...actual,
    readFileSync: () => JSON.stringify({ name: "x" }), // no version field
  };
});

import { packageVersion } from "../src/cli";

describe("packageVersion guard", () => {
  // Trace: FR-002-AC-3
  test("throws when package.json has no string version", () => {
    expect(() => packageVersion()).toThrow("package.json version is missing");
  });
});
