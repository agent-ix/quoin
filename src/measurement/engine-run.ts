/**
 * Engine-driven structural measurement (FR-087).
 *
 * The engine decides conformance; this records what it said. Nothing here
 * re-implements a rule, and nothing here converts a warning into a failure: an
 * error is a `fail`, a warning is an advisory finding, and a batch that
 * terminated abnormally is `could-not-run` for every document in it.
 *
 * `could-not-run` is deliberately not a `fail`. "The engine rejected this
 * document" and "the engine never reached this document" are different facts,
 * and merging them inflates a failure rate with runs that never happened.
 */

import { execFileSync } from "node:child_process";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

export type Outcome = "pass" | "fail" | "could-not-run";

export interface Diagnostic {
  readonly code: string | null;
  readonly severity: string;
  readonly message: string;
  readonly path: string | null;
  readonly line: number | null;
}

export interface Evaluation {
  readonly path: string;
  readonly outcome: Outcome;
  readonly diagnostics: readonly Diagnostic[];
  /** Set when `could-not-run`, naming why the batch did not produce an outcome. */
  readonly failure?: string;
}

/**
 * Materializes the pinned module set into one directory, read out of each
 * module repository's object store at its pinned commit.
 *
 * This is what makes the run attributable: the engine is pointed at these
 * trees, so an installed catalog copy or a repository-local module cannot
 * supply a contract the run did not pin.
 */
export function materializeModules(
  modules: readonly { name: string; repositoryPath: string; commit: string }[],
): string {
  const root = mkdtempSync(join(tmpdir(), "pinned-modules-"));
  for (const m of modules) {
    const tree = execFileSync(
      "git",
      ["ls-tree", "-r", "--name-only", m.commit],
      { cwd: m.repositoryPath, encoding: "utf8", maxBuffer: 1 << 28 },
    );
    const pkg = tree
      .split("\n")
      .find((p) => /^[a-z_]+\/manifest\.yaml$/.test(p))
      ?.split("/")[0];
    if (!pkg) continue;
    const dest = join(root, m.name);
    execFileSync("mkdir", ["-p", dest]);
    const archive = execFileSync(
      "git",
      ["archive", "--format=tar", m.commit, pkg],
      { cwd: m.repositoryPath, maxBuffer: 1 << 28 },
    );
    const tar = join(root, `${m.name}.tar`);
    writeFileSync(tar, archive);
    execFileSync("tar", ["-xf", tar, "-C", dest, "--strip-components=1"]);
  }
  return root;
}

interface RawDiagnostic {
  code?: string;
  kind?: string;
  severity?: string;
  message?: string;
  path?: string;
  line?: number;
}

function parseDiagnostics(stderr: string): Diagnostic[] {
  const out: Diagnostic[] = [];
  for (const line of stderr.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed.startsWith("{")) continue;
    let raw: RawDiagnostic;
    try {
      raw = JSON.parse(trimmed) as RawDiagnostic;
    } catch {
      continue;
    }
    // The engine does not always carry `path` as a field: the same run emits
    // it under one invocation and omits it under another, while the message
    // always begins with the document path. Matching on the field alone
    // associates zero diagnostics with every document and reports a clean
    // batch — a runner that cannot fail is not measuring anything.
    const message = raw.message ?? "";
    const fromMessage = /^(\/[^\s:]+\.md):/.exec(message);
    out.push({
      code: raw.code ?? raw.kind ?? null,
      severity: raw.severity ?? "error",
      message,
      path: raw.path ?? fromMessage?.[1] ?? null,
      line: raw.line ?? null,
    });
  }
  return out;
}

/**
 * Runs one batch. A batch is a repository, so an abnormal termination is
 * attributable to a bounded set of documents rather than to the whole corpus.
 */
export function runBatch(options: {
  scope: string;
  documents: readonly string[];
  modulesPath: string;
  quire?: string;
}): Evaluation[] {
  const { scope, documents, modulesPath, quire = "quire" } = options;
  if (documents.length === 0) return [];

  let stderr = "";
  let terminated: string | null = null;
  try {
    const result = execFileSync(
      quire,
      ["validate", "--scope", scope, "--diagnostics-format", "json", ...documents],
      {
        // `--scope` bounds relative globs, but the engine still resolves them
        // against the process working directory: run this from anywhere else
        // and it validates that directory's documents instead, reporting a
        // clean batch for documents it never opened.
        cwd: scope,
        encoding: "utf8",
        maxBuffer: 1 << 28,
        env: { ...process.env, IX_FILAMENT_MODULES_PATH: modulesPath },
        stdio: ["ignore", "pipe", "pipe"],
      },
    );
    void result;
  } catch (error) {
    const e = error as { stderr?: string; status?: number; signal?: string };
    stderr = e.stderr ?? "";
    // A non-zero exit is how the engine reports errors; only a signal or a
    // missing stream means the batch did not run.
    if (e.signal || (e.status === undefined && !e.stderr)) {
      terminated = `batch terminated: signal=${e.signal ?? "none"} status=${e.status ?? "none"}`;
    }
  }

  if (terminated) {
    return documents.map((path) => ({
      path,
      outcome: "could-not-run" as const,
      diagnostics: [],
      failure: terminated ?? undefined,
    }));
  }

  const diagnostics = parseDiagnostics(stderr);
  // The engine reports absolute paths. Matching on the paths as passed in
  // silently associates zero diagnostics with every document and reports the
  // whole batch as `pass` — a runner that cannot fail is not measuring.
  const byPath = new Map<string, Diagnostic[]>();
  for (const d of diagnostics) {
    if (!d.path) continue;
    const key = resolve(d.path);
    const list = byPath.get(key) ?? [];
    list.push(d);
    byPath.set(key, list);
  }

  return documents.map((path) => {
    const mine = byPath.get(resolve(scope, path)) ?? [];
    const hasError = mine.some((d) => d.severity === "error");
    return {
      path,
      outcome: hasError ? ("fail" as const) : ("pass" as const),
      diagnostics: mine,
    };
  });
}

export function tally(
  evaluations: readonly Evaluation[],
): Record<Outcome, number> {
  const out: Record<Outcome, number> = {
    pass: 0,
    fail: 0,
    "could-not-run": 0,
  };
  for (const e of evaluations) out[e.outcome] += 1;
  return out;
}
