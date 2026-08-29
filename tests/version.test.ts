import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

import { packageVersion, resolveVersion } from "../src/cli";
import { resolveBuildRevision } from "../vite.config";

describe("resolveVersion", () => {
  it("returns the baked version verbatim when present", () => {
    // A clean tagged release bakes a bare semver...
    expect(resolveVersion("0.5.2")).toBe("0.5.2");
    // ...and a drifted build bakes a describe string with a suffix.
    expect(resolveVersion("0.5.1-3-gabc123-dirty")).toBe(
      "0.5.1-3-gabc123-dirty",
    );
  });

  it("falls back to the package.json version when nothing is baked in", () => {
    const pkg = JSON.parse(
      readFileSync(new URL("../package.json", import.meta.url), "utf8"),
    ) as { version: string };
    // Under vitest __QUOIN_VERSION__ is "", so the fallback reads package.json.
    expect(resolveVersion("")).toBe(pkg.version);
    expect(packageVersion()).toBe(pkg.version);
  });
});

describe("resolveBuildRevision", () => {
  it("uses HEAD outside canonical verification", () => {
    expect(resolveBuildRevision(undefined)).toBe("HEAD");
  });

  it("uses an exact locked code revision for canonical evidence builds", () => {
    const revision = "1842e3886bae7982cb81e662d2b69cb86b6f6a89";
    expect(resolveBuildRevision(revision)).toBe(revision);
  });

  it.each([
    "1842e3886bae7982cb81e662d2b69cb86b6f6a8",
    "1842E3886BAE7982CB81E662D2B69CB86B6F6A89",
    "main",
    "",
  ])("rejects mutable or malformed locked revisions (%j)", (revision) => {
    expect(() => resolveBuildRevision(revision)).toThrow(
      "must be a full lowercase Git SHA",
    );
  });
});
