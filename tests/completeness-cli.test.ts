/**
 * FR-037-AC-9..AC-12 — the `quoin completeness` COMMAND.
 *
 * Four of FR-037's twelve criteria are stated over the command rather than
 * over the analysis, because the defect this program keeps finding is a matrix
 * reading ✅ over a capability no invocation reaches. The analysis moved to
 * `rust/crates/quoin-completeness/` (quoin#445); `src/commands/completeness.ts`
 * is RETAINED, so these four stay in TypeScript and were lifted out of
 * `tests/completeness.test.ts` when the rest of that file went with the module
 * it tested.
 *
 * The command now reaches the analysis through `src/core/completeness.ts`, so
 * it needs `quoin-core`. The lane is the one `tests/core-exec-e2e.test.ts`
 * established: `QUOIN_CORE` names an executable or the suite skips, and
 * `make rust-e2e` is what sets it.
 */

import {
  accessSync,
  constants,
  mkdirSync,
  mkdtempSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import type { Config } from "@oclif/core";
import { loadConfig } from "@agent-ix/ix-cli-core";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import Completeness from "../src/commands/completeness.js";
import { assessBundle } from "../src/core/completeness.js";
import type { CompletenessFinding } from "../src/core/types.js";

function coreBinary(): string | null {
  const path = process.env.QUOIN_CORE;
  if (!path) return null;
  try {
    accessSync(path, constants.X_OK);
    return path;
  } catch {
    return null;
  }
}

/** A module declaring one vocabulary with a three-value enum. */
function moduleWith(options: { justifiedAbsence?: boolean } = {}): string {
  const root = mkdtempSync(join(tmpdir(), "quoin-vocab-"));
  mkdirSync(join(root, "schemas"), { recursive: true });
  writeFileSync(
    join(root, "schemas", "nfr.schema.json"),
    JSON.stringify({
      type: "object",
      properties: {
        quality_attribute: { enum: ["reliability", "security", "safety"] },
      },
    }),
  );
  writeFileSync(
    join(root, "manifest.yaml"),
    [
      "manifest_version: 1",
      "name: vocab-fixture",
      "version: 0.0.0",
      "artifact_types:",
      "  - name: NFR",
      "    frontmatter_schema_ref: schemas/nfr.schema.json",
      "traceability:",
      "  vocabulary_coverage:",
      "  - name: quality-characteristics",
      "    from: NFR",
      "    field: quality_attribute",
      "    check: unowned-quality-characteristic",
      ...(options.justifiedAbsence === false
        ? []
        : ["    justified_absence_field: quality_attributes_not_applicable"]),
      "",
    ].join("\n"),
  );
  return root;
}

/** A bundle whose documents carry the frontmatter given. */
function bundleWith(documents: Record<string, string>): string {
  const root = mkdtempSync(join(tmpdir(), "quoin-bundle-"));
  const spec = join(root, "spec");
  mkdirSync(spec, { recursive: true });
  for (const [name, content] of Object.entries(documents)) {
    writeFileSync(join(spec, name), content);
  }
  return root;
}

const NFR = (attribute: string) =>
  `---\nid: NFR-001\ntype: NFR\nquality_attribute: ${attribute}\n---\n\n## Statement\n\nIt holds.\n`;

let logged: string[];
let warned: string[];
let config: Config;

beforeAll(async () => {
  // Command.run without an app Config asks @oclif/core to treat its own package
  // as the CLI root. Core's development-only help/plugins entries then emit
  // repeated missing-plugin errors even though the command succeeds (#247).
  const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
  config = await loadConfig({ root: repoRoot });
});

async function runCompleteness(argv: string[]): Promise<unknown> {
  return Completeness.run(argv, config);
}

beforeEach(() => {
  logged = [];
  warned = [];
  vi.spyOn(Completeness.prototype, "log").mockImplementation((m?: string) => {
    logged.push(String(m ?? ""));
  });
  vi.spyOn(Completeness.prototype, "warn").mockImplementation(((m: string) => {
    warned.push(String(m));
    return m;
  }) as never);
});

describe.skipIf(coreBinary() === null)("quoin completeness", () => {
  // Trace: FR-037-AC-9
  it("reports the gaps over a real bundle and exits 0 while advisory", async () => {
    const repo = bundleWith({ "NFR-001-a.md": NFR("reliability") });
    await runCompleteness(["--repo", repo, "--module", moduleWith(), "--json"]);
    const report = JSON.parse(logged.join("\n"));
    expect(report.verdict).toBe("CONDITIONAL");
    expect(report.rollups[0]).toMatchObject({ owned: 1, unowned: 2 });
    expect(report.findings.map((f: CompletenessFinding) => f.value)).toEqual([
      "safety",
      "security",
    ]);
  });

  // Trace: FR-037-AC-10
  it("exits non-zero on an unjustified exclusion, without --strict", async () => {
    const repo = bundleWith({
      "spec.md":
        "---\ntype: Spec\nquality_attributes_not_applicable: [safety]\n---\n\n# Spec\n",
      "NFR-001-a.md": NFR("reliability"),
    });
    await expect(
      runCompleteness(["--repo", repo, "--module", moduleWith()]),
    ).rejects.toMatchObject({ oclif: { exit: 1 } });
    expect(logged.join("\n")).toMatch(/FAIL — 1 high/);
  });

  // Trace: FR-037-AC-11
  it("exits non-zero under --strict on gaps alone", async () => {
    const repo = bundleWith({ "NFR-001-a.md": NFR("reliability") });
    const module = moduleWith();
    // Same bundle, both ways round: advisory passes, strict does not. Asserting
    // the pair is what makes --strict a policy rather than a second code path.
    await runCompleteness(["--repo", repo, "--module", module]);
    expect(logged.join("\n")).toMatch(/CONDITIONAL/);
    await expect(
      runCompleteness(["--repo", repo, "--module", module, "--strict"]),
    ).rejects.toMatchObject({ oclif: { exit: 1 } });
  });

  // Trace: FR-037-AC-12, FR-037-CON-4
  it("says nothing was checked when no module declares a vocabulary", async () => {
    // Not a pass. A repository whose modules declare no coverage has not been
    // checked, and PASS over it is the green-matrix-over-dead-links result.
    const repo = bundleWith({ "NFR-001-a.md": NFR("reliability") });
    const empty = mkdtempSync(join(tmpdir(), "quoin-vocab-none-"));
    writeFileSync(
      join(empty, "manifest.yaml"),
      "manifest_version: 1\nname: empty\n",
    );
    await runCompleteness(["--repo", repo, "--module", empty]);
    expect(warned.join("\n")).toMatch(/nothing was checked/);
    expect(logged.join("\n")).toMatch(/UNCHECKED/);
    expect(logged.join("\n")).not.toMatch(/PASS/);
    // A repository that has not adopted the vocabulary is not broken by
    // installing quoin; one that asked for strict completeness is told it
    // cannot be known.
    await expect(
      runCompleteness(["--repo", repo, "--module", empty, "--strict"]),
    ).rejects.toMatchObject({ oclif: { exit: 1 } });
  });
});

describe.skipIf(coreBinary() === null)("agreement with the engine", () => {
  // Trace: FR-037-AC-13
  //
  // Lifted out of `tests/completeness.test.ts` with the CLI block above, for
  // the same reason: its subject is `assessBundle`, which is RETAINED in
  // `src/core/completeness.ts` as the call that crosses the boundary. Deleting
  // it with the module would have left FR-037-AC-13 asserted nowhere, and the
  // criterion is specifically about two implementations NOT drifting.
  it("counts the same unowned values the bundle read reports", () => {
    // quoin and quire-rs answer the same question from the same declaration by
    // different routes. If they ever disagree, one of them is describing a
    // vocabulary the other is not — the failure TC-183 pins for FR-035.
    const repo = bundleWith({
      "NFR-001-a.md": NFR("reliability"),
      "NFR-002-b.md": NFR("security").replace("NFR-001", "NFR-002"),
    });
    const report = assessBundle({
      bundleRoot: join(repo, "spec"),
      moduleRoots: [moduleWith()],
    });
    expect(report.vocabularies).toEqual(["quality-characteristics"]);
    expect(report.rollups.map((r) => r.owned)).toEqual([2]);
    expect(report.findings.map((f) => f.value)).toEqual(["safety"]);
  });
});
