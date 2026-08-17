/**
 * Every command file is a build entry (TC-149).
 *
 * oclif discovers commands as individual modules under `dist/commands`, and
 * `vite.config.ts` enumerates those entries **by hand**. Nothing checked the
 * enumeration against the source tree, so adding a command file and forgetting
 * the entry produced a command that exists in `src/`, passes `tsc`, passes
 * every unit test, ships a `.d.ts` — and does not exist at runtime.
 *
 * Found by dogfooding: `quoin evidence baseline` landed in agent-ix/quoin#105
 * and `quoin advise` in #103, and neither was built. That is the same shape as
 * the defect #103 was filed about — code with full coverage and no reachable
 * path — arriving one layer down.
 */

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { describe, expect, it } from "vitest";

const REPO = join(import.meta.dirname, "..");

/**
 * Files under `src/commands/` that oclif would load as a command.
 *
 * A `default export class` is the test, not the path: `plugin/deprecation.ts`
 * lives there and exports a shared constant, so it is not a command and needs
 * no entry.
 */
function commandFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return commandFiles(path);
    if (!name.endsWith(".ts")) return [];
    return /export default class/.test(readFileSync(path, "utf8"))
      ? [path]
      : [];
  });
}

describe("TC-149 every command file is a vite build entry", () => {
  it("has no command that would be missing from dist/", () => {
    const config = readFileSync(join(REPO, "vite.config.ts"), "utf8");
    const declared = new Set(
      [...config.matchAll(/"(commands\/[\w/-]+)":/g)].map((m) => m[1]),
    );

    const expected = commandFiles(join(REPO, "src", "commands")).map((path) =>
      relative(join(REPO, "src"), path).replace(/\.ts$/, ""),
    );

    const missing = expected.filter((entry) => !declared.has(entry)).sort();
    expect(
      missing,
      `these command files have no vite entry, so they build no module and ` +
        `oclif cannot find them at runtime:\n  ${missing.join("\n  ")}`,
    ).toEqual([]);
  });

  it("declares no entry for a command file that does not exist", () => {
    const config = readFileSync(join(REPO, "vite.config.ts"), "utf8");
    const declared = [
      ...config.matchAll(/"(commands\/[\w/-]+)": "(src\/[\w/.-]+)"/g),
    ].map((m) => m[2]);

    const orphans = declared.filter((src) => {
      try {
        return !statSync(join(REPO, src)).isFile();
      } catch {
        return true;
      }
    });
    expect(orphans).toEqual([]);
  });
});
