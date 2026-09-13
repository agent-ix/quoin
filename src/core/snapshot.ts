/**
 * The caller's half of `validators.run`: read the repository, send its content.
 *
 * Owned by **FR-096-AC-8** (TC-1713). `quoin-core` does not walk a directory —
 * the boundary carries a file map, not a path — so this walk is the only place
 * in the cutover where a verdict can change without any analysis changing. Its
 * obligation is a superset: every path `quoin-core` would classify must survive
 * the pre-filter below, because a path that never goes on the wire cannot be
 * re-classified on the far side.
 *
 * Two instruments hold it, and they close different axes:
 *
 * - `tests/core-exec-e2e.test.ts` asks the real binary, path by path, which
 *   file NAMES its classifier accepts. That closes the filename axis.
 * - `tests/core-snapshot-differential.test.ts` runs one materialised tree
 *   through both repositories — this snapshot into `MemoryRepo`, and
 *   `quoin-disk-findings` into `DiskRepo` — and compares the findings. That
 *   closes the DIRECTORY axis, which the first cannot: the far side applies
 *   `is_excluded` too, so a directory this walk stops descending into reads as
 *   "the classifier said no" there as well (quoin#448 FND-001).
 */

import { readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";

import type { RunRequest } from "./types.js";

/**
 * Directory names never descended into, at any depth.
 *
 * The same set `quoin-validators` applies, and applied on BOTH sides on
 * purpose: the Rust analysis re-excludes whatever arrives, so this copy is an
 * optimisation of the walk and not a rule only one side knows.
 */
const EXCLUDED = new Set([
  ".git",
  "dist",
  "node_modules",
  "spec",
  "target",
  "vendor",
]);

/**
 * Is this file one `validators.run` could possibly classify?
 *
 * Deliberately COARSER than the Rust rules it stands in front of. `quoin-core`
 * re-derives which files are shell scripts and which are build wiring from the
 * map it receives, so this filter can only ever remove files the analysis would
 * have ignored — it decides nothing. It is therefore written to be an obvious
 * superset (case-insensitive, prefix-matched) rather than a second copy of
 * `is_shell_file` / `is_wiring_file`, which would be a rule in two places that
 * could disagree.
 *
 * The superset property is the only thing standing between a transport
 * optimisation and a changed verdict, so it is MEASURED and not listed:
 * `tests/core-exec-e2e.test.ts` asks the real `quoin-core` binary, path by
 * path, which paths its classifier accepts, and fails if this filter dropped
 * one, and `tests/core-snapshot-differential.test.ts` compares the verdict this
 * walk produces to the one `quoin-core` reaches over the same tree on disk. A fixture list is exactly as complete as whoever wrote it — narrowing
 * the `taskfile` arm below to the literal `taskfile.yml` loses every
 * `Taskfile.yaml` finding, and left the whole suite green before that test
 * existed.
 *
 * Sending the whole tree instead is not an option: quoin's own repository is
 * 1056 files and 26 MB after the exclusions above, and that is a request per
 * `quoin validate`.
 */
function couldMatter(relativePath: string): boolean {
  const name = relativePath.split("/").pop()?.toLowerCase() ?? "";
  return (
    name.endsWith(".sh") ||
    name.startsWith("makefile") ||
    name.startsWith("taskfile") ||
    name === "justfile" ||
    name === "package.json" ||
    relativePath.startsWith(".github/workflows/")
  );
}

function portable(path: string): string {
  return path.split("\\").join("/");
}

/**
 * Read a repository into the request `validators.run` accepts.
 *
 * Three states are recorded because the walk genuinely observes three: a file
 * read, a file found but unreadable (`null`), and a directory that could not be
 * listed. The third aborts the answer on the far side, because a subtree nobody
 * could read means the verdict would be computed over a repository nobody has
 * seen; the second refuses only if the analysis reaches for that file's text.
 *
 * Bodies are sent as lines split on `\n` alone — not `/\r?\n/` — so that
 * joining them with a newline reconstructs the bytes exactly. The far side does
 * its own CRLF handling.
 */
export function repoSnapshot(repo: string): RunRequest {
  const files: Record<string, string[] | null> = {};
  const unlistable: string[] = [];

  const visit = (dir: string): void => {
    let entries;
    try {
      entries = readdirSync(dir, { withFileTypes: true });
    } catch {
      unlistable.push(portable(relative(repo, dir)));
      return;
    }
    for (const entry of entries) {
      const path = join(dir, entry.name);
      if (entry.isDirectory()) {
        if (!EXCLUDED.has(entry.name)) visit(path);
        continue;
      }
      // A symlink is neither a directory nor a file here, which is what keeps
      // the walk from following a cycle out of the repository.
      if (!entry.isFile()) continue;
      const relativePath = portable(relative(repo, path));
      if (!couldMatter(relativePath)) continue;
      try {
        files[relativePath] = readFileSync(path, "utf8").split("\n");
      } catch {
        files[relativePath] = null;
      }
    }
  };

  visit(repo);
  return unlistable.length > 0 ? { files, unlistable } : { files };
}
