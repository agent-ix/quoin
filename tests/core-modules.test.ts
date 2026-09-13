/**
 * The printed key order of a module record, pinned against the registry file
 * that produced it (quoin#446).
 *
 * `quoin module list --json` and `quoin module install --json` print their
 * records with `JSON.stringify`, so the **key order** of the object handed to
 * them is user-visible output. Before the cutover that object was parsed
 * straight out of `registry.json` and carried the file's own field order. The
 * boundary emits canonical JSON — keys sorted at every depth — so a payload
 * parsed from its stdout carries sorted keys instead, which would have changed
 * the output of both commands without changing a single value.
 * `src/core/modules.ts` restores the order; this is what proves it.
 *
 * The oracle is the real `registry.json` written by a real install, not the
 * hand-written list in `src/core/modules.ts`. A field added to
 * `quoin_modules::InstalledModule`, or a field reordered in it, therefore fails
 * here rather than being silently dropped from the printed record — a list
 * checked against itself would have proved nothing.
 *
 * Needs the binary, so it skips cleanly without one, in the same lane as
 * `tests/core-exec-e2e.test.ts` and `tests/core-org.test.ts` (`make rust-e2e`).
 */

import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { quoinCoreExecutable } from "../src/core/exec.js";
import { installModule, listModules } from "../src/core/modules.js";

function boundaryAvailable(): boolean {
  try {
    quoinCoreExecutable();
    return true;
  } catch {
    return false;
  }
}

const available = boundaryAvailable();

const FIXTURE = join("tests", "fixtures", "semantic-module", "module-ok");

/** Every key of `value`, in the order the object holds them, at every depth. */
function keyShape(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(keyShape);
  if (value === null || typeof value !== "object") return null;
  return Object.entries(value as Record<string, unknown>).map(([key, item]) => [
    key,
    keyShape(item),
  ]);
}

describe.skipIf(!available)("the printed module record's key order", () => {
  // Trace: FR-019-AC-1
  it("is the registry file's own order, at every depth", () => {
    const home = mkdtempSync(join(tmpdir(), "quoin-core-modules-"));
    const installed = installModule(`path:${FIXTURE}`, home);

    const registry = JSON.parse(
      readFileSync(join(home, "filament", "registry.json"), "utf8"),
    ) as { plugins: unknown[] };
    // Not an empty population: an install that wrote no record would make every
    // assertion below vacuous, and this is the check that says so.
    expect(registry.plugins).toHaveLength(1);
    const onDisk = registry.plugins[0];

    // Nested too, and not just the top level: `source` is an internally tagged
    // enum whose `type` leads, and canonical JSON sorts it below `sha`.
    expect(keyShape(installed)).toEqual(keyShape(onDisk));
    expect(keyShape(listModules(home))).toEqual(keyShape([onDisk]));

    // The order is the point, but a reordering that dropped a field would pass
    // a shape comparison run over two equally-mangled sides, so the values are
    // pinned as well.
    expect(installed).toEqual(onDisk);
  });
});
