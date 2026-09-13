/**
 * Running the `quoin-core` subprocess (quoin#373 FR-096, Stage 0 = quoin#375).
 *
 * A deliberate copy-edit of {@link ../quire/exec.ts}, carrying `QUOIN_CORE` /
 * `QUOIN_EXPECTED_CORE_SHA256` where that file carries the QUIRE names. It is
 * a copy rather than a shared generic because the two are on opposite sides of
 * a retirement: `src/quire/exec.ts` is deleted at Stage 8 and this file becomes
 * the only subprocess caller quoin has. Factoring them together now would make
 * that deletion a refactor of live code.
 *
 * Four production incidents are encoded here. Every one of them is a property
 * of running ANY subprocess from Node, not of running quire, so every one
 * recurs at this boundary and is ported rather than rediscovered:
 *
 * 1. **Executable realpath resolution** — the binary is resolved once, to an
 *    absolute real path, before it is executed.
 * 2. **Bytes pinning** — `QUOIN_EXPECTED_CORE_SHA256` hashes the resolved file
 *    on every call, so replacing a file at the same path cannot move a
 *    canonical run.
 * 3. **A 64 MiB `maxBuffer`** — Node's default is 1 MiB and a real payload
 *    already exceeded it (agent-ix/quoin#164).
 * 4. **A three-way termination taxonomy** — the child's stderr is the
 *    diagnosis only when the child itself exited.
 *
 * On top of those, one contract that is quoin-core's own: a non-zero exit can
 * still carry a complete payload (exit 1). See {@link CORE_EXIT}.
 */

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { accessSync, constants, readFileSync, realpathSync } from "node:fs";
import { delimiter, isAbsolute, join } from "node:path";

import type { Diagnostic } from "./types.js";

/**
 * Node's default `maxBuffer` is 1 MiB, and a real corpus already exceeds it:
 * filament-ide-rs (268 spec files, 1,107 obligations) emits 1,090,714 bytes of
 * `quire coverage --json` — 4% over the default — which killed all six
 * commands that shelled out through `src/quire/exec.ts` (#164).
 *
 * The number is inherited rather than re-derived because the ceiling is a
 * property of the payloads, not of the producer: quoin-core will be asked for
 * the same coverage, evidence and measurement documents as the port proceeds,
 * and Stage 6 moves the 7,851-line measurement domain across this pipe. 64 MiB
 * is a cap on what `execFileSync` will accumulate, not an up-front allocation
 * — headroom costs nothing until a payload uses it, while an exceeded cap
 * kills the command outright.
 */
export const QUOIN_CORE_MAX_BUFFER = 64 * 1024 * 1024;

/**
 * The exit taxonomy `quoin-core` terminates with.
 *
 * Mirrors `rust/crates/quoin-core/src/protocol.rs`; the two are asserted equal
 * by the differential harness (`quoin-difftest`). The load-bearing member is
 * `PARTIAL`: a non-zero status whose stdout is nevertheless a complete, valid
 * payload.
 */
export const CORE_EXIT = {
  /** Complete payload, no diagnostics. */
  OK: 0,
  /** Complete payload AND diagnostics. The caller decides what it is worth. */
  PARTIAL: 1,
  /** Understood and refused by a stated rule. No payload. */
  REFUSED: 2,
  /** Not a well-formed request for a known operation. No payload. */
  INVALID: 3,
  /** The boundary itself failed. No payload. */
  INTERNAL: 4,
} as const;

/**
 * Whether an observed exit status means stdout holds a payload.
 *
 * `quire properties` exits 1 when **any** input document fails to resolve —
 * an asset with no `type:`, say — while still writing a complete, valid
 * payload for every document that did. Treating that as total failure threw
 * away the whole property-shape axis over two untyped files, silently
 * (agent-ix/quoin#103). `quoin-core` makes that case a named exit status
 * instead of a convention, and this predicate is how a caller reads it — one
 * function, rather than a `status === 0` comparison repeated at each site,
 * because "non-zero but valid" is exactly the judgement call sites get wrong.
 */
export function carriesPayload(exitCode: number): boolean {
  return exitCode === CORE_EXIT.OK || exitCode === CORE_EXIT.PARTIAL;
}

/**
 * One entry of quoin-core's stderr array.
 *
 * An alias of the GENERATED {@link Diagnostic}, not a second declaration of
 * it. The three fields were hand-written here and happened to match
 * `quoin_core::protocol::Diagnostic` — which is exactly the duplicate FR-097
 * forbids, because a type that agrees by coincidence drifts the first time the
 * Rust struct gains a field and nothing reports it. The name is kept because
 * it is the spelling callers import.
 */
export type CoreDiagnostic = Diagnostic;

/** What one `quoin-core` invocation produced. */
export interface CoreResult {
  /** The parsed payload, or `null` when the outcome carried none. */
  payload: unknown;
  /** The parsed stderr array; empty when the run was clean. */
  diagnostics: CoreDiagnostic[];
  /** The observed exit status. */
  exitCode: number;
  /** `exitCode === 0`. */
  ok: boolean;
}

/**
 * Resolve the producer once per invocation, never once per installation.
 *
 * `QUOIN_CORE` is the governed surface. Ordinary interactive use may still
 * resolve `quoin-core` from PATH, but the result is converted to an absolute
 * real path before execution. When `QUOIN_EXPECTED_CORE_SHA256` is present the
 * exact bytes are checked on every call, so replacing a file at the same path
 * cannot move a canonical run.
 */
export function quoinCoreExecutable(): string {
  const selected = process.env.QUOIN_CORE;
  const executable = selected
    ? resolveExplicitExecutable(selected)
    : resolveOnPath("quoin-core");
  const expected = process.env.QUOIN_EXPECTED_CORE_SHA256;
  if (expected) {
    if (!/^sha256:[0-9a-f]{64}$/.test(expected)) {
      throw new Error(
        "QUOIN_EXPECTED_CORE_SHA256 must be a full lowercase sha256 digest",
      );
    }
    const observed = `sha256:${createHash("sha256")
      .update(readFileSync(executable))
      .digest("hex")}`;
    if (observed !== expected) {
      throw new Error(
        `quoin-core executable digest mismatch: expected ${expected}, observed ${observed} at ${executable}`,
      );
    }
  }
  return executable;
}

function resolveExplicitExecutable(value: string): string {
  if (!isAbsolute(value)) {
    throw new Error(`QUOIN_CORE must be an absolute path, got ${value}`);
  }
  try {
    accessSync(value, constants.X_OK);
    return realpathSync(value);
  } catch {
    throw new Error(`QUOIN_CORE is not an executable file: ${value}`);
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

/** The operation name a caller passed, guarded before it reaches argv. */
function checkOperation(op: string): string {
  if (!/^[a-z][a-z0-9-]*\.[a-z][a-z0-9-]*$/.test(op)) {
    throw new Error(
      `quoin-core operations are spelled <domain>.<op>, got ${JSON.stringify(op)}`,
    );
  }
  return op;
}

/**
 * Run one operation and return its payload, surfacing **stderr** when it fails.
 *
 * Throws for every outcome but `OK` and `PARTIAL`. `PARTIAL` returns the
 * payload; a caller that needs to see the qualifying diagnostics uses
 * {@link runCoreAllowFailure}.
 *
 * `stdio: ["ignore", "pipe", "ignore"]` threw away exactly the sentence the
 * operator needs, one boundary over (agent-ix/quoin#106). stderr is captured
 * here for the same reason — and unlike quire, quoin-core's stderr is
 * structured, so the thrown message names a code rather than echoing prose.
 */
export function runCore(op: string, request: unknown): unknown {
  const result = runCoreAllowFailure(op, request);
  if (carriesPayload(result.exitCode)) {
    return result.payload;
  }
  const codes = result.diagnostics.map((d) => d.code).join(", ");
  const detail = result.diagnostics
    .map((d) => `${d.code}: ${d.message}`)
    .join("\n");
  throw new Error(
    `quoin-core ${op} exited ${result.exitCode}` +
      (codes ? ` (${codes}):\n${detail}` : " with no diagnostic on stderr."),
  );
}

/**
 * As {@link runCore}, but every outcome quoin-core reports is returned rather
 * than thrown — only a run that never produced an exit status throws.
 *
 * The three-way termination taxonomy is the part that must not be
 * re-simplified. `status == null` means the child never exited on its own:
 * Node killed it (ENOBUFS when output outgrew `maxBuffer`), a signal did, or
 * it never spawned. Its stderr is NOT the diagnosis then — appending it framed
 * quire's harmless DuplicateArchetype warnings as the cause of an ENOBUFS
 * death, and the warnings were investigated as the cause (#164).
 */
export function runCoreAllowFailure(op: string, request: unknown): CoreResult {
  checkOperation(op);
  // Resolved BEFORE the try, deliberately. `src/quire/exec.ts` calls
  // `quireExecutable()` inside the try, so a resolution failure — a digest
  // mismatch, a non-absolute QUOIN_QUIRE — lands in the catch, where it has no
  // `status`, no `signal` and no `code`, and is reported as
  // "could not be run (undefined)". The bytes-pinning diagnostic, which names
  // the expected and observed digests, is lost exactly when it is needed.
  // Caught by tests/core-exec-e2e.test.ts against the real binary; the same
  // shape is latent in src/quire/exec.ts and is not this ticket's to change.
  const executable = quoinCoreExecutable();
  const input = JSON.stringify(request ?? {});
  let stdout: string;
  let stderr: string;
  let exitCode: number;
  try {
    stdout = execFileSync(executable, [op], {
      encoding: "utf8",
      input,
      stdio: ["pipe", "pipe", "pipe"],
      maxBuffer: QUOIN_CORE_MAX_BUFFER,
    });
    stderr = "";
    exitCode = CORE_EXIT.OK;
  } catch (cause) {
    const err = cause as {
      stdout?: string | Buffer;
      stderr?: string | Buffer;
      status?: number | null;
      signal?: string | null;
      code?: string;
    };
    if (err.status == null) {
      if (err.code === "ENOBUFS") {
        throw new Error(
          `quoin-core ${op} produced more than ${QUOIN_CORE_MAX_BUFFER} ` +
            `bytes on one stream and was killed (ENOBUFS). The payload has ` +
            `outgrown QUOIN_CORE_MAX_BUFFER; raise it in src/core/exec.ts.`,
        );
      }
      if (err.signal) {
        throw new Error(
          `quoin-core ${op} was killed by ${err.signal} before it could exit.`,
        );
      }
      throw new Error(
        `quoin-core ${op} could not be run (${String(err.code)}).`,
      );
    }
    stdout = String(err.stdout ?? "");
    stderr = String(err.stderr ?? "");
    exitCode = err.status;
  }
  return {
    payload: carriesPayload(exitCode) ? parsePayload(op, stdout) : null,
    diagnostics: parseDiagnostics(op, stderr),
    exitCode,
    ok: exitCode === CORE_EXIT.OK,
  };
}

function parsePayload(op: string, stdout: string): unknown {
  if (stdout.trim() === "") {
    throw new Error(
      `quoin-core ${op} reported an outcome that carries a payload but wrote nothing to stdout.`,
    );
  }
  try {
    return JSON.parse(stdout) as unknown;
  } catch (cause) {
    throw new Error(
      `quoin-core ${op} wrote a payload that is not JSON: ${(cause as Error).message}`,
    );
  }
}

function parseDiagnostics(op: string, stderr: string): CoreDiagnostic[] {
  if (stderr.trim() === "") return [];
  let parsed: unknown;
  try {
    parsed = JSON.parse(stderr) as unknown;
  } catch {
    // An unstructured stderr is still the operator's best sentence; it is
    // reported as one diagnostic rather than discarded. The alternative —
    // throwing — would hide a real diagnosis behind a parse error, which is
    // the #106 defect with the streams swapped.
    return [
      { code: "CORE_UNSTRUCTURED_STDERR", message: stderr.trim(), context: {} },
    ];
  }
  if (!Array.isArray(parsed)) {
    throw new Error(
      `quoin-core ${op} wrote a non-array to stderr; the contract is a JSON array of diagnostics.`,
    );
  }
  return parsed as CoreDiagnostic[];
}

/** `core.ping`'s payload, or `null` when quoin-core is not resolvable. */
export function coreVersion(): unknown {
  try {
    return runCore("core.ping", {});
  } catch {
    return null;
  }
}
