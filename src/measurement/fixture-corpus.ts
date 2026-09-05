/**
 * The fixture corpus and the run budget (FR-092).
 *
 * Every reproducibility claim this measurement makes is made over this
 * corpus, not over `~/dev`. A determinism assertion against one developer's
 * workspace is not reproducible by anyone else and degrades the moment that
 * workspace changes — which, for a workspace this campaign is actively
 * committing to, is continuously.
 *
 * The corpus exercises each exclusion rule, each document state and each
 * outcome, so an assertion that passes here has actually met the cases rather
 * than met whatever the workspace happened to contain that day.
 */

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { cpus, totalmem, platform, arch } from "node:os";
import { mkdirSync, symlinkSync, writeFileSync } from "node:fs";
import { join } from "node:path";

export interface RunEnvironment {
  readonly cpuCount: number;
  readonly totalMemoryBytes: number;
  readonly platform: string;
  readonly arch: string;
}

/** Recorded in the run manifest: a timing without its machine is not a budget. */
export function runEnvironment(): RunEnvironment {
  return {
    cpuCount: cpus().length,
    totalMemoryBytes: totalmem(),
    platform: platform(),
    arch: arch(),
  };
}

function git(cwd: string, ...args: string[]): void {
  execFileSync("git", args, { cwd, stdio: "ignore" });
}

function repo(root: string, name: string): string {
  const dir = join(root, name);
  mkdirSync(join(dir, "spec"), { recursive: true });
  git(dir, "init", "-q");
  git(dir, "config", "user.email", "fixture@example.invalid");
  git(dir, "config", "user.name", "fixture");
  return dir;
}

function commitAll(dir: string): void {
  git(dir, "add", "-A");
  git(dir, "commit", "-qm", "fixture");
}

/**
 * Builds the fixture corpus. Every case the enumeration and the state
 * assignment can produce is represented, so a green run means the cases were
 * met rather than absent.
 */
export function buildFixtureCorpus(root: string): string {
  mkdirSync(root, { recursive: true });

  // A clean repository with an origin and one measured document.
  const clean = repo(root, "clean-with-origin");
  writeFileSync(
    join(clean, "spec", "measured.md"),
    "---\ntype: FR\n---\n# measured\n",
  );
  commitAll(clean);
  git(clean, "remote", "add", "origin", "https://example.invalid/clean.git");

  // A repository with no origin: `origin` must be recorded null, not guessed.
  const noOrigin = repo(root, "no-origin");
  writeFileSync(
    join(noOrigin, "spec", "doc.md"),
    "---\ntype: NFR\n---\n# doc\n",
  );
  commitAll(noOrigin);

  // A dirty tree: must be recorded `clean: false` rather than measured as if
  // its committed state were what was read.
  const dirty = repo(root, "dirty-tree");
  writeFileSync(join(dirty, "spec", "doc.md"), "---\ntype: FR\n---\n# doc\n");
  commitAll(dirty);
  writeFileSync(join(dirty, "spec", "doc.md"), "---\ntype: FR\n---\n# edited\n");

  // A `.git` file rather than a directory: the `git-link-file` rule, so a
  // worktree is not counted as a second copy of its repository.
  const linked = join(root, "worktree-link");
  mkdirSync(join(linked, "spec"), { recursive: true });
  writeFileSync(join(linked, ".git"), "gitdir: /elsewhere\n");
  writeFileSync(join(linked, "spec", "doc.md"), "---\ntype: FR\n---\n# doc\n");

  // A nested repository below an enumerated root: a separate population,
  // excluded from the enclosing count rather than folded into it.
  const nested = repo(root, "with-nested");
  writeFileSync(join(nested, "spec", "outer.md"), "---\ntype: FR\n---\n# outer\n");
  const inner = join(nested, "spec", "inner");
  mkdirSync(inner, { recursive: true });
  writeFileSync(join(inner, ".git"), "gitdir: /elsewhere\n");
  writeFileSync(join(inner, "doc.md"), "---\ntype: FR\n---\n# inner\n");
  commitAll(nested);

  // A repository with no `spec` directory.
  const noSpec = join(root, "no-spec");
  mkdirSync(noSpec, { recursive: true });
  git(noSpec, "init", "-q");

  // Documents covering each state.
  const states = repo(root, "document-states");
  const spec = join(states, "spec");
  writeFileSync(join(spec, "measured.md"), "---\ntype: FR\n---\n# ok\n");
  writeFileSync(join(spec, "no-type.md"), "# no frontmatter at all\n");
  writeFileSync(join(spec, "unknown-type.md"), "---\ntype: NotAType\n---\n# x\n");
  writeFileSync(join(spec, "unterminated.md"), "---\ntype: FR\nno closing fence\n");
  commitAll(states);

  // A symlink, never traversed.
  try {
    symlinkSync(clean, join(root, "a-symlink"));
  } catch {
    // A platform without symlink permission still gets the rest of the corpus;
    // the symlink case is asserted only where it can be created.
  }

  return root;
}

/** A digest over an artifact's bytes, used for the determinism assertions. */
export function digestOf(value: unknown): string {
  return createHash("sha256")
    .update(JSON.stringify(value, Object.keys(value as object).sort()))
    .digest("hex");
}

/**
 * Digests a record while ignoring fields that legitimately differ between
 * runs. The ignored set is explicit and small: if it grows, the determinism
 * claim is being weakened rather than met.
 */
export function stableDigest(
  record: Record<string, unknown>,
  ignore: readonly string[] = ["startedAt", "finishedAt", "durationMs"],
): string {
  const copy: Record<string, unknown> = {};
  for (const key of Object.keys(record).sort()) {
    if (ignore.includes(key)) continue;
    copy[key] = record[key];
  }
  return createHash("sha256").update(JSON.stringify(copy)).digest("hex");
}

export class BudgetError extends Error {}

/**
 * Asserts a run stayed inside its declared budget, against the machine it ran
 * on. A wall-clock budget with no machine recorded is a number that cannot be
 * reproduced or contested.
 */
export function assertBudget(input: {
  durationMs: number;
  peakMemoryBytes: number;
  budgetMs: number;
  budgetMemoryBytes: number;
  environment: RunEnvironment;
}): void {
  if (input.durationMs > input.budgetMs) {
    throw new BudgetError(
      `run took ${input.durationMs}ms against a ${input.budgetMs}ms budget on ` +
        `${input.environment.platform}/${input.environment.arch} with ${input.environment.cpuCount} cpus`,
    );
  }
  if (input.peakMemoryBytes > input.budgetMemoryBytes) {
    throw new BudgetError(
      `run peaked at ${input.peakMemoryBytes} bytes against a ${input.budgetMemoryBytes} byte budget`,
    );
  }
}
