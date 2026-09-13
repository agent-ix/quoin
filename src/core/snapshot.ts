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

import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

import { locateModuleRoot } from "../module-roots.js";

import type {
  DocumentSource,
  ModuleSource,
  RunRequest,
  SchemaSource,
  UnreadableDocument,
} from "./types.js";

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

/**
 * Read a bundle into the `documents` / `unreadable` pair the completeness and
 * assurance operations accept (quoin#445).
 *
 * The same shape decision as {@link repoSnapshot} and for the same reason:
 * `quoin-core`'s library half is audited as a reusable library, so an operation
 * that took `bundleRoot` and walked it would acquire a host capability the
 * boundary has said it does not have. The walk is the command shell's job; the
 * decision crosses as bytes.
 *
 * Unlike `repoSnapshot` there is no pre-filter beyond the `.md` extension,
 * because there is nothing coarser to be: the far side needs the leading `---`
 * block of every markdown document in the bundle, and "which of these carries
 * frontmatter" is exactly the question it answers.
 *
 * Three states, as the walk genuinely observes three: a document read, a
 * document found and unreadable (recorded in `unreadable`, with the OS reason),
 * and a root that could not be listed at all — which reads as an EMPTY bundle
 * rather than an error, because the retained reader did the same and the
 * command prints the root it looked in, so an absent bundle stays legible.
 */
export function bundleSnapshot(bundleRoot: string): {
  documents: DocumentSource[];
  unreadable: UnreadableDocument[];
} {
  const documents: DocumentSource[] = [];
  const unreadable: UnreadableDocument[] = [];

  // Recursion written out rather than `readdirSync(…, { recursive: true })`,
  // because which entries a recursive listing yields is Node's decision and the
  // far side's is `quoin-completeness`'. Node descends a SYMLINKED directory;
  // the Rust walk, reading `DirEntry::file_type()`, does not — so a bundle
  // holding one link put documents on the wire that no reader of the same tree
  // finds, and the two sides disagreed about the size of the bundle. Descent is
  // therefore decided here, in the same three cases the disk reader states:
  //
  // - a REAL directory is descended. `withFileTypes`, because a directory may
  //   be named `notes.md`: filtering on the extension alone opened one and
  //   recorded `EISDIR` as an unreadable document the bundle does not contain,
  //   and a snapshot that ADDS an entry the tree does not hold is the same
  //   class of defect as one that drops an entry it does.
  // - a SYMLINK is a document exactly when it RESOLVES to a file. `Dirent`
  //   reports the link's own type, so `isFile()` alone is false for every
  //   symlink and drops one silently — and `spec/FR-042.md` symlinked into a
  //   shared spec directory is the ordinary monorepo layout. A document that
  //   does not cross is not merely absent from the request, it is absent from
  //   the VERDICT: the value it owned reads `unowned` and a finding appears
  //   that a direct disk read never produces. A link resolving to a DIRECTORY
  //   is neither a document nor descended, which is also what keeps a cycle
  //   from leaving the bundle; a BROKEN one resolves to nothing and has no
  //   bytes, so recording it `unreadable` would invent an entry too.
  // - anything else (a fifo, a socket) is not a document.
  //
  // `tests/core-bundle-snapshot.test.ts` pins every direction of this against
  // an independent walk of one real tree.
  const found: string[] = [];
  const walk = (prefix: string): void => {
    let entries;
    try {
      entries = readdirSync(join(bundleRoot, prefix), { withFileTypes: true });
    } catch {
      // A directory that cannot be listed contributes nothing and stops
      // nothing, as the disk reader's `read_dir` does — an unreadable subtree
      // must not turn the rest of the bundle into an empty one.
      return;
    }
    for (const entry of entries) {
      const relativePath = prefix ? `${prefix}/${entry.name}` : entry.name;
      if (entry.isDirectory()) {
        walk(relativePath);
        continue;
      }
      if (!entry.name.endsWith(".md")) continue;
      if (entry.isFile()) {
        found.push(relativePath);
        continue;
      }
      if (!entry.isSymbolicLink()) continue;
      try {
        if (statSync(join(bundleRoot, relativePath)).isFile()) {
          found.push(relativePath);
        }
      } catch {
        // Resolves to nothing: a broken link is not a document.
      }
    }
  };
  walk("");

  // Sorted, and sorted on the relative path exactly as the retained reader
  // sorted: directory order is filesystem order, and a report whose document
  // order depends on the inode layout is one whose diff is noise.
  found.sort();
  for (const entry of found) {
    const path = join(bundleRoot, entry);
    const relativePath = portable(entry);
    try {
      documents.push({ path: relativePath, raw: readFileSync(path, "utf8") });
    } catch (cause) {
      unreadable.push({ path: relativePath, reason: reasonOf(cause) });
    }
  }
  return { documents, unreadable };
}

/**
 * Read every located module into the `modules` list
 * `completeness.assess_bundle` accepts.
 *
 * Two-phase on purpose. The shell cannot know which frontmatter schemas a
 * manifest needs without understanding the module layout, and that layout is a
 * rule `quoin-completeness` owns — so it asks: `refsOf(manifest)` is
 * `completeness.schema_refs`, and only the refs it names are read. A shell that
 * guessed (`schemas/*.json`, say) would be a second copy of that rule, and the
 * two would disagree the first time either moved.
 *
 * A module whose `manifest.yaml` is unreadable contributes nothing, silently:
 * that is the retained behaviour, on the retained reasoning that a module which
 * cannot be parsed declares no coverage. A SCHEMA that cannot be read is
 * different — it is reported as `{ unreadable }`, because the manifest named it
 * and its absence is why a declaration cannot be resolved.
 */
export function moduleSnapshots(
  moduleRoots: string[],
  refsOf: (manifest: string) => string[],
): ModuleSource[] {
  const modules: ModuleSource[] = [];
  const seen = new Set<string>();

  for (const candidate of moduleRoots) {
    const moduleRoot = locateModuleRoot(candidate);
    if (!moduleRoot || seen.has(moduleRoot)) continue;
    seen.add(moduleRoot);

    let manifest: string;
    try {
      manifest = readFileSync(join(moduleRoot, "manifest.yaml"), "utf8");
    } catch {
      continue;
    }

    const schemas: Record<string, SchemaSource> = {};
    for (const ref of refsOf(manifest)) {
      try {
        schemas[ref] = { text: readFileSync(join(moduleRoot, ref), "utf8") };
      } catch (cause) {
        schemas[ref] = { unreadable: reasonOf(cause) };
      }
    }
    modules.push({ label: moduleRoot, manifest, schemas });
  }
  return modules;
}

function reasonOf(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}
