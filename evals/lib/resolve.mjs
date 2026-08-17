// Binary + sibling-package resolution for the eval harness.
//
// The harness drives the REAL `claude` agent (via agent-pty) through the quoin
// CLI + skills, so it needs to locate three things that are NOT npm dependencies:
//   - agent-pty's built dist (sibling checkout, dynamically imported)
//   - a `quire` build new enough to support `validate --scope <glob>` (>= 0.2.4)
//   - this repo's own `quoin` bin
// It then builds a shim PATH so the spawned agent's bare `quoin`/`quire`
// commands resolve to exactly those, regardless of global install state.

import { existsSync, mkdirSync, rmSync, symlinkSync, chmodSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { pathToFileURL } from "node:url";

/** quoin repo root (this file lives at evals/lib/resolve.mjs). */
export function repoRoot() {
  return resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
}

export function evalsRoot() {
  return join(repoRoot(), "evals");
}

/** Absolute path to this repo's quoin entry (needs `dist/` built). */
export function ixSpecBin() {
  const bin = join(repoRoot(), "bin", "quoin.js");
  const dist = join(repoRoot(), "dist", "cli.js");
  if (!existsSync(dist)) {
    throw new Error(
      `quoin is not built: ${dist} missing. Run \`make build\` first.`,
    );
  }
  return bin;
}

/**
 * Resolve a `quire` that supports scoped glob validation (`validate --scope`).
 * The authoring pack emits `quire validate --scope <repo> "<glob>"`, which only
 * quire >= 0.2.4 understands; the cargo-bin `quire` on PATH may be older. Prefer
 * the sibling debug/release builds, then PATH, returning the first that advertises
 * `--scope` in `validate --help`.
 */
let _quireCache;
export function findQuire() {
  if (_quireCache) return _quireCache;
  const candidates = [
    resolve(repoRoot(), "..", "quire-cli", "target", "debug", "quire"),
    resolve(repoRoot(), "..", "quire-cli", "target", "release", "quire"),
    "quire",
  ];
  for (const candidate of candidates) {
    if (candidate !== "quire" && !existsSync(candidate)) continue;
    const help = spawnSync(candidate, ["validate", "--help"], {
      encoding: "utf8",
    });
    if (help.status === 0 && /--scope/.test(help.stdout ?? "")) {
      _quireCache = candidate;
      return candidate;
    }
  }
  throw new Error(
    "no `quire` supporting `validate --scope` found (need >= 0.2.4). " +
      "Build quire-cli (`cargo build` in ../quire-cli) or install a newer quire.",
  );
}

/**
 * The quire CLI floor this harness needs, mirroring `QUIRE_CONTRACT.minimumCli`
 * (src/quire/contract.ts, FR-029).
 *
 * Restated here rather than imported because the harness runs against sources,
 * not against `dist/`, and a build step between "run the evals" and "know which
 * quire you need" is a step that gets skipped. `assertQuirePremise` is checked
 * by a unit test against the TypeScript constant, so the two cannot drift
 * silently.
 */
export const HARNESS_MIN_QUIRE = "0.21.0";

/**
 * Fail an eval run early when the `quire` on PATH predates the contract.
 *
 * The harness keys `quire validate` on exit code, which an old binary still
 * produces — so without this an eval against a stale quire fails on the
 * *assertion*, and the reader diagnoses the spec instead of the toolchain.
 */
export function assertQuirePremise(quireBin = findQuire()) {
  const raw = quireVersion(quireBin);
  const found = /(\d+)\.(\d+)\.(\d+)/.exec(raw)?.[0] ?? null;
  const cmp = (a, b) => {
    const pa = a.split(".").map(Number);
    const pb = b.split(".").map(Number);
    for (let i = 0; i < 3; i++) {
      const d = (pa[i] ?? 0) - (pb[i] ?? 0);
      if (d !== 0) return d;
    }
    return 0;
  };
  if (found === null || cmp(found, HARNESS_MIN_QUIRE) < 0) {
    throw new Error(
      `quire ${found ?? "(unknown)"} is older than the ${HARNESS_MIN_QUIRE} this ` +
        `harness requires. Its JSON payloads predate the published schemas, so a ` +
        `failing eval here would be the toolchain rather than the spec under test.`,
    );
  }
  return found;
}

export function quireVersion(quireBin = findQuire()) {
  const out = spawnSync(quireBin, ["--version"], { encoding: "utf8" });
  return (out.stdout ?? "").trim() || "unknown";
}

/** Dynamically import agent-pty from its sibling built dist (never an npm dep). */
export async function findAgentPty() {
  const entry = resolve(repoRoot(), "..", "agent-pty", "dist", "index.js");
  if (!existsSync(entry)) {
    throw new Error(
      `agent-pty build missing at ${entry}. ` +
        "Run `make build` (or `pnpm build`) in ../agent-pty.",
    );
  }
  return { mod: await import(pathToFileURL(entry).href), entry };
}

/**
 * Build a shim bin dir holding `quoin` -> this repo and `quire` -> the resolved
 * build, so the spawned agent's bare commands are pinned. Returns the dir; prepend
 * it to PATH. `claude` and `ix-flow` come from the inherited PATH.
 */
export function shimDir() {
  const dir = join(evalsRoot(), ".bin");
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  link(join(dir, "quoin"), ixSpecBin());
  link(join(dir, "quire"), findQuire());
  return dir;
}

function link(linkPath, target) {
  symlinkSync(target, linkPath);
  try {
    chmodSync(target, 0o755);
  } catch {
    // target may be read-only or already executable; the symlink still resolves.
  }
}

/** PATH string with the shim dir prepended. */
export function binPaths(shim = shimDir()) {
  return `${shim}:${process.env.PATH ?? ""}`;
}
