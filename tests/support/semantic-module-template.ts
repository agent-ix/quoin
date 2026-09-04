/**
 * Helpers for the semantic-module template gate (quoin FR-083).
 *
 * These live beside the test rather than under `src/` on purpose: they are the
 * harness for a template, not a surface Quoin exposes. The CONTRACT they read —
 * required surfaces, residue patterns, pinned maintained repositories,
 * exemptions — is a declared file, `templates/semantic-module/conformance.yaml`
 * (FR-083-CON-2), so a contract change is reviewable on its own.
 */
import { execFileSync } from "node:child_process";
import {
  existsSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { parse } from "yaml";

export const REPO_ROOT = resolve(
  fileURLToPath(new URL("../..", import.meta.url)),
);
export const TEMPLATE_DIR = join(REPO_ROOT, "templates", "semantic-module");
export const CONFORMANCE_PATH = join(TEMPLATE_DIR, "conformance.yaml");

export type ModuleKind = "object" | "artifact" | "mixed";

export interface MaintainedModule {
  repo: string;
  remote: string;
  revision: string;
  local_path: string;
  package: string;
  kind: ModuleKind;
}

export interface Conformance {
  contract_version: string;
  all: { required_paths: string[]; forbidden_globs: string[] };
  kinds: Record<ModuleKind, { required_paths: string[] }>;
  residue: Record<string, { pattern: string; reason: string }>;
  private_registry: {
    hosts: string[];
    publication_keys: string[];
    allowed_occurrences: { path: string; context: string; reason: string }[];
  };
  maintained_modules: MaintainedModule[];
  drift_exemptions: { path?: string; pattern?: string; reason: string }[];
}

export function loadConformance(path = CONFORMANCE_PATH): Conformance {
  if (!existsSync(path)) {
    throw new Error(
      `the conformance contract is missing at ${path}. It is a declared file, ` +
        "not a list inside a test body, so its absence is a real failure.",
    );
  }
  return parse(readFileSync(path, "utf8")) as Conformance;
}

/** Every regular file under `root`, repository-relative, ignoring VCS and installs. */
export function walkFiles(root: string, prefix = ""): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    if (entry.name === ".git" || entry.name === "node_modules") continue;
    const child = join(root, entry.name);
    const rel = prefix ? `${prefix}/${entry.name}` : entry.name;
    if (entry.isDirectory()) out.push(...walkFiles(child, rel));
    else out.push(rel);
  }
  return out.sort();
}

/** A tool this gate shells out to, and what to say when it is not there. */
export interface ToolRequirement {
  command: string;
  args: string[];
  install: string;
}

/**
 * Assert a required external command exists, FAILING with the install step when
 * it does not. Never returns a "tool absent" signal a caller could skip on: a
 * check that reports success without running is the defect this gate exists to
 * catch (NFR-020).
 */
export function requireTool({
  command,
  args,
  install,
}: ToolRequirement): string {
  try {
    return execFileSync(command, args, {
      encoding: "utf8",
      stdio: "pipe",
    }).trim();
  } catch (error) {
    throw new Error(
      `\`${command}\` is required by the semantic-module template gate and is not ` +
        `usable here. Install it with: ${install}. This is a failure and not a ` +
        `skip — a render gate that reports green with no renderer rendered ` +
        `nothing. (${(error as Error).message})`,
    );
  }
}

export interface RenderOptions {
  kind: ModuleKind;
  repoName?: string;
  extra?: Record<string, string>;
  templateDir?: string;
}

export interface Rendered {
  dir: string;
  root: string;
  repoName: string;
  packageName: string;
  kind: ModuleKind;
  dispose: () => void;
}

const DEFAULT_REPO_NAME: Record<ModuleKind, string> = {
  object: "spec-objects-example",
  artifact: "spec-artifacts-example",
  mixed: "spec-mixed-example",
};

/**
 * Render one variant into a fresh temporary directory, unattended.
 *
 * The directory is ALWAYS outside the working tree and the caller must call
 * `dispose()`; the test suite does so in a `finally`, so a failing assertion
 * leaves no residue behind either (FR-083-AC-2, FR-083-CON-1).
 */
export function render(options: RenderOptions): Rendered {
  const kind = options.kind;
  const repoName = options.repoName ?? DEFAULT_REPO_NAME[kind];
  const packageName = repoName.replace(/-/g, "_");
  const dir = mkdtempSync(join(tmpdir(), "quoin-semantic-module-"));
  const extra = Object.entries({
    module_kind: kind,
    repo_name: repoName,
    ...(kind === "mixed"
      ? { imported_modules: "agent-ix/spec-objects-business@0.3.0" }
      : {}),
    ...(options.extra ?? {}),
  }).map(([key, value]) => `${key}=${value}`);

  try {
    execFileSync(
      "cookiecutter",
      [options.templateDir ?? TEMPLATE_DIR, "--no-input", "-o", dir, ...extra],
      { encoding: "utf8", stdio: "pipe" },
    );
  } catch (error) {
    rmSync(dir, { recursive: true, force: true });
    const detail = [
      (error as { stdout?: string }).stdout,
      (error as { stderr?: string }).stderr,
    ]
      .filter(Boolean)
      .join("\n")
      .trim();
    throw new RenderRefused(detail || (error as Error).message, dir);
  }

  return {
    dir,
    root: join(dir, repoName),
    repoName,
    packageName,
    kind,
    dispose: () => rmSync(dir, { recursive: true, force: true }),
  };
}

/** A rendering the template refused, carrying the message it refused with. */
export class RenderRefused extends Error {
  readonly outputDir: string;
  constructor(message: string, outputDir: string) {
    super(message);
    this.name = "RenderRefused";
    this.outputDir = outputDir;
  }
}

/** Render and dispose, whatever the body does. */
export function withRendered<T>(
  options: RenderOptions,
  body: (rendered: Rendered) => T,
): T {
  const rendered = render(options);
  try {
    return body(rendered);
  } finally {
    rendered.dispose();
  }
}

export function requiredPathsFor(
  conformance: Conformance,
  kind: ModuleKind,
  packageName: string,
): string[] {
  return [
    ...conformance.all.required_paths,
    ...conformance.kinds[kind].required_paths,
  ].map((path) => path.replace("{package}", packageName));
}

/** Surfaces the contract requires that the rendered tree does not carry. */
export function missingSurfaces(
  conformance: Conformance,
  rendered: Rendered,
): string[] {
  const missing: string[] = [];
  for (const required of requiredPathsFor(
    conformance,
    rendered.kind,
    rendered.packageName,
  )) {
    const target = join(rendered.root, required.replace(/\/$/, ""));
    if (!existsSync(target)) {
      missing.push(required);
      continue;
    }
    if (required.endsWith("/") && readdirSync(target).length === 0) {
      missing.push(`${required} (empty)`);
    }
  }
  return missing;
}

export interface ResidueHit {
  file: string;
  klass: string;
  match: string;
}

const BINARY_SUFFIXES = [".png", ".jpg", ".gif", ".ico", ".woff", ".woff2"];

/**
 * Compile a declared pattern. A leading `(?i)` is written that way because the
 * contract is read by more than one language; JavaScript spells it as a flag.
 */
export function compile(pattern: string): RegExp {
  return pattern.startsWith("(?i)")
    ? new RegExp(pattern.slice(4), "i")
    : new RegExp(pattern);
}

/** Every residue-class match in the rendered tree, by file and class. */
export function residueHits(
  conformance: Conformance,
  rendered: Rendered,
): ResidueHit[] {
  const hits: ResidueHit[] = [];
  for (const file of walkFiles(rendered.root)) {
    if (BINARY_SUFFIXES.some((suffix) => file.endsWith(suffix))) continue;
    const text = readFileSync(join(rendered.root, file), "utf8");
    for (const [klass, { pattern }] of Object.entries(conformance.residue)) {
      const match = compile(pattern).exec(text);
      if (match) hits.push({ file, klass, match: match[0] });
    }
  }
  return hits;
}

/**
 * Private-registry hosts that appear anywhere other than a declared, allowed
 * occurrence. The class is publication configuration, not every mention: the
 * rendered `dev-quire` command names one host deliberately, because no index the
 * repository may depend on serves the engine wheel.
 */
export function privateRegistryHits(
  conformance: Conformance,
  rendered: Rendered,
): ResidueHit[] {
  const allowed = new Set(
    conformance.private_registry.allowed_occurrences.map((entry) => entry.path),
  );
  const hits: ResidueHit[] = [];
  for (const file of walkFiles(rendered.root)) {
    const text = readFileSync(join(rendered.root, file), "utf8");
    for (const host of conformance.private_registry.hosts) {
      if (!text.includes(host)) continue;
      if (allowed.has(file)) continue;
      hits.push({ file, klass: "private_registry", match: host });
    }
  }
  return hits;
}

/** Files under a maintained module repository at its pinned revision. */
export function locateMaintained(module: MaintainedModule): string {
  // A git worktree puts this repository one or two levels deeper than the
  // sibling checkout, so the search walks up rather than assuming a depth.
  const candidates = [
    resolve(REPO_ROOT, module.local_path),
    resolve(REPO_ROOT, "..", module.repo),
    resolve(REPO_ROOT, "..", "..", module.repo),
    resolve(REPO_ROOT, "..", "..", "..", module.repo),
  ];
  const found = candidates.find((path) => existsSync(join(path, ".git")));
  if (!found) {
    throw new Error(
      `${module.repo} is not available beside this repository, so the drift ` +
        `check cannot read it. Clone ${module.remote}. Looked in: ` +
        `${candidates.join(", ")}. Reporting "no drift" without reading it ` +
        "would be reporting on nothing.",
    );
  }
  return found;
}

export function maintainedSurfaces(module: MaintainedModule): string[] {
  const root = locateMaintained(module);
  let listing: string;
  try {
    listing = execFileSync(
      "git",
      ["-C", root, "ls-tree", "-r", "--name-only", module.revision],
      { encoding: "utf8", stdio: "pipe" },
    );
  } catch (error) {
    throw new Error(
      `${module.repo} cannot be read at its pinned revision ${module.revision}. ` +
        "Two runs of the drift check must read the same bytes, so an unreachable " +
        `revision is a failure rather than a clean result. (${(error as Error).message})`,
    );
  }
  return listing.split("\n").filter(Boolean).sort();
}

/**
 * Surfaces every maintained module carries that the contract neither requires
 * nor exempts. A shared surface the template omits is drift; a surface only one
 * of them carries is that repository's own business.
 */
export function driftedSurfaces(conformance: Conformance): string[] {
  const perModule = conformance.maintained_modules.map((module) => {
    const generic = new Set<string>();
    for (const path of maintainedSurfaces(module)) {
      generic.add(
        path.replace(new RegExp(`^${module.package}/`), "{package}/"),
      );
    }
    return generic;
  });
  if (perModule.length === 0) return [];

  const required = new Set(
    conformance.all.required_paths.flatMap((path) => [
      path,
      ...Object.values(conformance.kinds).flatMap(
        (kind) => kind.required_paths,
      ),
    ]),
  );
  for (const kind of Object.values(conformance.kinds)) {
    for (const path of kind.required_paths) required.add(path);
  }
  const exemptPaths = conformance.drift_exemptions
    .map((entry) => entry.path)
    .filter((path): path is string => typeof path === "string");
  const exemptPatterns = conformance.drift_exemptions
    .map((entry) => entry.pattern)
    .filter((pattern): pattern is string => typeof pattern === "string")
    .map((pattern) => new RegExp(pattern));

  const shared = [...perModule[0]].filter((path) =>
    perModule.every((module) => module.has(path)),
  );

  return shared
    .filter((path) => !required.has(path))
    .filter(
      (path) =>
        !exemptPaths.some((entry) =>
          entry.endsWith("/") ? path.startsWith(entry) : path === entry,
        ),
    )
    .filter((path) => !exemptPatterns.some((pattern) => pattern.test(path)))
    .sort();
}

export function relativeToRepo(path: string): string {
  return relative(REPO_ROOT, path);
}

export function isDirectory(path: string): boolean {
  return existsSync(path) && statSync(path).isDirectory();
}
