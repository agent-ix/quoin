#!/usr/bin/env node

// Builtins only: canonical source preflight runs before package installation.
import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const V1_SOURCES = [
  "quoin",
  "quire",
  "quire-cli",
  "qa-corpus",
  "filament-ide-rs",
  "spec-artifacts-process",
  "spec-artifacts-iso",
];
export const V2_SOURCES = [...V1_SOURCES, "engineering-assurance"];
const ROUTES = [
  ["spec-artifacts-process", "spec_artifacts_process"],
  ["spec-artifacts-iso", "spec_artifacts_iso"],
  ["engineering-assurance", "engineering_assurance"],
];
const FULL_SHA = /^[0-9a-f]{40}$/;
const DIGEST = /^sha256:[0-9a-f]{64}$/;
const hash = (bytes) =>
  `sha256:${createHash("sha256").update(bytes).digest("hex")}`;
const relativePath = (path) =>
  typeof path === "string" &&
  path.length > 0 &&
  !isAbsolute(path) &&
  !/[\\\0]/.test(path) &&
  path.split("/").every((part) => part !== "" && part !== "." && part !== "..");
const keys = (value, names) =>
  value &&
  typeof value === "object" &&
  !Array.isArray(value) &&
  Object.keys(value).sort().join() === [...names].sort().join();

export function sourceNames(lock) {
  if (lock?.schemaVersion === "quoin-verification-stack-lock-v1")
    return V1_SOURCES;
  if (lock?.schemaVersion === "quoin-verification-stack-lock-v2")
    return V2_SOURCES;
  throw new Error("verification lock has unsupported schemaVersion");
}

// TC-1594 / FR-043-AC-34: v2 has one explicit, closed validation policy.
export function validateDeclarationShape(lock) {
  if (lock.schemaVersion === "quoin-verification-stack-lock-v1") {
    if (lock.declarations !== undefined)
      throw new Error("historical v1 cannot claim v2 declaration isolation");
    return;
  }
  if (!keys(lock.repositories, V2_SOURCES))
    throw new Error("v2 requires exactly eight known source repositories");
  if (!keys(lock.declarations, ["quoinValidation"]))
    throw new Error("v2 requires an explicit quoinValidation declaration set");
  const entries = lock.declarations.quoinValidation;
  if (!Array.isArray(entries) || entries.length !== ROUTES.length)
    throw new Error(
      "v2 requires the complete ordered process, ISO, engineering-assurance declaration set",
    );
  for (const [index, entry] of entries.entries()) {
    if (
      !keys(entry, ["repository", "path", "tree", "files"]) ||
      entry.repository !== ROUTES[index][0] ||
      entry.path !== ROUTES[index][1] ||
      !FULL_SHA.test(entry.tree ?? "")
    )
      throw new Error(`invalid declaration route or tree at index ${index}`);
    if (!Array.isArray(entry.files) || !entry.files.length)
      throw new Error("declaration inventory must be non-empty");
    let previous = null;
    for (const file of entry.files) {
      if (
        !keys(file, ["path", "mode", "digest"]) ||
        !relativePath(file.path) ||
        !["100644", "100755"].includes(file.mode) ||
        !DIGEST.test(file.digest ?? "") ||
        (previous !== null && previous >= file.path)
      )
        throw new Error(
          "declaration inventory requires sorted unique safe paths, regular modes and full digests",
        );
      previous = file.path;
    }
    if (!entry.files.some((file) => file.path === "manifest.yaml"))
      throw new Error("declaration inventory is missing manifest.yaml");
  }
}

function git(root, args, timeout) {
  return execFileSync("git", ["-C", root, ...args], {
    timeout,
    maxBuffer: 128 * 1024 * 1024,
    stdio: ["ignore", "pipe", "pipe"],
  });
}

/** Complete literal Git tree, bounded without filters, export attributes or working files. */
export function committedTree(root, revision, path = "", timeout = 120_000) {
  if (!FULL_SHA.test(revision) || (path !== "" && !relativePath(path)))
    throw new Error(
      "committed snapshot requires a full revision and safe relative path",
    );
  const selection = path ? `${revision}:${path}` : `${revision}^{tree}`;
  const tree = git(root, ["rev-parse", "--verify", selection], timeout)
    .toString()
    .trim();
  if (
    !FULL_SHA.test(tree) ||
    git(root, ["cat-file", "-t", tree], timeout).toString().trim() !== "tree"
  )
    throw new Error("committed snapshot selection is not a tree");
  const entries = git(root, ["ls-tree", "-r", "-z", tree], timeout)
    .toString()
    .split("\0")
    .filter(Boolean)
    .map((row) => {
      const separator = row.indexOf("\t");
      const [mode, kind, object] = row.slice(0, separator).split(" ");
      const file = row.slice(separator + 1);
      if (
        !["100644", "100755"].includes(mode) ||
        kind !== "blob" ||
        !relativePath(file)
      )
        throw new Error(
          `committed snapshot refuses non-regular or unsafe Git input ${file}`,
        );
      return { path: file, mode, object };
    })
    .sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
  if (!entries.length) throw new Error("committed snapshot tree is empty");
  const objects = execFileSync("git", ["-C", root, "cat-file", "--batch"], {
    input: entries.map((entry) => entry.object).join("\n") + "\n",
    timeout,
    maxBuffer: 128 * 1024 * 1024,
    stdio: ["pipe", "pipe", "pipe"],
  });
  let offset = 0;
  const files = entries.map((entry) => {
    const end = objects.indexOf(10, offset);
    const [object, kind, sizeText] = objects
      .subarray(offset, end)
      .toString()
      .split(" ");
    const size = Number(sizeText);
    if (
      end < 0 ||
      object !== entry.object ||
      kind !== "blob" ||
      !Number.isSafeInteger(size) ||
      size < 0 ||
      end + size + 1 >= objects.length ||
      objects[end + size + 1] !== 10
    )
      throw new Error("malformed Git object snapshot batch");
    const bytes = objects.subarray(end + 1, end + size + 1);
    offset = end + size + 2;
    return { path: entry.path, mode: entry.mode, bytes };
  });
  if (offset !== objects.length)
    throw new Error("unexpected trailing Git snapshot bytes");
  return { tree, files };
}

export function writeCommittedTree(snapshot, destination) {
  // The caller owns a fresh scratch parent. Never merge with an existing directory.
  mkdirSync(destination);
  for (const file of snapshot.files) {
    const target = resolve(destination, file.path);
    if (!target.startsWith(`${resolve(destination)}/`))
      throw new Error("snapshot path escapes destination");
    mkdirSync(dirname(target), { recursive: true });
    writeFileSync(target, file.bytes, {
      flag: "wx",
      mode: file.mode === "100755" ? 0o755 : 0o644,
    });
  }
}

function inventory(snapshot) {
  return {
    tree: snapshot.tree,
    files: snapshot.files.map(({ path, mode, bytes }) => ({
      path,
      mode,
      digest: hash(bytes),
    })),
  };
}

export function describeModule(root, revision, path, timeout) {
  return inventory(committedTree(root, revision, path, timeout));
}

// TC-1595 / FR-043-AC-35: verify all declarations before materializing any.
export function verifyDeclarations(lock, roots) {
  validateDeclarationShape(lock);
  if (lock.schemaVersion !== "quoin-verification-stack-lock-v2")
    throw new Error("explicit declaration materialization requires v2");
  return lock.declarations.quoinValidation.map((entry) => {
    if (!roots[entry.repository] || !isAbsolute(roots[entry.repository]))
      throw new Error(
        `missing explicit declaration source ${entry.repository}`,
      );
    const snapshot = committedTree(
      roots[entry.repository],
      lock.repositories[entry.repository].revision,
      entry.path,
      lock.timeouts.installMilliseconds,
    );
    const actual = inventory(snapshot);
    if (
      actual.tree !== entry.tree ||
      actual.files.length !== entry.files.length ||
      actual.files.some((file, index) =>
        ["path", "mode", "digest"].some(
          (key) => file[key] !== entry.files[index][key],
        ),
      )
    )
      throw new Error(
        `declaration tree or file inventory drift: ${entry.repository}`,
      );
    return snapshot;
  });
}

export function materializeDeclarations(lock, roots, destination) {
  const snapshots = verifyDeclarations(lock, roots);
  mkdirSync(destination);
  return snapshots.map((snapshot, index) => {
    const root = join(destination, String(index));
    writeCommittedTree(snapshot, root);
    return root;
  });
}

// Native validation is still the verdict owner. This is argument routing, not an assessor.
export function runExactValidation(quire, moduleRoots, options = {}) {
  if (
    !Array.isArray(moduleRoots) ||
    !moduleRoots.length ||
    moduleRoots.some((root) => typeof root !== "string" || !isAbsolute(root)) ||
    new Set(moduleRoots).size !== moduleRoots.length
  )
    throw new Error(
      "explicit validation requires non-empty unique absolute module roots",
    );
  const env = { ...(options.env ?? process.env) };
  delete env.QUOIN_MODULE_PATHS;
  delete env.IX_FILAMENT_MODULES_PATH;
  const home = mkdtempSync(join(tmpdir(), "quoin-validation-home-"));
  try {
    env.IX_HOME = home;
    const result = spawnSync(
      quire,
      [
        "validate",
        "spec/**/*.md",
        "plan/**/*.md",
        "reviews/*.md",
        ...moduleRoots.flatMap((root) => ["--module", root]),
      ],
      {
        cwd: options.cwd,
        env,
        timeout: options.timeout ?? 600_000,
        stdio: "inherit",
      },
    );
    if (result.error || result.status !== 0)
      throw new Error(
        `exact native validation failed (${result.status ?? result.error?.message})`,
      );
  } finally {
    rmSync(home, { recursive: true, force: true });
  }
}

// Existing Make validate invokes this only when canonical v2 supplies its manifest.
if (resolve(process.argv[1] ?? "") === fileURLToPath(import.meta.url)) {
  try {
    if (process.argv.length !== 4)
      throw new Error("expected QUIRE and explicit module-root manifest");
    const roots = JSON.parse(readFileSync(process.argv[3], "utf8"));
    runExactValidation(process.argv[2], roots, { cwd: process.cwd() });
  } catch (error) {
    console.error(`verification-declarations: ${error.message}`);
    process.exitCode = 1;
  }
}
