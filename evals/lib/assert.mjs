// Ground-truth success assertion. The harness independently re-runs the relevant
// CLI/validation (the agent's own runs inside the transcript can lie or be skipped),
// and keys `quire validate` pass/fail on EXIT CODE only — `DuplicateArchetype`
// warnings on stderr coexist with a passing (exit 0) validation.

import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { spawnSync } from "node:child_process";

import { findQuire, ixSpecBin } from "./resolve.mjs";
import { findCommand } from "./metrics.mjs";

/** Recursively list repo-relative file paths (posix-style), skipping dotdirs. */
function listFiles(root, dir = root, acc = []) {
  for (const name of readdirSync(dir)) {
    if (name.startsWith(".")) continue;
    const abs = join(dir, name);
    if (statSync(abs).isDirectory()) listFiles(root, abs, acc);
    else acc.push(relative(root, abs).split(sep).join("/"));
  }
  return acc;
}

/** Glob -> RegExp supporting `**`, `*`, `?`. */
function globToRegExp(glob) {
  // Brace expansion is NOT supported, and silently escaping `{`/`}` compiled
  // `*.{js,ts,mjs}` to a literal-suffix match that can never hit a real file.
  // In `fileContains` that fails loudly; in `absentFiles` nothing matches, so
  // nothing is "present", so an assertion written to prove a file was NOT
  // created passes whether or not it was (agent-ix/quoin#135). A glob that
  // cannot express what its author meant must fail at load, not pass at run.
  if (glob.includes("{") || glob.includes("}")) {
    throw new Error(
      `glob braces are not supported: ${glob} — write one glob per alternative instead`,
    );
  }
  let re = "";
  for (let i = 0; i < glob.length; i++) {
    const c = glob[i];
    if (c === "*") {
      if (glob[i + 1] === "*") {
        i++;
        if (glob[i + 1] === "/") i++;
        re += "(?:.*/)?";
      } else {
        re += "[^/]*";
      }
    } else if (c === "?") re += "[^/]";
    else re += c.replace(/[.+^${}()|[\]\\]/g, "\\$&");
  }
  // case-insensitive: agents legitimately name files `fr-001.md` (matching the
  // lowercase skeletons) or `FR-001.md` (canonical type casing) — both should match.
  return new RegExp(`^${re}$`, "i");
}

export function matchFiles(repo, glob) {
  const re = globToRegExp(glob);
  return listFiles(repo).filter((f) => re.test(f));
}

/** Read the frontmatter `type:` discriminator of a markdown file (null if none). */
function frontmatterType(abs) {
  let text;
  try {
    text = readFileSync(abs, "utf8");
  } catch {
    return null;
  }
  if (!text.startsWith("---")) return null;
  const end = text.indexOf("\n---", 3);
  if (end === -1) return null;
  const m = text
    .slice(3, end)
    .match(/^type:\s*["']?([A-Za-z0-9_-]+)["']?\s*$/m);
  return m ? m[1] : null;
}

/** Map every `*.md` under root to its frontmatter `type` (repo-relative path -> type). */
function artifactsByType(root) {
  const byType = {};
  for (const rel of listFiles(root)) {
    if (!rel.toLowerCase().endsWith(".md")) continue;
    const t = frontmatterType(join(root, rel));
    if (t) (byType[t.toLowerCase()] ??= []).push(rel);
  }
  return byType;
}

function ixSpec(ctx, args, env) {
  return spawnSync(process.execPath, [ixSpecBin(), ...args], {
    cwd: ctx.repo,
    encoding: "utf8",
    env,
  });
}

/**
 * Assert a scenario's expectations against the final repo + config state.
 * @param expect {
 *   files?: string[], absentFiles?: string[], validate?: {globs, shouldPass},
 *   artifacts?: {require?: {[TYPE]: {min?, dir?}}, absent?: string[]},
 *   flow?: [{id, defName}], cliRejects?: string[],
 *   plugin?: {name, present}, resolvesTo?: {type, moduleNameIncludes},
 *   fileContains?: [{glob, includes?: string[], excludes?: string[]}],
 *   sentinel?: "complete"|"failed"
 * }
 * @returns { ok, failures, validation, matchedFiles, checks }
 */
export function assertExpectations(
  ctx,
  expect,
  runResult,
  { quireBin = findQuire(), modulesDir, extraEnv = {} } = {},
) {
  const env = {
    ...process.env,
    IX_HOME: ctx.ixHome,
    IX_SCHEMA_PATH: modulesDir,
    ...extraEnv,
  };
  const scope = ctx.scope ?? ctx.repo; // validation + file-assertion root
  const failures = [];
  const matchedFiles = {};
  const checks = {};

  for (const glob of expect.files ?? []) {
    const hits = matchFiles(scope, glob);
    matchedFiles[glob] = hits.length;
    if (hits.length === 0)
      failures.push(`no file matched expected glob: ${glob}`);
  }

  // Negative file globs: assert NO file matches (e.g. an update-in-place scenario
  // must not spawn a second plan bundle).
  for (const glob of expect.absentFiles ?? []) {
    const hits = matchFiles(scope, glob);
    matchedFiles[`!${glob}`] = hits.length;
    if (hits.length > 0)
      failures.push(
        `expected no file matching ${glob}, found ${hits.length}: ${hits.join(", ")}`,
      );
  }

  // Artifact-completeness: a request must produce the right artifact TYPES as
  // discrete files (an FR authored only as a row in spec.md's table is NOT an FR
  // artifact and must fail here). Keyed on frontmatter `type:`.
  if (expect.artifacts) {
    const { require: required = {}, absent = [] } = expect.artifacts;
    const byType = artifactsByType(scope);
    const pathsFor = (type) => byType[type.toLowerCase()] ?? [];
    for (const [type, spec] of Object.entries(required)) {
      const { min = 1, max, dir } = spec ?? {};
      let paths = pathsFor(type);
      if (dir) {
        const prefix = dir.replace(/\/+$/, "") + "/";
        paths = paths.filter((p) => p.startsWith(prefix));
      }
      const okMin = paths.length >= min;
      const okMax = max === undefined || paths.length <= max;
      checks[`artifacts:${type}`] = { ok: okMin && okMax, paths };
      if (!okMin) {
        failures.push(
          `expected >=${min} ${type} artifact(s)${dir ? ` under ${dir}/` : ""}, found ${paths.length}`,
        );
      }
      if (!okMax) {
        failures.push(
          `expected <=${max} ${type} artifact(s)${dir ? ` under ${dir}/` : ""}, found ${paths.length}: ${paths.join(", ")}`,
        );
      }
    }
    for (const type of absent) {
      const paths = pathsFor(type);
      checks[`artifacts:absent:${type}`] = { ok: paths.length === 0, paths };
      if (paths.length > 0) {
        failures.push(
          `expected no ${type} artifact but found ${paths.length}: ${paths.join(", ")}`,
        );
      }
    }
  }

  // Content assertions: the first file matching `glob` must match every `includes`
  // regex and no `excludes` regex (case-insensitive). Used to assert a SpecReview's
  // Verdict (PASS/CONDITIONAL/FAIL) and that specific findings were recorded.
  for (const fc of expect.fileContains ?? []) {
    const { glob, includes = [], excludes = [] } = fc;
    const hits = matchFiles(scope, glob);
    const text = hits.length ? readFileSync(join(scope, hits[0]), "utf8") : "";
    const missing = includes.filter((p) => !new RegExp(p, "i").test(text));
    const unexpected = excludes.filter((p) => new RegExp(p, "i").test(text));
    const ok =
      hits.length > 0 && missing.length === 0 && unexpected.length === 0;
    checks[`fileContains:${glob}`] = {
      ok,
      file: hits[0] ?? null,
      missing,
      unexpected,
    };
    if (!ok) {
      if (hits.length === 0)
        failures.push(`fileContains: no file matched ${glob}`);
      if (missing.length)
        failures.push(`fileContains ${glob}: missing ${missing.join(", ")}`);
      if (unexpected.length)
        failures.push(
          `fileContains ${glob}: unexpected ${unexpected.join(", ")}`,
        );
    }
  }

  let validation = null;
  if (expect.validate) {
    const { globs, shouldPass = true, strict = false } = expect.validate;
    // `strict: true` adds `--strict`, which escalates advisory warnings —
    // including EARS requirement-grammar findings (FR-042) — to a failing
    // exit. So `{ strict: true, shouldPass: true }` asserts the docs are both
    // structurally valid AND grammar-clean.
    const validateArgs = ["validate", "--scope", scope, ...globs];
    if (strict) validateArgs.push("--strict");
    const r = spawnSync(quireBin, validateArgs, {
      cwd: scope,
      encoding: "utf8",
      env,
    });
    const passed = (r.status ?? 1) === 0;
    validation = {
      exitCode: r.status ?? null,
      passed,
      stderr: (r.stderr ?? "").slice(-2000),
    };
    if (passed !== shouldPass) {
      failures.push(
        `validation expected pass=${shouldPass} but got pass=${passed}`,
      );
    }
  }

  for (const { id, defName } of expect.flow ?? []) {
    const r = spawnSync(
      "ix-flow",
      ["status", id, "--json", "--config-root", ctx.ixHome],
      { cwd: ctx.repo, encoding: "utf8", env },
    );
    let ok = false;
    let actual = null;
    try {
      const data = JSON.parse(r.stdout || "{}");
      actual = data?.data?.defName ?? data?.summary?.defName;
      ok = (r.status ?? 1) === 0 && actual === defName;
    } catch {
      // non-JSON => failure below
    }
    checks[`flow:${id}`] = { ok, defName: actual, exitCode: r.status };
    if (!ok)
      failures.push(`flow ${id} not running as ${defName} (got ${actual})`);
  }

  for (const type of expect.cliRejects ?? []) {
    const r = ixSpec(ctx, ["write", ctx.repo, "--types", type], env);
    const rejected = (r.status ?? 0) !== 0;
    checks[`cliRejects:${type}`] = { ok: rejected, exitCode: r.status };
    if (!rejected) failures.push(`CLI did not reject unknown type: ${type}`);
  }

  if (expect.plugin) {
    const { name, present = true } = expect.plugin;
    const r = ixSpec(ctx, ["plugin", "list", "--json"], env);
    let found = false;
    try {
      found = (JSON.parse(r.stdout || "{}").plugins ?? []).some(
        (p) => p.name === name,
      );
    } catch {
      // parse failure => found stays false
    }
    checks[`plugin:${name}`] = { ok: found === present, present: found };
    if (found !== present)
      failures.push(`plugin ${name} present=${found}, expected ${present}`);
  }

  if (expect.resolvesTo) {
    const { type, moduleNameIncludes } = expect.resolvesTo;
    const r = ixSpec(ctx, ["write", ctx.repo, "--types", type, "--json"], env);
    let ok = false;
    let moduleRoot = null;
    try {
      const pack = JSON.parse(r.stdout || "{}");
      moduleRoot = pack.types?.[0]?.moduleRoot ?? "";
      ok = moduleRoot.includes(moduleNameIncludes);
    } catch {
      // parse failure => ok stays false
    }
    checks[`resolvesTo:${type}`] = { ok, moduleRoot };
    if (!ok)
      failures.push(
        `${type} did not resolve to a module under "${moduleNameIncludes}" (got ${moduleRoot})`,
      );
  }

  for (const { pattern, desc } of expect.agentRan ?? []) {
    const { ran, succeeded } = findCommand(ctx.transcriptPath, pattern);
    checks[`agentRan:${desc}`] = { ran, succeeded };
    if (!ran || !succeeded)
      failures.push(`agent did not successfully run: ${desc}`);
  }

  const wantSentinel = expect.sentinel ?? "complete";
  if (runResult.exitReason !== wantSentinel) {
    failures.push(
      `expected sentinel "${wantSentinel}" but run ended "${runResult.exitReason}"`,
    );
  }

  return {
    ok: failures.length === 0,
    failures,
    validation,
    matchedFiles,
    checks,
  };
}
