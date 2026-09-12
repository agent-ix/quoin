/**
 * `src/core/exec.ts`'s failure contract (quoin#375, FR-096).
 *
 * The equivalent coverage to `tests/quire-exec.test.ts`, for the boundary that
 * replaces it. Four production incidents are encoded in that file and every
 * one of them recurs with any subprocess, so every one is tested here rather
 * than trusted to a copy-edit:
 *
 *   1. executable realpath resolution under `QUOIN_CORE`
 *   2. `QUOIN_EXPECTED_CORE_SHA256` bytes pinning
 *   3. the 64 MiB `maxBuffer` (#164)
 *   4. the three-way termination taxonomy
 *
 * Plus the contract that is quoin-core's own and is the reason the taxonomy
 * has five members: exit 1 carries a complete payload (#103).
 *
 * The fake binary is a `quoin-core` script placed FIRST on PATH (the
 * quire-exec.test.ts pattern), because tests/arch-boundaries.test.ts requires
 * the executed binary to stay the string literal "quoin-core" — the seam is
 * the PATH, not a parameter. The explicit-path cases point `QUOIN_CORE` at the
 * same script.
 */

import { createHash } from "node:crypto";
import {
  chmodSync,
  mkdtempSync,
  readFileSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { afterEach, describe, expect, it } from "vitest";

import {
  CORE_EXIT,
  QUOIN_CORE_MAX_BUFFER,
  carriesPayload,
  quoinCoreExecutable,
  runCore,
  runCoreAllowFailure,
} from "../src/core/index.js";

// Unstructured noise on stderr. Asserting it is ABSENT from kill-path messages
// is the point of the taxonomy: the child's stderr is the diagnosis only when
// the child itself exited.
const NOISE = "warning: reusing a cached module set; first-wins";

function fakeCoreDir(script: string): { dir: string; bin: string } {
  const dir = mkdtempSync(join(tmpdir(), "quoin-fake-core-"));
  const bin = join(dir, "quoin-core");
  writeFileSync(bin, `#!/bin/sh\n${script}\n`);
  chmodSync(bin, 0o755);
  return { dir, bin };
}

const savedPath = process.env.PATH;

afterEach(() => {
  process.env.PATH = savedPath;
  delete process.env.QUOIN_CORE;
  delete process.env.QUOIN_EXPECTED_CORE_SHA256;
});

describe("quoinCoreExecutable — resolution and bytes pinning", () => {
  // Incident 1: executable realpath resolution.
  it("resolves QUOIN_CORE through a symlink to the real path", () => {
    // A governed run must name the bytes it ran, and a symlink names a
    // location. Resolving it here is what lets the digest check below be a
    // statement about the file that actually executed.
    const { dir, bin } = fakeCoreDir("exit 0");
    const link = join(mkdtempSync(join(tmpdir(), "quoin-link-")), "quoin-core");
    symlinkSync(bin, link);
    process.env.QUOIN_CORE = link;
    expect(quoinCoreExecutable()).toBe(bin);
    expect(dir).toBeTruthy();
  });

  it("refuses a relative QUOIN_CORE", () => {
    process.env.QUOIN_CORE = "./quoin-core";
    expect(() => quoinCoreExecutable()).toThrow(/must be an absolute path/);
  });

  it("refuses a QUOIN_CORE that is not executable", () => {
    const dir = mkdtempSync(join(tmpdir(), "quoin-noexec-"));
    const file = join(dir, "quoin-core");
    writeFileSync(file, "not executable");
    process.env.QUOIN_CORE = file;
    expect(() => quoinCoreExecutable()).toThrow(/not an executable file/);
  });

  it("resolves a PATH hit to an absolute real path", () => {
    const { dir, bin } = fakeCoreDir("exit 0");
    process.env.PATH = `${dir}:${savedPath}`;
    expect(quoinCoreExecutable()).toBe(bin);
  });

  // Incident 2: bytes pinning.
  it("accepts the pinned digest of the bytes it resolved", () => {
    const { bin } = fakeCoreDir("exit 0");
    const digest = createHash("sha256").update(readFileSync(bin)).digest("hex");
    process.env.QUOIN_CORE = bin;
    process.env.QUOIN_EXPECTED_CORE_SHA256 = `sha256:${digest}`;
    expect(quoinCoreExecutable()).toBe(bin);
  });

  it("refuses a file swapped in at the pinned path", () => {
    // The incident this exists for: replacing a file at the same path must not
    // be able to move a canonical run.
    const { bin } = fakeCoreDir("exit 0");
    const digest = createHash("sha256").update(readFileSync(bin)).digest("hex");
    process.env.QUOIN_CORE = bin;
    process.env.QUOIN_EXPECTED_CORE_SHA256 = `sha256:${digest}`;
    writeFileSync(bin, "#!/bin/sh\nexit 0\n# different bytes\n");
    chmodSync(bin, 0o755);
    expect(() => quoinCoreExecutable()).toThrow(/digest mismatch/);
  });

  it("refuses a malformed pin rather than ignoring it", () => {
    const { bin } = fakeCoreDir("exit 0");
    process.env.QUOIN_CORE = bin;
    process.env.QUOIN_EXPECTED_CORE_SHA256 = "sha256:deadbeef";
    expect(() => quoinCoreExecutable()).toThrow(/full lowercase sha256 digest/);
  });
});

describe("runCore — the termination taxonomy", () => {
  // Incident 3: the raised maxBuffer, on the success path.
  it("returns a payload larger than Node's 1 MiB default whole", () => {
    // The headline #164 fix, pinned: without `maxBuffer: QUOIN_CORE_MAX_BUFFER`
    // this payload dies ENOBUFS under Node's 1 MiB default — the
    // filament-ide-rs corpus emitted 1,090,714 bytes, 4% over, and all six
    // shelling commands were killed. The kill-path tests below cannot catch a
    // reverted cap (an overrun dies the same way under either limit), so this
    // success path is the one that fails when the raise is lost.
    const size = 2 * 1024 * 1024;
    const { dir } = fakeCoreDir(
      `printf '{"big":"'\nhead -c ${size} /dev/zero | tr '\\0' 'x'\nprintf '"}\\n'`,
    );
    process.env.PATH = `${dir}:${savedPath}`;
    const payload = runCore("core.ping", {}) as { big: string };
    expect(payload.big).toHaveLength(size);
  });

  // Incident 4a: ENOBUFS.
  it("names the buffer overrun on an ENOBUFS death — not an exit status, not the child's stderr", () => {
    const { dir } = fakeCoreDir(
      `echo '${NOISE}' >&2\nhead -c ${QUOIN_CORE_MAX_BUFFER + 1024 * 1024} /dev/zero`,
    );
    process.env.PATH = `${dir}:${savedPath}`;
    let message = "";
    try {
      runCore("core.ping", {});
    } catch (err) {
      message = (err as Error).message;
    }
    expect(message).toContain(`more than ${QUOIN_CORE_MAX_BUFFER} bytes`);
    expect(message).toContain("ENOBUFS");
    expect(message).not.toContain("exited");
    expect(message).not.toContain(NOISE);
  });

  // Incident 4b: signal death.
  it("names the signal on a signal death, and does not append unrelated stderr", () => {
    const { dir } = fakeCoreDir(`echo '${NOISE}' >&2\nkill -TERM $$`);
    process.env.PATH = `${dir}:${savedPath}`;
    let message = "";
    try {
      runCore("core.ping", {});
    } catch (err) {
      message = (err as Error).message;
    }
    expect(message).toContain("killed by SIGTERM");
    expect(message).not.toContain("exited");
    expect(message).not.toContain(NOISE);
  });

  // Incident 4c, first half: nothing to spawn. Because resolution happens
  // before the try (see the comment at the call site), this is now reported by
  // the RESOLVER, which can say which name it looked for — strictly more than
  // "could not be run (ENOENT)" said.
  it("names the binary it could not find on PATH", () => {
    process.env.PATH = mkdtempSync(join(tmpdir(), "quoin-empty-bin-"));
    let error: NodeJS.ErrnoException | undefined;
    try {
      runCore("core.ping", {});
    } catch (err) {
      error = err as NodeJS.ErrnoException;
    }
    expect(error?.message).toContain("quoin-core is not executable on PATH");
    expect(error?.code).toBe("ENOENT");
    expect(error?.message).not.toContain("exited");
  });

  // Incident 4c, second half: resolvable, executable, and still never ran.
  // `status == null`, no signal, an errno — the taxonomy's last branch, and
  // the only true statement available is the cause.
  it("reports a binary that could not be spawned at all, by its cause", () => {
    // Resolvable and executable, with a shebang naming an interpreter that is
    // not there: `execve` fails and no child is ever created. A file with no
    // shebang would NOT do — the kernel returns ENOEXEC and libuv retries it
    // under /bin/sh, which then exits 127, putting it on the "child exited"
    // branch instead.
    const { dir } = fakeCoreDir("");
    writeFileSync(join(dir, "quoin-core"), "#!/nonexistent/interpreter\n");
    chmodSync(join(dir, "quoin-core"), 0o755);
    process.env.PATH = `${dir}:${savedPath}`;
    let message = "";
    try {
      runCore("core.ping", {});
    } catch (err) {
      message = (err as Error).message;
    }
    expect(message).toContain("could not be run");
    expect(message).toContain("ENOENT");
    expect(message).not.toContain("exited");
  });

  // Incident 4d: the child exited — now its stderr IS the diagnosis.
  it("surfaces the child's own diagnostic codes when the child itself exited non-zero", () => {
    const { dir } = fakeCoreDir(
      `echo '[{"code":"CORE_UNKNOWN_OP","message":"no such operation in this build","context":{}}]' >&2\nexit 3`,
    );
    process.env.PATH = `${dir}:${savedPath}`;
    let message = "";
    try {
      runCore("evidence.record", {});
    } catch (err) {
      message = (err as Error).message;
    }
    expect(message).toContain("exited 3");
    expect(message).toContain("CORE_UNKNOWN_OP");
    expect(message).toContain("no such operation in this build");
  });
});

describe("runCoreAllowFailure — non-zero but valid", () => {
  it("returns the payload of an exit-1 run instead of throwing", () => {
    // The #103 lesson: a qualified result is the caller's to judge. Exit 1
    // means the payload is complete AND there is something to say about it.
    const { dir } = fakeCoreDir(
      `echo '{"protocol_version":1}'\necho '[{"code":"CORE_PROTOCOL_SKEW","message":"skew","context":{"actual":"1"}}]' >&2\nexit 1`,
    );
    process.env.PATH = `${dir}:${savedPath}`;
    const result = runCoreAllowFailure("core.ping", {});
    expect(result.exitCode).toBe(CORE_EXIT.PARTIAL);
    expect(result.ok).toBe(false);
    expect(result.payload).toEqual({ protocol_version: 1 });
    expect(result.diagnostics[0].code).toBe("CORE_PROTOCOL_SKEW");
    expect(runCore("core.ping", {})).toEqual({ protocol_version: 1 });
  });

  it("reports a refusal as no payload, not as an empty one", () => {
    const { dir } = fakeCoreDir(
      `echo '[{"code":"CORE_REFUSED","message":"too large","context":{}}]' >&2\nexit 2`,
    );
    process.env.PATH = `${dir}:${savedPath}`;
    const result = runCoreAllowFailure("core.ping", {});
    expect(result.exitCode).toBe(CORE_EXIT.REFUSED);
    expect(result.payload).toBeNull();
    expect(result.diagnostics[0].code).toBe("CORE_REFUSED");
  });

  it("refuses an outcome that promises a payload and writes none", () => {
    // A silent empty payload on exit 0 reads downstream as "no findings",
    // which is the #103 defect wearing the other exit status.
    const { dir } = fakeCoreDir("exit 0");
    process.env.PATH = `${dir}:${savedPath}`;
    expect(() => runCoreAllowFailure("core.ping", {})).toThrow(
      /carries a payload but wrote nothing/,
    );
  });

  it("keeps an unstructured stderr as a diagnostic rather than discarding it", () => {
    const { dir } = fakeCoreDir(`echo 'segmentation fault' >&2\nexit 4`);
    process.env.PATH = `${dir}:${savedPath}`;
    const result = runCoreAllowFailure("core.ping", {});
    expect(result.exitCode).toBe(CORE_EXIT.INTERNAL);
    expect(result.diagnostics[0].code).toBe("CORE_UNSTRUCTURED_STDERR");
    expect(result.diagnostics[0].message).toBe("segmentation fault");
  });

  it("sends the request on stdin", () => {
    const { dir } = fakeCoreDir(`cat`);
    process.env.PATH = `${dir}:${savedPath}`;
    expect(runCore("core.ping", { echo: "corr-7" })).toEqual({
      echo: "corr-7",
    });
  });

  it("propagates a resolution failure instead of reporting 'could not be run'", () => {
    // A digest mismatch has no exit status, no signal and no errno, so if the
    // resolver is called inside the try it falls through the termination
    // taxonomy's last branch and the pinning diagnostic — the one naming the
    // expected and observed digests — is replaced by "(undefined)".
    const { bin } = fakeCoreDir("exit 0");
    process.env.QUOIN_CORE = bin;
    process.env.QUOIN_EXPECTED_CORE_SHA256 = `sha256:${"0".repeat(64)}`;
    expect(() => runCoreAllowFailure("core.ping", {})).toThrow(
      /digest mismatch/,
    );
  });

  it("refuses an operation that is not <domain>.<op> before it reaches argv", () => {
    expect(() => runCoreAllowFailure("--version", {})).toThrow(
      /spelled <domain>\.<op>/,
    );
    expect(() => runCoreAllowFailure("core.ping; rm -rf /", {})).toThrow(
      /spelled <domain>\.<op>/,
    );
  });
});

describe("the exit taxonomy", () => {
  it("is the five documented statuses", () => {
    expect(CORE_EXIT).toEqual({
      OK: 0,
      PARTIAL: 1,
      REFUSED: 2,
      INVALID: 3,
      INTERNAL: 4,
    });
  });

  it("distinguishes non-zero-but-valid from failed", () => {
    expect(carriesPayload(CORE_EXIT.OK)).toBe(true);
    expect(carriesPayload(CORE_EXIT.PARTIAL)).toBe(true);
    for (const dead of [
      CORE_EXIT.REFUSED,
      CORE_EXIT.INVALID,
      CORE_EXIT.INTERNAL,
    ]) {
      expect(carriesPayload(dead)).toBe(false);
    }
  });
});
