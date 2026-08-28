/**
 * Running the `quire` CLI (FR-029).
 *
 * One place, so the subprocess contract — what is captured, what is reported on
 * failure — is a decision made once rather than repeated at each call site.
 */

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { accessSync, constants, readFileSync, realpathSync } from "node:fs";
import { delimiter, isAbsolute, join } from "node:path";

/**
 * Node's default `maxBuffer` is 1 MiB, and a real corpus already exceeds it:
 * filament-ide-rs (268 spec files, 1,107 obligations) emits 1,090,714 bytes of
 * `quire coverage --json` — 4% over the default — which killed all six
 * commands that shell out here (#164). #53 projects ~2.5x payload growth,
 * putting the near-term ceiling around 2.7 MB; 64 MiB is ~25x that projection
 * again, deliberately generous because `maxBuffer` is a cap on what
 * `execFileSync` will accumulate, not an up-front allocation — headroom costs
 * nothing until a payload actually uses it, while an exceeded cap kills the
 * command outright.
 */
export const QUIRE_MAX_BUFFER = 64 * 1024 * 1024;

/**
 * Resolve the producer once per invocation, never once per installation.
 *
 * `QUOIN_QUIRE` is the governed surface used by the verification-stack runner.
 * Ordinary interactive use may still resolve `quire` from PATH, but the result
 * is converted to an absolute real path before execution. When
 * `QUOIN_EXPECTED_QUIRE_SHA256` is present the exact bytes are checked on every
 * call, so replacing a file at the same path cannot move a canonical run.
 */
export function quireExecutable(): string {
  const selected = process.env.QUOIN_QUIRE;
  const executable = selected
    ? resolveExplicitExecutable(selected)
    : resolveOnPath("quire");
  const expected = process.env.QUOIN_EXPECTED_QUIRE_SHA256;
  if (expected) {
    if (!/^sha256:[0-9a-f]{64}$/.test(expected)) {
      throw new Error(
        "QUOIN_EXPECTED_QUIRE_SHA256 must be a full lowercase sha256 digest",
      );
    }
    const observed = `sha256:${createHash("sha256")
      .update(readFileSync(executable))
      .digest("hex")}`;
    if (observed !== expected) {
      throw new Error(
        `quire executable digest mismatch: expected ${expected}, observed ${observed} at ${executable}`,
      );
    }
  }
  return executable;
}

function resolveExplicitExecutable(value: string): string {
  if (!isAbsolute(value)) {
    throw new Error(`QUOIN_QUIRE must be an absolute path, got ${value}`);
  }
  try {
    accessSync(value, constants.X_OK);
    return realpathSync(value);
  } catch {
    throw new Error(`QUOIN_QUIRE is not an executable file: ${value}`);
  }
}

function resolveOnPath(name: string): string {
  const extensions =
    process.platform === "win32"
      ? (process.env.PATHEXT ?? ".EXE;.CMD;.BAT").split(";")
      : [""];
  for (const directory of (process.env.PATH ?? "").split(delimiter)) {
    if (!directory) continue;
    for (const extension of extensions) {
      const candidate = join(directory, `${name}${extension}`);
      try {
        accessSync(candidate, constants.X_OK);
        return realpathSync(candidate);
      } catch {
        // Continue to the next PATH entry.
      }
    }
  }
  throw Object.assign(new Error(`${name} is not executable on PATH`), {
    code: "ENOENT",
  });
}

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
    return execFileSync(quireExecutable(), args, {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
      maxBuffer: QUIRE_MAX_BUFFER,
    });
  } catch (cause) {
    const err = cause as {
      stderr?: string | Buffer;
      status?: number | null;
      signal?: string | null;
      code?: string;
    };
    // `status == null` means the child never exited on its own: Node killed it
    // (ENOBUFS when output outgrew maxBuffer), a signal did, or it never
    // spawned. Its stderr is NOT the diagnosis then — appending it framed
    // quire's harmless DuplicateArchetype warnings as the cause of an ENOBUFS
    // death, and the warnings were investigated as the cause (#164).
    if (err.status == null) {
      if (err.code === "ENOBUFS") {
        throw new Error(
          `quire ${args.join(" ")} produced more than ${QUIRE_MAX_BUFFER} ` +
            `bytes on one stream and was killed (ENOBUFS). The payload has ` +
            `outgrown QUIRE_MAX_BUFFER; raise it in src/quire/exec.ts.`,
        );
      }
      if (err.signal) {
        throw new Error(
          `quire ${args.join(" ")} was killed by ${err.signal} before it could exit.`,
        );
      }
      throw new Error(
        `quire ${args.join(" ")} could not be run (${String(err.code)}).`,
      );
    }
    const stderr = String(err.stderr ?? "").trim();
    throw new Error(
      `quire ${args.join(" ")} exited ${err.status}` +
        (stderr ? `:\n${stderr}` : " with no diagnostic on stderr."),
    );
  }
}

/**
 * As {@link runQuire}, but a non-zero exit returns what was written rather than
 * throwing.
 *
 * `quire properties` exits 1 when **any** input document fails to resolve — an
 * asset with no `type:`, say — while still writing a complete, valid payload
 * for every document that did. Treating that as total failure threw away the
 * whole property-shape axis over two untyped files, silently (agent-ix/quoin#103).
 *
 * The caller decides what a partial result is worth; this only stops the
 * decision being made by an exception.
 */
export function runQuireAllowFailure(args: string[]): {
  stdout: string;
  stderr: string;
  ok: boolean;
} {
  try {
    return {
      stdout: execFileSync(quireExecutable(), args, {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "pipe"],
        maxBuffer: QUIRE_MAX_BUFFER,
      }),
      stderr: "",
      ok: true,
    };
  } catch (cause) {
    const err = cause as { stdout?: string | Buffer; stderr?: string | Buffer };
    return {
      stdout: String(err.stdout ?? ""),
      stderr: String(err.stderr ?? "").trim(),
      ok: false,
    };
  }
}

/** `quire --version` output, or `null` when quire is not on PATH. */
export function quireVersion(): string | null {
  try {
    return execFileSync(quireExecutable(), ["--version"], {
      encoding: "utf8",
      maxBuffer: QUIRE_MAX_BUFFER,
    });
  } catch {
    return null;
  }
}
