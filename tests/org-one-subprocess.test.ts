/**
 * FR-025-AC-7: resolving the organization executes exactly one subprocess, and
 * it is `quoin-core`.
 *
 * The successor to `tests/org-no-subprocess.test.ts`, deleted with `src/org.ts`
 * in quoin#446. The criterion it carried read "Resolution executes no
 * subprocess", which the cutover made false, so the criterion was amended to
 * the property it was actually written against: resolution must not shell out
 * to `git`, and must therefore hold on a host with no Git executable at all
 * (NFR-004). Amending a criterion and deleting its only test in the same change
 * is how a requirement quietly stops being checked; this file is why that did
 * not happen here.
 *
 * Asserting it this way, rather than by emptying `PATH`: an empty `PATH` proves
 * nothing, because `execFileSync("/usr/bin/git", …)` succeeds with an empty
 * `PATH` and would pass such a test unchanged. Every `node:child_process` entry
 * point is replaced — the one `src/core/exec.ts` uses with a recorder that
 * still runs the real call, so the resolution under test is a real one, and the
 * other six with recorders that throw. Any attempt to shell out, by absolute
 * path or otherwise, is caught and named.
 *
 * This lives in its own file because `vi.mock` applies to a whole module graph,
 * and nothing else in this suite should run against a stubbed
 * `node:child_process`.
 */

import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";

const calls: string[] = [];

vi.mock("node:child_process", async (importOriginal) => {
  const actual = await importOriginal<typeof import("node:child_process")>();
  const refuse =
    (name: string) =>
    (...args: unknown[]) => {
      calls.push(`${name}(${String(args[0])})`);
      throw new Error(`unexpected subprocess: ${name}(${String(args[0])})`);
    };
  return {
    ...actual,
    // Recorded and then run for real: the assertion is about *which* program
    // was spawned and how many times, and a stub that refused would only prove
    // that a refused resolution resolves nothing.
    execFileSync: (...args: Parameters<typeof actual.execFileSync>) => {
      calls.push(`execFileSync(${String(args[0])})`);
      return actual.execFileSync(...args);
    },
    execSync: refuse("execSync"),
    spawnSync: refuse("spawnSync"),
    exec: refuse("exec"),
    execFile: refuse("execFile"),
    spawn: refuse("spawn"),
    fork: refuse("fork"),
  };
});

const { quoinCoreExecutable } = await import("../src/core/exec.js");
const { resolveOrg } = await import("../src/core/org.js");

function boundaryAvailable(): boolean {
  try {
    quoinCoreExecutable();
    return true;
  } catch {
    return false;
  }
}

/** A repo root whose `origin` remote names `acme`. */
function repoWithRemote(): string {
  const root = mkdtempSync(join(tmpdir(), "quoin-org-onesub-"));
  mkdirSync(join(root, ".git"), { recursive: true });
  writeFileSync(
    join(root, ".git", "config"),
    '[core]\n\trepositoryformatversion = 0\n[remote "origin"]\n\turl = git@github.com:acme/widgets.git\n',
  );
  return root;
}

/** The programs `calls` recorded, by basename. */
function programs(): string[] {
  return calls.map((call) => basename(call.replace(/^[a-zA-Z]+\(|\)$/g, "")));
}

describe.skipIf(!boundaryAvailable())(
  "resolving an org shells out to quoin-core and to nothing else",
  () => {
    const priorHome = process.env.XDG_CONFIG_HOME;
    const priorOrg = process.env.QUOIN_ORG;

    beforeEach(() => {
      calls.length = 0;
      // An org in the developer's own user config would resolve from `config`
      // and never reach the git layer, so the case below would assert the count
      // over a resolution that never happened.
      process.env.XDG_CONFIG_HOME = mkdtempSync(
        join(tmpdir(), "quoin-org-onesub-home-"),
      );
      delete process.env.QUOIN_ORG;
    });

    afterEach(() => {
      if (priorHome === undefined) delete process.env.XDG_CONFIG_HOME;
      else process.env.XDG_CONFIG_HOME = priorHome;
      if (priorOrg === undefined) delete process.env.QUOIN_ORG;
      else process.env.QUOIN_ORG = priorOrg;
    });

    // Trace: FR-025-AC-7
    test("the git remote is read without invoking git", () => {
      expect(
        resolveOrg(repoWithRemote(), { projectConfigEnabled: false }),
      ).toEqual({ org: "acme", source: "git", degraded: false });

      // The whole criterion, in two lines: one child process, and it is the
      // boundary. `git` never appears, which is what makes resolution work on a
      // host that has none.
      expect(programs()).toEqual(["quoin-core"]);
      expect(calls.join("\n")).not.toMatch(/\bgit\b/);
    });

    // Trace: FR-025-AC-7
    test("an unresolved org costs the same one subprocess", () => {
      const root = mkdtempSync(join(tmpdir(), "quoin-org-onesub-empty-"));
      expect(resolveOrg(root, { projectConfigEnabled: false })).toEqual({
        source: "none",
        degraded: false,
      });
      expect(programs()).toEqual(["quoin-core"]);
    });
  },
);
