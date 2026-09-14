/**
 * Asking `quoin-core` a measurement question (quoin#478, Stage 6 wiring).
 *
 * The measurement commands and `quoin report` are rewired through the boundary
 * here rather than each repeating the same six lines of exit handling. It is a
 * copy-edit of `src/commands/change-assurance/common.ts`'s `askCore` for the
 * same reason that file states: the two domains are on opposite sides of a
 * retirement, and the measurement half is deleted at the cutover (#479, #480).
 *
 * Every engine refusal reaches the caller as this surface's own exit 2, which
 * is the status the retained commands already exit with on any failure.
 */

import { carriesPayload, runCoreAllowFailure } from "../../core/exec.js";

/**
 * Run one measurement operation and return its payload.
 *
 * `fail` is the command's `this.error`, which never returns.
 */
export function askCore(
  op: string,
  request: unknown,
  fail: (message: string) => never,
): unknown {
  const result = runCoreAllowFailure(op, request);
  if (!carriesPayload(result.exitCode)) {
    const detail = result.diagnostics
      .map((d) => `${d.code}: ${d.message}`)
      .join("\n");
    fail(
      detail ||
        `quoin-core ${op} exited ${result.exitCode} with no diagnostic on stderr.`,
    );
  }
  return result.payload;
}

/**
 * One string member of a payload.
 *
 * The publishing and producing routes answer with `{ path }` and the rendering
 * routes with `{ rendered }`; a payload carrying neither is a boundary fault
 * rather than a user error, and it is named rather than logged as `undefined`.
 */
export function stringMember(
  payload: unknown,
  member: "path" | "rendered",
  op: string,
  fail: (message: string) => never,
): string {
  const value =
    payload !== null && typeof payload === "object"
      ? (payload as Record<string, unknown>)[member]
      : undefined;
  if (typeof value !== "string") {
    fail(`quoin-core ${op} returned no ${member}`);
  }
  return value;
}
