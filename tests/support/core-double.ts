/**
 * A `quoin-core` that answers named operations from canned payloads.
 *
 * Before quoin#502 the engine was a separate `quire` binary, and a command
 * test that needed a fixed coverage payload put a shell script called `quire`
 * first on `PATH`. The engine is linked now, so there is no second executable
 * to shadow: the request and the answer both belong to `quoin-core`.
 *
 * Faking the whole binary is not an option — one command run asks it four or
 * five questions (the evidence store, the auditor, the catalog) and only one
 * or two of them are being controlled. So the double delegates every other
 * operation to the real boundary, resolved once, before it is installed.
 */

import { chmodSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { quoinCoreExecutable } from "../../src/core/exec.js";

/** What {@link coreDouble} stands in for. */
export interface DoubleOptions {
  /** Operation name → the exact stdout for it. Everything else delegates. */
  answers: Record<string, string>;
  /** Append each intercepted request to this file, one JSON object per line. */
  requestLog?: string;
  /**
   * Refuse every intercepted operation with this message instead of answering.
   *
   * Exit 2 and a diagnostic array, which is what `CORE_REFUSED` looks like on
   * the wire — the refusal path a caller must surface rather than swallow.
   */
  refuseWith?: string;
}

/**
 * Write an executable standing in for `quoin-core` and return its path.
 *
 * The caller installs it by setting `QUOIN_CORE`, and must clear
 * `QUOIN_EXPECTED_CORE_SHA256` if the environment pins one — a double is by
 * definition not the pinned bytes.
 */
export function coreDouble(options: DoubleOptions): string {
  const real = quoinCoreExecutable();
  const dir = mkdtempSync(join(tmpdir(), "quoin-core-double-"));
  const bin = join(dir, "quoin-core");
  writeFileSync(
    bin,
    `#!${process.execPath}
const fs = require("node:fs");
const { spawnSync } = require("node:child_process");
const answers = ${JSON.stringify(options.answers)};
const op = process.argv[2];
const stdin = fs.readFileSync(0, "utf8");
if (!Object.hasOwn(answers, op)) {
  const real = spawnSync(${JSON.stringify(real)}, [op], {
    input: stdin,
    maxBuffer: 64 * 1024 * 1024,
  });
  process.stdout.write(real.stdout ?? "");
  process.stderr.write(real.stderr ?? "");
  process.exit(real.status ?? 4);
}
${
  options.requestLog
    ? `fs.appendFileSync(${JSON.stringify(options.requestLog)}, stdin.trim() + "\\n");`
    : ""
}
${
  options.refuseWith
    ? `process.stderr.write(
  JSON.stringify([
    { code: "CORE_REFUSED", message: ${JSON.stringify(options.refuseWith)}, context: {} },
  ]),
);
process.exit(2);`
    : `process.stdout.write(answers[op]);`
}
`,
  );
  chmodSync(bin, 0o755);
  return bin;
}
