/**
 * Running the `quire` CLI (FR-029).
 *
 * One place, so the subprocess contract — what is captured, what is reported on
 * failure — is a decision made once rather than repeated at each call site.
 */

import { execFileSync } from "node:child_process";

/**
 * Run `quire` and return stdout, surfacing **stderr** when it fails.
 *
 * `stdio: ["ignore", "pipe", "ignore"]` threw away exactly the sentence the
 * operator needs. Reproduced (agent-ix/quoin#106):
 *
 * ```
 * $ quire coverage --scope . --json
 * no module in scope declares a `traceability:` model, so there is nothing to
 * reconcile; install a module that declares one (e.g. spec-artifacts-process)
 * or pass --module
 * ```
 *
 * `src/quire/contract.ts` goes to real trouble over the version-premise
 * diagnostic for precisely this reason; discarding the subprocess's own
 * diagnostic undid it one frame later.
 */
export function runQuire(args: string[]): string {
  try {
    return execFileSync("quire", args, {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
  } catch (cause) {
    const err = cause as { stderr?: string | Buffer; status?: number };
    const stderr = String(err.stderr ?? "").trim();
    throw new Error(
      `quire ${args.join(" ")} exited ${err.status ?? "abnormally"}` +
        (stderr ? `:\n${stderr}` : " with no diagnostic on stderr."),
    );
  }
}

/** `quire --version` output, or `null` when quire is not on PATH. */
export function quireVersion(): string | null {
  try {
    return execFileSync("quire", ["--version"], { encoding: "utf8" });
  } catch {
    return null;
  }
}
