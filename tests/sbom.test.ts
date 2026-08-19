/**
 * FR-041 — SBOM inventories as run evidence (TC-231..TC-236).
 *
 * Both fixtures are real tool output, checked in unedited:
 *
 * - `cyclonedx-real.json` — `@cyclonedx/cyclonedx-npm` 6.0.1 over a real
 *   `npm install`, CycloneDX 1.6.
 * - `spdx-real.json` — GitHub's dependency-graph SBOM for `sindresorhus/slugify`,
 *   SPDX-2.3.
 *
 * A fixture written to match the reader only proves the reader parses itself.
 */

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import {
  parseSbom,
  selectAdapter,
  AdapterError,
} from "../src/evidence/index.js";

const here = dirname(fileURLToPath(import.meta.url));
const fixture = (name: string) =>
  readFileSync(join(here, "fixtures", "evidence", name), "utf8");

describe("the SBOM adapter", () => {
  // Trace: FR-041-AC-1
  it("reads a real CycloneDX document as one entry per component", () => {
    const result = parseSbom(fixture("cyclonedx-real.json"));
    expect(result.entries.length).toBeGreaterThan(0);
    // `purl` is the stable identity, so the symbol is the purl where the
    // document carries one.
    expect(result.entries.every((e) => e.symbol.startsWith("pkg:"))).toBe(true);
    expect(result.entries.every((e) => e.outcome === "pass")).toBe(true);
  });

  // Trace: FR-041-AC-2
  it("reads a real SPDX document from its purl external refs", () => {
    // SPDX puts the purl in `externalRefs[referenceType=purl]`, not in a
    // top-level field — reading `name` alone would give a different identity
    // for the same component depending on which format produced it.
    const result = parseSbom(fixture("spdx-real.json"));
    expect(result.entries.length).toBe(8);
    expect(result.entries.some((e) => e.symbol.startsWith("pkg:npm/"))).toBe(
      true,
    );
  });

  // Trace: FR-041-AC-3
  it("produces no entries for an inventory that lists nothing", () => {
    // The whole reason for one-entry-per-component. An empty SBOM yields zero
    // entries, and `vacuous-evidence` names it with no new machinery — a tool
    // that ran and found nothing is exactly what that check exists to catch.
    // A single entry carrying the count in `score` would make this healthy.
    const empty = JSON.stringify({
      bomFormat: "CycloneDX",
      specVersion: "1.6",
      components: [],
    });
    expect(parseSbom(empty).entries).toEqual([]);
  });

  // Trace: FR-041-AC-4
  it("rejects a document it does not recognise, rather than reading it as empty", () => {
    // Zero entries is a real finding about the consumer's build. A file the
    // adapter simply could not read must not be able to masquerade as one.
    expect(() => parseSbom(JSON.stringify({ hello: "world" }))).toThrow(
      AdapterError,
    );
    expect(() => parseSbom(JSON.stringify({ hello: "world" }))).toThrow(
      /neither a CycloneDX .* nor an SPDX/,
    );
    expect(() => parseSbom("not json at all")).toThrow(/not JSON/);
  });

  // Trace: FR-041-AC-5
  it("drops a component with no identity instead of inventing one", () => {
    // A fabricated symbol binds to nothing AND inflates the count that proves
    // the inventory is not vacuous — it would make an unreadable SBOM look
    // healthier than an empty one.
    const mixed = JSON.stringify({
      bomFormat: "CycloneDX",
      specVersion: "1.6",
      components: [
        { type: "library", purl: "pkg:npm/a@1.0.0" },
        { type: "library", version: "2.0.0" },
        { type: "library", name: "b", version: "2.0.0" },
      ],
    });
    expect(parseSbom(mixed).entries.map((e) => e.symbol)).toEqual([
      "pkg:npm/a@1.0.0",
      "b@2.0.0",
    ]);
  });

  // Trace: FR-041-AC-6
  it("is selected by --adapter and by the tools that emit these formats", () => {
    expect(selectAdapter({ adapter: "sbom" }).name).toBe("sbom");
    expect(selectAdapter({ tool: "syft 1.0.0" }).name).toBe("sbom");
    expect(selectAdapter({ tool: "cyclonedx-npm 6.0.1" }).name).toBe("sbom");
  });
});
