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
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
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
 * filename holding a non-ASCII character, a document the OS will not open, and
 * a directory holding nothing at all.
 */
function bundle(): string {
  const root = mkdtempSync(join(tmpdir(), "quoin-bundle-snapshot-"));
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

  return root;
}

/**
 * Every `*.md` under `root`, read from the filesystem.
 *
 * Written here rather than imported: the whole point is that two INDEPENDENT
 * descriptions of one tree agree. Reusing `snapshot.ts`'s walk would make the
 * test agree with itself no matter what the walk did.
 */
function independentWalk(root: string): {
  documents: { path: string; raw: string }[];
  unreadable: { path: string; reason: string }[];
} {
  const paths: string[] = [];
  const descend = (prefix: string): void => {
    for (const entry of readdirSync(join(root, prefix), {
      withFileTypes: true,
    })) {
      const relative = prefix ? `${prefix}/${entry.name}` : entry.name;
      if (entry.isDirectory()) descend(relative);
      else if (relative.endsWith(".md")) paths.push(relative);
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
