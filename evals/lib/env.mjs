// Per-scenario workspace: an isolated repo + IX_HOME, a pre-generated session id,
// and the predetermined Claude Code transcript path.

import { cpSync, existsSync, mkdirSync, mkdtempSync, rmSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { randomUUID } from "node:crypto";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { snapshotInto } from "./seed.mjs";

// The repo's working-tree skills (evals/lib/env.mjs -> repo root -> skills/).
// Injected into each scenario as project skills so evals exercise the CURRENT
// skills (e.g. /specify) rather than whatever is globally installed.
const REPO_SKILLS = join(
  dirname(fileURLToPath(import.meta.url)),
  "..",
  "..",
  "skills",
);

/**
 * Claude Code's project-dir slug for a cwd: every non-alphanumeric character
 * becomes `-`, case preserved. (Verified against live transcript dir names.)
 */
export function slug(absPath) {
  return absPath.replace(/[^a-zA-Z0-9]/g, "-");
}

/** Where Claude Code writes the transcript for a session run in `cwd`. */
export function transcriptPathFor(cwd, sessionId) {
  return join(
    homedir(),
    ".claude",
    "projects",
    slug(cwd),
    `${sessionId}.jsonl`,
  );
}

/**
 * Create an isolated workspace for one scenario run.
 *
 * `cwd` (agent working dir) and `scope` (validation/file-assertion root) both
 * default to the single `repo`. A multi-repo scenario's setup can repoint them at
 * `work` (the parent holding several sub-repos). `transcriptPath` is finalized from
 * `cwd` by the runner AFTER setup, so a setup that changes `cwd` still resolves it.
 * @returns {{ id, work, repo, cwd, scope, ixHome, modulesDir, sessionId, transcriptPath, data, cleanup }}
 */
export function makeScenarioWorkspace(id) {
  const work = mkdtempSync(join(tmpdir(), `quoin-${id.toLowerCase()}-`));
  const repo = join(work, "repo");
  const ixHome = join(work, "ix-home");
  mkdirSync(join(repo, "spec"), { recursive: true });
  mkdirSync(ixHome, { recursive: true });
  // Project-scoped skills: each harness loads the working-tree `/specify` etc.
  // instead of a globally-installed plugin copy. Both directories are dotdirs,
  // so file/artifact assertions and quire's spec globs skip them.
  if (existsSync(REPO_SKILLS)) {
    cpSync(REPO_SKILLS, join(repo, ".claude", "skills"), { recursive: true });
    cpSync(REPO_SKILLS, join(repo, ".agents", "skills"), { recursive: true });
  }
  const modulesDir = snapshotInto(ixHome);
  const sessionId = randomUUID();
  return {
    id,
    work,
    repo,
    cwd: repo, // agent's working dir (setup may repoint to `work`)
    scope: repo, // validation + file-assertion root (setup may repoint to `work`)
    ixHome,
    modulesDir,
    sessionId,
    transcriptPath: transcriptPathFor(repo, sessionId),
    data: {}, // scratch for setup(ctx) -> env(ctx)/prompt(ctx) handoff
    cleanup() {
      rmSync(work, { recursive: true, force: true });
    },
  };
}
