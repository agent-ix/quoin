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
 * one. A fixture list is exactly as complete as whoever wrote it — narrowing
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
