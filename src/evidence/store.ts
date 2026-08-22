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
  SCANS_DIR,
  STORE_SCHEMA_VERSION,
  TRUST_DIR,
  type BaselineFile,
  type Binding,
  type BindingsFile,
  type FindingRecord,
  type RunRecord,
  type TrustDecision,
} from "./types.js";
import { validateTrustDecision } from "./trust.js";

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

export function trustDecisionPath(repo: string, id: string): string {
  if (!/^ETD-[0-9]+$/.test(id))
    throw new Error(`invalid trust decision id '${id}'`);
  return join(storeRoot(repo), TRUST_DIR, `${id}.json`);
}

export function writeTrustDecision(repo: string, raw: TrustDecision): string {
  const decision = validateTrustDecision(raw);
  const path = trustDecisionPath(repo, decision.id);
  writeCanonical(path, { ...decision, schemaVersion: STORE_SCHEMA_VERSION });
  return path;
}

export function readTrustDecision(
  repo: string,
  id: string,
): TrustDecision | null {
  const raw = readJson<TrustDecision>(trustDecisionPath(repo, id));
  return raw === null ? null : validateTrustDecision(raw);
}

export function readTrustDecisions(
  repo: string,
  skipped: string[] = [],
): TrustDecision[] {
  const dir = join(storeRoot(repo), TRUST_DIR);
  if (!existsSync(dir)) return [];
  const decisions: TrustDecision[] = [];
  for (const file of readdirSync(dir)
    .filter((name) => name.endsWith(".json"))
    .sort()) {
    const path = join(dir, file);
    try {
      const raw = readJson<TrustDecision>(path);
      if (raw) decisions.push(validateTrustDecision(raw));
    } catch {
      skipped.push(path);
    }
  }
  return decisions;
}

/** `runs/<SUITE-N>/<commit12>.json` — one file is one run of one suite. */
export function runPath(repo: string, suite: string, commit: string): string {
  return join(storeRoot(repo), RUNS_DIR, suite, `${short(commit)}.json`);
}

/** `scans/<SUITE-N>/<commit12>.json` — one file is one scan of one suite. */
export function scanPath(repo: string, suite: string, commit: string): string {
  return join(storeRoot(repo), SCANS_DIR, suite, `${short(commit)}.json`);
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

/**
 * A store file that exists and cannot be read as JSON.
 *
 * `bindings.json` and `baseline.json` are **checked into git**, so a merge
 * conflict leaves `<<<<<<< HEAD` in one of them. Before this, every store read
 * threw a bare `SyntaxError: Unexpected token '<'` naming no file
 * (agent-ix/quoin#106).
 */
export class StoreReadError extends Error {
  constructor(
    readonly path: string,
    readonly cause: unknown,
  ) {
    const detail = cause instanceof Error ? cause.message : String(cause);
    super(
      `${path} exists but is not readable JSON: ${detail}. A merge conflict ` +
        `in a checked-in store file is the usual cause — resolve it, or delete ` +
        `the file to start from an empty store.`,
    );
    this.name = "StoreReadError";
  }
}

function readJson<T>(path: string): T | null {
  if (!existsSync(path)) return null;
  try {
    return JSON.parse(readFileSync(path, "utf8")) as T;
  } catch (cause) {
    throw new StoreReadError(path, cause);
  }
}

/**
 * As {@link readJson}, but a corrupt file is **skipped and reported** rather
 * than fatal.
 *
 * Used for `runs/**`, where one bad file must not hide every finding in the
 * report. The binding graph and the baseline are different: those are the
 * store's spine, and silently reading an empty graph because the file is
 * unparseable would report every obligation as undischarged.
 */
function readJsonSkippable<T>(path: string, skipped: string[]): T | null {
  try {
    return readJson<T>(path);
  } catch (error) {
    if (error instanceof StoreReadError) {
      skipped.push(path);
      return null;
    }
    throw error;
  }
}

/** Write one run record. Last-write-wins at the same (suite, commit). */
export function writeRun(repo: string, record: RunRecord): string {
  const path = runPath(repo, record.suite, record.commit);
  writeCanonical(path, { ...record, schemaVersion: STORE_SCHEMA_VERSION });
  return path;
}

/**
 * Write one finding-shaped scan. Last-write-wins at the same (suite, commit).
 *
 * Writing the record IS the assertion that the scan executed, so this is only
 * ever called with an envelope an adapter actually read (FR-034).
 */
export function writeScan(repo: string, record: FindingRecord): string {
  const path = scanPath(repo, record.suite, record.commit);
  writeCanonical(path, { ...record, schemaVersion: STORE_SCHEMA_VERSION });
  return path;
}

/** Read one scan record, or `null` when that (suite, commit) has none. */
export function readScan(
  repo: string,
  suite: string,
  commit: string,
): FindingRecord | null {
  return readJson<FindingRecord>(scanPath(repo, suite, commit));
}

/** Every scan file recorded for a suite, in filename order (not time order). */
export function listScans(repo: string, suite: string): string[] {
  const dir = join(storeRoot(repo), SCANS_DIR, suite);
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter((f) => f.endsWith(".json"))
    .sort();
}

/**
 * Every scan recorded for a suite, oldest first by `timestamp`.
 *
 * Ordered exactly as `readRuns` is, and for the same reason: a filename is a
 * commit prefix, which is uniformly random hex, so lexical order is not time
 * order. agent-ix/quoin#104 fixed that once for runs; reintroducing it here
 * would rebuild the same defect beside the fix.
 */
export function readScans(
  repo: string,
  suite: string,
  skipped: string[] = [],
): FindingRecord[] {
  return listScans(repo, suite)
    .map((f) =>
      readJsonSkippable<FindingRecord>(
        scanPath(repo, suite, f.replace(/\.json$/, "")),
        skipped,
      ),
    )
    .filter((r): r is FindingRecord => r !== null)
    .sort(
      (a, b) =>
        compare(a.timestamp, b.timestamp) || compare(a.commit, b.commit),
    );
}

/** The newest scan of a suite by timestamp, or `null` when it has none. */
export function latestScan(
  repo: string,
  suite: string,
  skipped: string[] = [],
): FindingRecord | null {
  return readScans(repo, suite, skipped).at(-1) ?? null;
}

/**
 * Whether a scan proves nothing — the finding-shaped meaning of vacuity.
 *
 * **Not "found nothing".** A scan that ran every rule and reported no finding
 * is a clean result and is exactly the evidence this record type exists to
 * preserve. What proves nothing is a scan that **evaluated no rules**: it also
 * reports zero findings, and from the findings list alone the two are
 * identical.
 *
 * Returns `null` when the tool did not say how many rules it evaluated. The
 * question cannot be asked, so the auditor says nothing rather than something
 * wrong — the same posture `evidenceKind` takes for method conformance
 * (agent-ix/quoin#105).
 */
export function scanIsVacuous(record: FindingRecord): boolean | null {
  if (record.rulesEvaluated === undefined) return null;
  return record.rulesEvaluated === 0;
}

/** Read one run record, or `null` when that (suite, commit) has none. */
export function readRun(
  repo: string,
  suite: string,
  commit: string,
): RunRecord | null {
  return readJson<RunRecord>(runPath(repo, suite, commit));
}

/**
 * Every run file recorded for a suite, in filename order.
 *
 * Filename order is **not** time order — a filename is a commit prefix, which
 * is uniformly random hex. Use [`latestRun`] when you mean the newest run;
 * this exists for enumeration.
 */
export function listRuns(repo: string, suite: string): string[] {
  const dir = join(storeRoot(repo), RUNS_DIR, suite);
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter((f) => f.endsWith(".json"))
    .sort();
}

/**
 * Every run recorded for a suite, oldest first by `timestamp`.
 *
 * An unreadable run file is skipped and its path pushed onto `skipped` when the
 * caller passes one — one corrupt record must not hide every finding in the
 * report (agent-ix/quoin#106).
 */
export function readRuns(
  repo: string,
  suite: string,
  skipped: string[] = [],
): RunRecord[] {
  return listRuns(repo, suite)
    .map((f) =>
      readJsonSkippable<RunRecord>(
        runPath(repo, suite, f.replace(/\.json$/, "")),
        skipped,
      ),
    )
    .filter((r): r is RunRecord => r !== null)
    .sort(
      (a, b) =>
        compare(a.timestamp, b.timestamp) || compare(a.commit, b.commit),
    );
}

/**
 * The newest run of a suite, by **timestamp** — or `null` when it has none.
 *
 * Not `listRuns().at(-1)`. A run filename is `<commit12>.json` and a commit
 * prefix is uniformly random hex, so the lexicographically last file is the
 * newest run with probability 1/n. That arbitrary choice drove the auditor's
 * freshness and vacuity checks and `gc`'s retention: record a fresh run at HEAD
 * and, if an older run's prefix happened to sort higher, `stale-evidence` was
 * reported against evidence that was current — and re-recording could not fix
 * it, because the ordering is a property of the hashes and not of what you do
 * (agent-ix/quoin#104).
 *
 * `RunRecord.timestamp` was there the whole time. The commit is the tiebreak,
 * so two runs stamped identically still order deterministically.
 */
export function latestRun(
  repo: string,
  suite: string,
  skipped: string[] = [],
): RunRecord | null {
  return readRuns(repo, suite, skipped).at(-1) ?? null;
}

/**
 * The newest recorded run of every suite that has one.
 *
 * Lives here rather than in a command because more than one caller needs it.
 * It was private to `evidence audit`, and the matching `latestScans` being
 * private *and uncalled* is exactly what SR-005 FND-002 was: the record type,
 * the vacuity check and the tests all existed while `audit()` ran with no scans
 * at all. A reader every consumer needs belongs beside the store it reads.
 */
export function latestRuns(repo: string): RunRecord[] {
  return listRecordedSuites(repo)
    .map((suite) => latestRun(repo, suite))
    .filter((r): r is RunRecord => r !== null);
}

/** The newest scan of every recorded suite. Mirrors {@link latestRuns}. */
export function latestScans(repo: string): FindingRecord[] {
  return listRecordedSuites(repo)
    .map((suite) => latestScan(repo, suite))
    .filter((s): s is FindingRecord => s !== null);
}

/** Locale-independent string order. */
function compare(a: string, b: string): number {
  return a === b ? 0 : a < b ? -1 : 1;
}

/** Every suite that has at least one recorded run. */
export function listRecordedSuites(repo: string): string[] {
  // BOTH directories. A suite that recorded only scans has evidence, and
  // reading `runs/` alone made it invisible to every caller that enumerates —
  // the auditor included, so its obligations read as undischarged while the
  // evidence sat on disk (SR-005 FND-003).
  const suites = new Set<string>();
  for (const which of [RUNS_DIR, SCANS_DIR]) {
    const dir = join(storeRoot(repo), which);
    if (!existsSync(dir)) continue;
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      if (entry.isDirectory()) suites.add(entry.name);
    }
  }
  return [...suites].sort();
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

/**
 * Write the binding graph, ordered by `(obligation, suite)` so diffs stay
 * minimal.
 *
 * Compared with plain `<`, not `localeCompare`: this file is checked in and its
 * diff is meant to *be* the per-PR delta, and `localeCompare` without an
 * explicit locale depends on the runtime's ICU data — so two machines could
 * serialize the same binding set in different orders and produce a diff nobody
 * made.
 */
/**
 * `Pick<…, "bindings">` and not `BindingsFile`: the version written is always
 * `STORE_SCHEMA_VERSION`, and `file.schemaVersion` was never read. Demanding it
 * made callers supply a value the function discards — which one command did not,
 * and the resulting type error stayed latent until an unrelated entry widened
 * the build graph enough for `tsc` to reach the call site.
 */
export function writeBindings(
  repo: string,
  file: Pick<BindingsFile, "bindings">,
): void {
  const bindings = [...file.bindings].sort(byObligationThenSuite);
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
    accepted: [...file.accepted].sort(compare),
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
    // "Latest" is the newest by `timestamp`, not the last filename. A filename
    // is a commit prefix — uniformly random hex — so retaining `runs.at(-1)`
    // deleted the actual newest run whenever its prefix sorted low and no
    // binding named it (agent-ix/quoin#104).
    const latest = latestRun(repo, suite);
    if (latest) keep.add(`${short(latest.commit)}.json`);
    for (const run of runs) {
      if (keep.has(run) || referenced.has(`${suite}/${run}`)) continue;
      const path = join(storeRoot(repo), RUNS_DIR, suite, run);
      deleted.push(path);
      if (!dryRun) rmSync(path);
    }

    // Scans, on the same rule. Walking only `runs/` left every scan record on
    // disk forever, so a store with scan suites grew without bound while `gc`
    // reported it had collected everything (SR-005 FND-004).
    const scans = listScans(repo, suite);
    const keepScan = new Set<string>();
    const latestScanRecord = latestScan(repo, suite);
    if (latestScanRecord) {
      keepScan.add(`${short(latestScanRecord.commit)}.json`);
    }
    for (const scan of scans) {
      if (keepScan.has(scan) || referenced.has(`${suite}/${scan}`)) continue;
      const path = join(storeRoot(repo), SCANS_DIR, suite, scan);
      deleted.push(path);
      if (!dryRun) rmSync(path);
    }
  }
  return deleted.sort();
}

/** Total order over the binding graph. Locale-independent by construction. */
function byObligationThenSuite(a: Binding, b: Binding): number {
  if (a.obligation !== b.obligation)
    return a.obligation < b.obligation ? -1 : 1;
  if (a.suite === b.suite) return 0;
  return a.suite < b.suite ? -1 : 1;
}

/**
 * Bind an obligation to the run that discharged it, stamping the statement hash
 * as it stands now.
 *
 * **Keyed on `(obligation, suite)`, not on the obligation alone.** The binding
 * graph is cross-suite by design — one obligation discharged by a unit suite
 * and a mutation suite is the case `bindings.json` exists to hold in one file —
 * and keying on the obligation made the second suite *overwrite* the first.
 * Two consequences, both silent: the cross-suite relationship was destroyed on
 * write, and `insufficient-multiplicity` counted distinct suites per obligation
 * across a set that could never hold more than one, so the finding could not be
 * cleared by any amount of evidence (agent-ix/quoin#102).
 *
 * **Auto-bind, explicit affirmation.** First discharge binds without asking:
 * requiring a human to confirm a binding the evidence already proves would put
 * a gate in front of the common case and teach people to click through it. What
 * stays explicit is *re-affirmation* after a statement changes — that is the
 * judgement call, and it is the one worth a signature.
 *
 * Returns the binding, whether it was newly created, and (when this
 * `(obligation, suite)` already existed) whether it is now suspect.
 */
export function bind(
  existing: Binding[],
  next: Omit<Binding, "affirmations">,
): { bindings: Binding[]; created: boolean; suspect: boolean } {
  const idx = existing.findIndex(
    (b) => b.obligation === next.obligation && b.suite === next.suite,
  );
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
    commit: next.commit,
    symbols: next.symbols,
    // Lineage describes the CURRENT evidence relationship. Omission on a new
    // record clears it rather than silently carrying an old independence claim
    // into evidence that did not state one.
    lineage: next.lineage,
  };
  const bindings = [...existing];
  bindings[idx] = merged;
  return { bindings, created: false, suspect };
}

/**
 * Record a re-affirmation, clearing suspicion.
 *
 * Affirms **every** binding for the obligation, not the first one found. The
 * judgement being recorded is "I have read the new statement and the evidence
 * still discharges it", and that is a judgement about the requirement rather
 * than about one suite — affirming only the first would leave a sibling suite
 * suspect for a reason nobody could act on. `suite` narrows it when the
 * reviewer means only one.
 */
export function affirm(
  existing: Binding[],
  obligation: string,
  currentHash: string,
  who: string,
  commit: string,
  note?: string,
  suite?: string,
): { bindings: Binding[]; found: boolean } {
  const matches = existing
    .map((b, i) => [b, i] as const)
    .filter(
      ([b]) => b.obligation === obligation && (!suite || b.suite === suite),
    );
  if (matches.length === 0) return { bindings: existing, found: false };
  const bindings = [...existing];
  for (const [prior, idx] of matches) {
    bindings[idx] = {
      ...prior,
      // Affirming moves the hash forward: the reviewer has read the new
      // statement and says the evidence still discharges it.
      statementHashAtBinding: currentHash,
      affirmations: [...(prior.affirmations ?? []), { who, commit, note }],
    };
  }
  return { bindings, found: true };
}
