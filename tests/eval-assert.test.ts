/**
 * FR-038-AC-10 — the eval harness's negative file assertions are falsifiable
 * (TC-270).
 *
 * `globToRegExp` supports `**`, `*` and `?` — not brace expansion. A brace
 * glob like `*fuzz*.{js,ts,mjs}` therefore compiles to a literal-suffix
 * match that can never hit a real file. `fileContains` fails loudly on that
 * ("no file matched"), which is how agent-ix/quoin#135 was found — but
 * `absentFiles` inverts the check: nothing matches, so nothing is "present",
 * so the assertion PASSES whether or not the file it was written to forbid
 * exists. A gate that cannot fire, inside the harness built to catch gates
 * that cannot fire.
 */

import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterAll, describe, expect, it } from "vitest";

// The harness is plain ESM, deliberately importable from vitest so its own
// behaviour is covered by `make test` rather than only by running evals.
import { assertExpectations, matchFiles } from "../evals/lib/assert.mjs";

const repo = mkdtempSync(join(tmpdir(), "quoin-eval-assert-"));
mkdirSync(join(repo, "test"), { recursive: true });
writeFileSync(join(repo, "test", "codes.fuzz.test.js"), "// a fuzz target\n");

afterAll(() => rmSync(repo, { recursive: true, force: true }));

/** The ctx/runResult shape assertExpectations reads for file assertions. */
const ctx = { repo, ixHome: join(repo, ".ix") } as never;
const complete = { exitReason: "complete" } as never;

describe("TC-270 an inexpressible glob is a load error, not a vacuous pass", () => {
  // TC-270
  it("rejects a brace glob instead of compiling it to match nothing", () => {
    // Pre-fix behaviour, kept on record: the brace glob compiled to a regex
    // demanding a literal `{js,ts,mjs}` suffix, matched zero files, and an
    // `absentFiles` assertion over it passed with the forbidden file present.
    expect(() => matchFiles(repo, "**/*fuzz*.{js,ts,mjs}")).toThrowError(
      /brace/,
    );
  });

  // TC-270
  it("fails a scenario carrying a brace glob in absentFiles at load", () => {
    // The scenario author's intent ("no fuzz test of any extension") cannot be
    // expressed — the run must die where the glob is read, not report ok.
    expect(() =>
      assertExpectations(
        ctx,
        { absentFiles: ["**/*fuzz*.{js,ts,mjs}"] },
        complete,
      ),
    ).toThrowError(/\*\*\/\*fuzz\*\.\{js,ts,mjs\}/);
  });

  // TC-270
  it("still fails absentFiles honestly when a supported glob matches", () => {
    // The other half of falsifiability: with the trap removed, the gate must
    // actually fire on the file it forbids.
    const result = assertExpectations(
      ctx,
      { absentFiles: ["**/*fuzz*.test.js"] },
      complete,
    );
    expect(result.ok).toBe(false);
    expect(result.failures.join(" ")).toContain("test/codes.fuzz.test.js");
  });

  // TC-270
  it("keeps supported globs matching, case-insensitively", () => {
    expect(matchFiles(repo, "**/*fuzz*.test.js")).toEqual([
      "test/codes.fuzz.test.js",
    ]);
    expect(matchFiles(repo, "**/CODES.FUZZ.TEST.JS")).toEqual([
      "test/codes.fuzz.test.js",
    ]);
  });
});
