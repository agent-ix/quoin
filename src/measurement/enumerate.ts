/**
 * Corpus enumeration and the read-only envelope (FR-084, FR-092).
 *
 * The population this measurement reports on is an authored input, not a
 * discovered fact. The same workspace yields 331,702, 24,643 or 7,587 documents
 * depending on the exclusion vocabulary, so the vocabulary is recorded verbatim
 * beside every count. A rate without its population is not a measurement.
 *
 * Nothing here writes outside the declared output directory, and the run
 * asserts that: `HEAD` and cleanliness are read for every enumerated repository
 * before and after the walk, and any repository whose either value moved is
 * recorded `stable: false` rather than silently averaged into the result.
 */

import { execFileSync } from "node:child_process";
import { existsSync, lstatSync, readdirSync, statSync } from "node:fs";
import { join, relative, resolve, sep } from "node:path";

/** A candidate rejected by enumeration, with the rule that rejected it. */
export interface Excluded {
  readonly path: string;
  readonly rule:
    | "git-link-file"
    | "nested-repository"
    | "symlink"
    | "excluded-directory"
    | "no-spec-directory"
    | "not-a-repository";
}

/** One enumerated repository, with the state it was read in. */
export interface RepositoryRecord {
  readonly path: string;
  readonly origin: string | null;
  readonly commit: string | null;
  readonly clean: boolean;
  /** False when `HEAD` or cleanliness moved between the pre- and post-walk reads. */
  readonly stable: boolean;
  readonly documents: number;
}

export interface CorpusRecord {
  readonly corpusId: string;
  readonly workspaceRoot: string;
  readonly exclusionVocabulary: readonly string[];
  readonly repositories: readonly RepositoryRecord[];
  readonly excluded: readonly Excluded[];
}

function git(cwd: string, ...args: string[]): string | null {
  try {
    return execFileSync("git", args, { cwd, encoding: "utf8" }).trim();
  } catch {
    return null;
  }
}

/** `HEAD` and cleanliness, read together so the pair can be compared later. */
function readState(root: string): { commit: string | null; clean: boolean } {
  const commit = git(root, "rev-parse", "HEAD");
  const status = git(root, "status", "--porcelain");
  return { commit, clean: status === "" };
}

function isRepository(dir: string): boolean {
  const dot = join(dir, ".git");
  return existsSync(dot) && statSync(dot).isDirectory();
}

/**
 * Counts `spec/**\/*.md` documents, excluding any subtree carrying its own
 * `.git`. A nested repository is a separate population and is evaluated on its
 * own, never folded into the enclosing count.
 */
function countDocuments(root: string, excluded: Excluded[]): number {
  let total = 0;
  const walk = (dir: string): void => {
    let entries: string[];
    try {
      entries = readdirSync(dir);
    } catch {
      return;
    }
    if (dir !== root && entries.includes(".git")) {
      excluded.push({ path: dir, rule: "nested-repository" });
      return;
    }
    for (const entry of entries) {
      const full = join(dir, entry);
      const info = lstatSync(full);
      if (info.isSymbolicLink()) {
        excluded.push({ path: full, rule: "symlink" });
        continue;
      }
      if (info.isDirectory()) {
        walk(full);
        continue;
      }
      if (entry.endsWith(".md")) total += 1;
    }
  };
  const specDir = join(root, "spec");
  if (existsSync(specDir)) walk(specDir);
  return total;
}

/**
 * Refuses before reading anything when the output directory sits inside a
 * candidate repository: a run that writes into its own population has changed
 * what it is measuring, and no later assertion can undo that.
 */
export function assertOutputOutsideCorpus(
  outputDir: string,
  workspaceRoot: string,
  vocabulary: readonly string[],
): void {
  const out = resolve(outputDir);
  for (const entry of readdirSync(workspaceRoot)) {
    if (vocabulary.includes(entry)) continue;
    const dir = join(workspaceRoot, entry);
    if (!existsSync(dir) || !lstatSync(dir).isDirectory()) continue;
    if (!isRepository(dir)) continue;
    const rel = relative(dir, out);
    if (rel !== "" && !rel.startsWith("..") && !rel.startsWith(sep)) {
      throw new Error(
        `the output directory ${out} is inside the enumerated repository ${dir}; ` +
          `a run that writes into its own population has changed what it measures`,
      );
    }
  }
}

/** Enumerates the corpus. Exits normally whatever it finds. */
export function enumerateCorpus(options: {
  workspaceRoot: string;
  exclusionVocabulary: readonly string[];
  corpusId: string;
}): CorpusRecord {
  const { workspaceRoot, exclusionVocabulary, corpusId } = options;
  const excluded: Excluded[] = [];
  const repositories: RepositoryRecord[] = [];
  const before = new Map<string, { commit: string | null; clean: boolean }>();

  const candidates = readdirSync(workspaceRoot).sort();
  const roots: string[] = [];
  for (const entry of candidates) {
    const dir = join(workspaceRoot, entry);
    if (exclusionVocabulary.includes(entry)) {
      excluded.push({ path: dir, rule: "excluded-directory" });
      continue;
    }
    let info: ReturnType<typeof lstatSync>;
    try {
      info = lstatSync(dir);
    } catch {
      continue;
    }
    if (info.isSymbolicLink()) {
      excluded.push({ path: dir, rule: "symlink" });
      continue;
    }
    if (!info.isDirectory()) continue;
    const dot = join(dir, ".git");
    if (!existsSync(dot)) {
      excluded.push({ path: dir, rule: "not-a-repository" });
      continue;
    }
    if (!statSync(dot).isDirectory()) {
      // A worktree or submodule link. Counting it would report the repository
      // it belongs to twice, under two different states.
      excluded.push({ path: dir, rule: "git-link-file" });
      continue;
    }
    if (!existsSync(join(dir, "spec"))) {
      excluded.push({ path: dir, rule: "no-spec-directory" });
      continue;
    }
    roots.push(dir);
    before.set(dir, readState(dir));
  }

  for (const dir of roots) {
    const documents = countDocuments(dir, excluded);
    const after = readState(dir);
    const pre = before.get(dir);
    repositories.push({
      path: dir,
      origin: git(dir, "remote", "get-url", "origin"),
      commit: after.commit,
      clean: after.clean,
      stable: pre?.commit === after.commit && pre?.clean === after.clean,
      documents,
    });
  }

  return {
    corpusId,
    workspaceRoot,
    exclusionVocabulary: [...exclusionVocabulary],
    repositories,
    excluded,
  };
}
