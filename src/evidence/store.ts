/**
 * Reading and writing the evidence store (FR-030).
 *
 * Every write is **canonical**: key-sorted, stable ordering, two-space JSON
 * with a trailing newline. That is what makes a PR diff of the store *be* the
 * per-PR delta — a store whose serialization wobbled would produce noise diffs
 * that reviewers learn to skip, which is how a review artifact stops being
 * read.
 */

import {
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { dirname, join } from "node:path";

import {
  RUNS_DIR,
  STORE_SCHEMA_VERSION,
  type BaselineFile,
  type Binding,
  type BindingsFile,
  type RunRecord,
} from "./types.js";

/**
 * The store root for a repository.
 *
 * Under `spec/`, not at the repository root. quire-rs CR-045 bounds the
 * document walk to `<scope>/spec`, so the authored half of the store —
 * `suites.md` and `inspections.md` — is only a validated corpus document if it
 * lives there. The machine-written half sits beside it so the whole store is
 * one directory rather than two halves in different places.
 *
 * agent-ix/quoin#79's original layout put `evidence/` at the repository root on
 * the premise that quire "validates them wherever they live". Measured: it does
 * not — a typed registry at the root minted nothing and was reported nowhere.
 */
export function storeRoot(repo: string): string {
  return join(repo, "spec", "evidence");
}

export function suitesPath(repo: string): string {
  return join(storeRoot(repo), "suites.md");
}

export function inspectionsPath(repo: string): string {
  return join(storeRoot(repo), "inspections.md");
}

export function bindingsPath(repo: string): string {
  return join(storeRoot(repo), "bindings.json");
}

export function baselinePath(repo: string): string {
  return join(storeRoot(repo), "baseline.json");
}

/** `runs/<SUITE-N>/<commit12>.json` — one file is one run of one suite. */
export function runPath(repo: string, suite: string, commit: string): string {
  return join(storeRoot(repo), RUNS_DIR, suite, `${short(commit)}.json`);
}

/** The 12-character commit prefix the run filenames use. */
export function short(commit: string): string {
  return commit.slice(0, 12);
}

/**
 * Canonical JSON: keys sorted at every level, two-space indent, trailing
 * newline. Deterministic by construction, so two runs over identical inputs
 * produce identical bytes.
 */
export function canonicalJson(value: unknown): string {
  return `${JSON.stringify(sortKeys(value), null, 2)}\n`;
}

function sortKeys(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(sortKeys);
  if (value === null || typeof value !== "object") return value;
  const out: Record<string, unknown> = {};
  for (const key of Object.keys(value as Record<string, unknown>).sort()) {
    out[key] = sortKeys((value as Record<string, unknown>)[key]);
  }
  return out;
}

function writeCanonical(path: string, value: unknown): void {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, canonicalJson(value), "utf8");
}

function readJson<T>(path: string): T | null {
  if (!existsSync(path)) return null;
  return JSON.parse(readFileSync(path, "utf8")) as T;
}

/** Write one run record. Last-write-wins at the same (suite, commit). */
export function writeRun(repo: string, record: RunRecord): string {
  const path = runPath(repo, record.suite, record.commit);
  writeCanonical(path, { ...record, schemaVersion: STORE_SCHEMA_VERSION });
  return path;
}

/** Read one run record, or `null` when that (suite, commit) has none. */
export function readRun(
  repo: string,
  suite: string,
  commit: string,
): RunRecord | null {
  return readJson<RunRecord>(runPath(repo, suite, commit));
}

/** Every run recorded for a suite, newest filename last (lexicographic). */
export function listRuns(repo: string, suite: string): string[] {
  const dir = join(storeRoot(repo), RUNS_DIR, suite);
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter((f) => f.endsWith(".json"))
    .sort();
}

/** Every suite that has at least one recorded run. */
export function listRecordedSuites(repo: string): string[] {
  const dir = join(storeRoot(repo), RUNS_DIR);
  if (!existsSync(dir)) return [];
  return readdirSync(dir, { withFileTypes: true })
    .filter((e) => e.isDirectory())
    .map((e) => e.name)
    .sort();
}

/** The binding graph. An absent file reads as an empty graph, not an error. */
export function readBindings(repo: string): BindingsFile {
  return (
    readJson<BindingsFile>(bindingsPath(repo)) ?? {
      schemaVersion: STORE_SCHEMA_VERSION,
      bindings: [],
    }
  );
}

/** Write the binding graph, ordered by obligation id so diffs stay minimal. */
export function writeBindings(repo: string, file: BindingsFile): void {
  const bindings = [...file.bindings].sort((a, b) =>
    a.obligation.localeCompare(b.obligation),
  );
  writeCanonical(bindingsPath(repo), {
    schemaVersion: STORE_SCHEMA_VERSION,
    bindings,
  });
}

/** The ratchet baseline, or `null` when none has been accepted. */
export function readBaseline(repo: string): BaselineFile | null {
  return readJson<BaselineFile>(baselinePath(repo));
}

export function writeBaseline(repo: string, file: BaselineFile): void {
  writeCanonical(baselinePath(repo), {
    schemaVersion: STORE_SCHEMA_VERSION,
    commit: file.commit,
    undischarged: [...file.undischarged].sort(),
    suspect: [...file.suspect].sort(),
  });
}

/**
 * Retain the latest run per suite plus anything a binding references; delete
 * the rest. Returns the deleted paths.
 *
 * Dir-per-suite is what makes this local: a policy change for one expensive
 * suite touches one directory, and a `.gitignore` decision can be made per
 * suite rather than for the whole store.
 */
export function gc(repo: string, dryRun = false): string[] {
  const referenced = new Set(
    readBindings(repo).bindings.map(
      (b) => `${b.suite}/${short(b.commit)}.json`,
    ),
  );
  const deleted: string[] = [];
  for (const suite of listRecordedSuites(repo)) {
    const runs = listRuns(repo, suite);
    const keep = new Set<string>();
    // "Latest" is the last filename lexicographically, which is a commit
    // prefix rather than a time — deliberately: the store holds no clock of
    // its own, and a caller that wants time ordering has the timestamps.
    const latest = runs.at(-1);
    if (latest) keep.add(latest);
    for (const run of runs) {
      if (keep.has(run) || referenced.has(`${suite}/${run}`)) continue;
      const path = join(storeRoot(repo), RUNS_DIR, suite, run);
      deleted.push(path);
      if (!dryRun) rmSync(path);
    }
  }
  return deleted.sort();
}

/**
 * Bind an obligation to the run that discharged it, stamping the statement hash
 * as it stands now.
 *
 * **Auto-bind, explicit affirmation.** First discharge binds without asking:
 * requiring a human to confirm a binding the evidence already proves would put
 * a gate in front of the common case and teach people to click through it. What
 * stays explicit is *re-affirmation* after a statement changes — that is the
 * judgement call, and it is the one worth a signature.
 *
 * Returns the binding, whether it was newly created, and (when it already
 * existed) whether it is now suspect.
 */
export function bind(
  existing: Binding[],
  next: Omit<Binding, "affirmations">,
): { bindings: Binding[]; created: boolean; suspect: boolean } {
  const idx = existing.findIndex((b) => b.obligation === next.obligation);
  if (idx === -1) {
    return {
      bindings: [...existing, { ...next }],
      created: true,
      suspect: false,
    };
  }
  const prior = existing[idx];
  // The hash is NOT overwritten on re-discharge. Re-running a test does not
  // re-affirm a reworded requirement — if it did, the suspect state would clear
  // itself on the next CI run and the detector would never fire.
  const suspect = prior.statementHashAtBinding !== next.statementHashAtBinding;
  const merged: Binding = {
    ...prior,
    suite: next.suite,
    commit: next.commit,
    symbols: next.symbols,
  };
  const bindings = [...existing];
  bindings[idx] = merged;
  return { bindings, created: false, suspect };
}

/** Record a re-affirmation against an existing binding, clearing suspicion. */
export function affirm(
  existing: Binding[],
  obligation: string,
  currentHash: string,
  who: string,
  commit: string,
  note?: string,
): { bindings: Binding[]; found: boolean } {
  const idx = existing.findIndex((b) => b.obligation === obligation);
  if (idx === -1) return { bindings: existing, found: false };
  const prior = existing[idx];
  const bindings = [...existing];
  bindings[idx] = {
    ...prior,
    // Affirming moves the hash forward: the reviewer has read the new statement
    // and says the evidence still discharges it.
    statementHashAtBinding: currentHash,
    affirmations: [...(prior.affirmations ?? []), { who, commit, note }],
  };
  return { bindings, found: true };
}
