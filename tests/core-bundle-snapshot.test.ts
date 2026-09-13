/**
 * `bundleSnapshot` against the filesystem it claims to describe (quoin#445).
 *
 * `src/core/snapshot.ts` walks the bundle and sends document CONTENT across the
 * boundary; `quoin-completeness` analyses only what arrives. Whichever documents
 * that walk decides not to send are therefore not merely absent from the
 * request — they are absent from the VERDICT, and no amount of re-classifying on
 * the Rust side can put them back. Re-classification protects against a
 * pre-filter that is too COARSE; nothing protects against one that is too
 * NARROW.
 *
 * agent-ix/quoin#448 is the incident this file exists because of. That PR
 * defended the same boundary with tests that enumerated the walker's own rules
 * (`tests/core-snapshot.test.ts` is the surviving example). Adding one directory
 * name to the walker's exclusion set left the whole suite green at 14/14 while
 * the shipped command's verdict went from one finding to zero, exit 0 both ways.
 * A test that restates a filter's rules can only ever agree with the filter.
 *
 * So nothing below enumerates a rule. ONE real tree is materialised and compared
 * against an INDEPENDENT walk of that same tree written here, from the
 * filesystem rather than from `snapshot.ts`'s beliefs — and then the whole
 * boundary path is driven end to end so the equivalence is asserted of what the
 * shipped command actually sends.
 *
 * The end-to-end half **skips cleanly** when `QUOIN_CORE` is unset, exactly as
 * `tests/core-exec-e2e.test.ts` does and for the same reason: a plain
 * `vitest run` has no Rust build. `make rust-e2e` builds the workspace and sets
 * it.
 */

import {
  accessSync,
  chmodSync,
  constants,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  statSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

import { describe, expect, it } from "vitest";

import { readBundleFrontmatter } from "../src/core/completeness.js";
import { bundleSnapshot } from "../src/core/snapshot.js";

/** A document with parseable frontmatter, so the far side reports it. */
function document(title: string): string {
  return `---\ntitle: ${title}\nquality_attribute: security\n---\n\n# ${title}\n`;
}

/** The path of the one document whose frontmatter does not parse. */
const BROKEN = "level1/level2/level3/broken.md";

/**
 * Materialise the tree both walks are asked about.
 *
 * Every shape here is one a walk could lose without a rule-enumerating test
 * noticing: three levels of nesting with a document at each, a document whose
 * frontmatter does not parse, a file that is not markdown, a file whose
 * extension is markdown in the WRONG case, a filename holding a space, a
 * filename holding a non-ASCII character, a document the OS will not open, a
 * directory holding nothing at all, a directory NAMED `*.md`, and the three
 * symlink shapes — one resolving to a markdown file outside the bundle, one
 * resolving to a directory, and one resolving to nothing.
 *
 * The bundle root is NESTED inside the temp dir so `shared/` can sit beside it:
 * a symlink whose target lives inside the bundle would be found by the walk
 * anyway and could not show a walk that loses the link.
 */
function bundle(): string {
  const temp = mkdtempSync(join(tmpdir(), "quoin-bundle-snapshot-"));
  const root = join(temp, "bundle");
  mkdirSync(root, { recursive: true });
  const write = (relative: string, contents: string): void => {
    const target = join(root, relative);
    mkdirSync(dirname(target), { recursive: true });
    writeFileSync(target, contents);
  };

  write("index.md", document("Index"));
  write("level1/a.md", document("A"));
  write("level1/level2/b.md", document("B"));
  write("level1/level2/level3/c.md", document("C"));

  // Unparseable frontmatter: an unterminated flow sequence. It must still
  // CROSS — the document that does not parse may be the one carrying the
  // exclusion, and dropping it turns a broken bundle into a clean one.
  write(BROKEN, "---\nquality_attribute: [security\n---\n\nbody\n");

  // Not markdown, and its bytes are valid frontmatter on purpose: a walk that
  // took it would appear as an extra document rather than as nothing.
  write("level1/notes.txt", document("Notes"));

  // Markdown extension in the wrong case; matched case-sensitively on both
  // sides, so if either side ever stopped it would show up as a difference.
  write("level1/UPPER.MD", document("Upper"));

  write("level1/with space.md", document("With Space"));
  write("level1/café.md", document("Café"));

  // Found and unopenable — the third state the walk genuinely observes. Left
  // deliberately unasserted as to WHICH state it lands in: running as root the
  // read succeeds, and the independent walk below observes the same outcome the
  // snapshot does either way.
  write("level1/locked.md", document("Locked"));
  chmodSync(join(root, "level1/locked.md"), 0o000);

  mkdirSync(join(root, "level1/empty"), { recursive: true });

  // A DIRECTORY whose name ends in `.md`. Nothing forbids it, and a recursive
  // listing yields directory entries alongside file ones — so a walk that
  // filtered on the extension alone opened it and recorded `EISDIR` as an
  // unreadable document the bundle does not contain. The file inside it is
  // deliberately not markdown, so the only thing this shape can produce is the
  // invented entry.
  mkdirSync(join(root, "level1/notes.md"), { recursive: true });
  writeFileSync(join(root, "level1/notes.md/inner.txt"), "not a document\n");

  // SYMLINKS, the one narrowing axis nothing here covered. `Dirent.isFile()` is
  // FALSE for a symlink entry, so a filter written on it alone drops a document
  // the direct disk reader on the other side of the boundary reads — and a
  // dropped document is not merely missing from the request, it is missing from
  // the VERDICT: the value it owned reads `unowned` and a finding appears that
  // no reader of the same tree produces. `spec/FR-042.md` symlinked into a
  // shared spec directory is the ordinary monorepo layout, not an exotic shape.
  const shared = join(temp, "shared");
  mkdirSync(join(shared, "nested"), { recursive: true });
  writeFileSync(join(shared, "FR-042.md"), document("Shared FR-042"));
  // Only ever reachable THROUGH a symlink, so a walk that descended one would
  // show up as this extra document.
  writeFileSync(join(shared, "nested/unreachable.md"), document("Unreachable"));

  // Resolves to a FILE: a document.
  symlinkSync(join(shared, "FR-042.md"), join(root, "level1/shared.md"));
  // Resolves to a DIRECTORY, and named `*.md`: not a document, not descended.
  symlinkSync(join(shared, "nested"), join(root, "level1/linked-dir.md"));
  // Resolves to NOTHING: not a document. There are no bytes to read, and
  // recording it `unreadable` would invent an entry the bundle does not hold —
  // the same class of defect as the `EISDIR` one above.
  symlinkSync(join(shared, "gone.md"), join(root, "level1/dangling.md"));

  assertShapes(root);
  return root;
}

/**
 * Every shape {@link bundle} claims, asserted to be on disk.
 *
 * Without this the differential passes over whatever tree it happens to get: a
 * fixture that silently stopped materialising the symlinks — a path that
 * drifted, a `symlinkSync` that moved above an early return — would leave both
 * descriptions agreeing about a smaller tree and the axis uncovered again,
 * which is exactly how agent-ix/quoin#448 stayed green.
 */
function assertShapes(root: string): void {
  const file = (relative: string): void => {
    expect(
      lstatSync(join(root, relative)).isFile(),
      `fixture shape '${relative}' is no longer a plain file`,
    ).toBe(true);
  };
  const directory = (relative: string): void => {
    expect(
      lstatSync(join(root, relative)).isDirectory(),
      `fixture shape '${relative}' is no longer a directory`,
    ).toBe(true);
  };
  const link = (relative: string, resolvesTo: "file" | "dir" | "nothing") => {
    expect(
      lstatSync(join(root, relative)).isSymbolicLink(),
      `fixture shape '${relative}' is no longer a symlink`,
    ).toBe(true);
    let resolved: "file" | "dir" | "nothing" | "other";
    try {
      const stat = statSync(join(root, relative));
      resolved = stat.isFile() ? "file" : stat.isDirectory() ? "dir" : "other";
    } catch {
      resolved = "nothing";
    }
    expect(
      resolved,
      `fixture shape '${relative}' no longer resolves as claimed`,
    ).toBe(resolvesTo);
  };

  file("index.md");
  file("level1/level2/level3/c.md");
  file(BROKEN);
  file("level1/notes.txt");
  file("level1/UPPER.MD");
  file("level1/with space.md");
  file("level1/café.md");
  file("level1/locked.md");
  directory("level1/empty");
  directory("level1/notes.md");
  link("level1/shared.md", "file");
  link("level1/linked-dir.md", "dir");
  link("level1/dangling.md", "nothing");
}

/**
 * Every `*.md` under `root`, read from the filesystem.
 *
 * Written here rather than imported: the whole point is that two INDEPENDENT
 * descriptions of one tree agree. Reusing `snapshot.ts`'s walk would make the
 * test agree with itself no matter what the walk did.
 *
 * Independence has to hold per AXIS, not per file. This classified with
 * `Dirent.isDirectory()`-else-`.md` — which follows a symlink where the walk
 * under test drops it — so on that one axis the two descriptions were never
 * comparable, they simply disagreed by construction over a fixture that
 * materialised no symlink. It now asks the filesystem directly and with two
 * different calls: `lstatSync` never follows, so only a REAL directory is
 * descended and a symlinked one cannot loop the walk out of the tree;
 * `statSync` always follows, so a link is a document exactly when it resolves
 * to a file, and a broken one resolves to nothing.
 */
function independentWalk(root: string): {
  documents: { path: string; raw: string }[];
  unreadable: { path: string; reason: string }[];
} {
  const paths: string[] = [];
  const descend = (prefix: string): void => {
    for (const name of readdirSync(join(root, prefix))) {
      const relative = prefix ? `${prefix}/${name}` : name;
      const absolute = join(root, relative);
      if (lstatSync(absolute).isDirectory()) {
        descend(relative);
        continue;
      }
      if (!relative.endsWith(".md")) continue;
      try {
        if (statSync(absolute).isFile()) paths.push(relative);
      } catch {
        // Resolves to nothing: a broken link is not a document.
      }
    }
  };
  descend("");

  const documents: { path: string; raw: string }[] = [];
  const unreadable: { path: string; reason: string }[] = [];
  for (const path of paths.sort()) {
    try {
      documents.push({ path, raw: readFileSync(join(root, path), "utf8") });
    } catch (cause) {
      unreadable.push({
        path,
        reason: cause instanceof Error ? cause.message : String(cause),
      });
    }
  }
  return { documents, unreadable };
}

function binary(): string | null {
  const path = process.env.QUOIN_CORE;
  if (!path) return null;
  try {
    accessSync(path, constants.X_OK);
    return path;
  } catch {
    return null;
  }
}

const available = binary() !== null;

describe("bundleSnapshot (quoin#445)", () => {
  // Trace: FR-096, FR-037
  it("sends exactly the documents an independent walk of the tree finds", () => {
    const root = bundle();
    const expected = independentWalk(root);

    // One equality, covering both leaking axes at once: which FILENAMES cross
    // and which DIRECTORIES are descended into. Neither is restated as a rule,
    // and paths, content and the unreadable set are all compared.
    expect(bundleSnapshot(root)).toEqual(expected);
  });

  // Trace: FR-096, FR-037
  it("actually swept a populated tree", () => {
    // The floor under the equality above: two descriptions that both saw
    // NOTHING are also equal, so without this a walk that lost the tree passes.
    const root = bundle();
    const snapshot = bundleSnapshot(root);
    const crossed = [...snapshot.documents, ...snapshot.unreadable].map(
      (entry) => entry.path,
    );
    expect(crossed.length).toBeGreaterThanOrEqual(8);
    expect(crossed).toContain("level1/level2/level3/c.md");
    expect(crossed).toContain(BROKEN);
  });

  // Trace: FR-096, FR-037
  it("sends a symlink exactly when it resolves to a file", () => {
    // The equality above catches a disagreement between the two descriptions,
    // but not a decision both got wrong in the same direction — and this
    // decision is verdict-changing on its own. `Dirent.isFile()` is false for
    // every symlink, so the walk used to drop `spec/FR-042.md` symlinked into a
    // shared spec directory: its claims never reached the far side, the value
    // it owned was reported `unowned`, and the verdict moved.
    const root = bundle();
    const crossed = [
      ...bundleSnapshot(root).documents,
      ...bundleSnapshot(root).unreadable,
    ].map((entry) => entry.path);

    expect(crossed).toContain("level1/shared.md");
    // A directory, reached through a link and named `*.md`: not a document, and
    // not recorded `unreadable` either — that would invent an EISDIR entry.
    expect(crossed).not.toContain("level1/linked-dir.md");
    // No bytes to read, so neither a document nor an unreadable one.
    expect(crossed).not.toContain("level1/dangling.md");
    // And a symlinked directory is not descended.
    expect(crossed.filter((path) => path.includes("unreachable.md"))).toEqual(
      [],
    );
  });
});

describe.skipIf(!available)("readBundleFrontmatter ↔ quoin-core", () => {
  // Trace: FR-096, FR-037, FR-101-AC-4
  it("reports exactly the documents the tree holds, over the real boundary", () => {
    const root = bundle();
    const walked = independentWalk(root);

    const read = readBundleFrontmatter(root);

    // Every document the tree holds and the walk could open, minus the one
    // whose frontmatter does not parse — which the far side moves into
    // `unreadable` rather than dropping.
    expect(read.documents.map((entry) => entry.path)).toEqual(
      walked.documents
        .map((entry) => entry.path)
        .filter((path) => path !== BROKEN),
    );
    expect(read.unreadable.map((entry) => entry.path)).toEqual([
      ...walked.unreadable.map((entry) => entry.path),
      BROKEN,
    ]);
    // And the content crossed intact, not just the names.
    const deepest = read.documents.find(
      (entry) => entry.path === "level1/level2/level3/c.md",
    );
    expect(deepest?.frontmatter).toEqual({
      quality_attribute: "security",
      title: "C",
    });
  });
});
