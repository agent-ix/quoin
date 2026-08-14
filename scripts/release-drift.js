#!/usr/bin/env node
/**
 * Release drift checks
 *
 * Two independent guards against publishing something that does not match main:
 *
 *   check  Fails while main is ahead of the latest release tag in any path that
 *          affects published behaviour. The path list is DERIVED from the
 *          package.json `files` array (with dist/ mapped back to its source,
 *          src/) so it cannot go stale when the shipped file set changes.
 *
 *   pins   Reports each default-modules.yaml pin against that repo's latest
 *          tag. Reports only — pinning behind latest is sometimes deliberate,
 *          and the value is knowing rather than being forced.
 */

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { parse } from "yaml";

// Tests point this at a fixture repository; everything else runs against the
// checkout this script lives in.
const repoRoot =
  process.env.QUOIN_DRIFT_ROOT ??
  resolve(dirname(fileURLToPath(import.meta.url)), "..");

/** Shipped paths that are built rather than committed, and their source. */
const BUILD_OUTPUT_SOURCES = { "dist/": "src" };

/**
 * Release-relevant paths, derived from what the package actually ships.
 *
 * package.json itself is always included: it carries the version, the bin map,
 * the oclif manifest and the dependency ranges, none of which live under a
 * `files` entry.
 */
export function releasePaths(pkg) {
  const paths = new Set(["package.json"]);
  for (const entry of pkg.files ?? []) {
    const mapped = BUILD_OUTPUT_SOURCES[entry] ?? entry;
    paths.add(mapped.replace(/\/$/, ""));
  }
  return [...paths].sort();
}

/**
 * Compare pinned refs against the latest tag known for each source repo.
 *
 * `latestTags` maps `owner/repo` to its newest tag. A repo missing from the map
 * is reported as `unknown` rather than treated as current — a lookup failure
 * must not read as a pass.
 */
export function comparePins(entries, latestTags) {
  return (entries ?? []).map((entry) => {
    const url = entry.source?.url ?? "";
    const pinned = entry.source?.ref ?? "";
    const latest = latestTags[url];
    let state = "current";
    if (!latest) {
      state = "unknown";
    } else if (latest !== pinned) {
      state = "behind";
    }
    return { name: entry.name, url, pinned, latest: latest ?? "", state };
  });
}

function git(args, options = {}) {
  return execFileSync("git", args, {
    cwd: repoRoot,
    encoding: "utf8",
    ...options,
  }).trim();
}

function latestTagOf(url) {
  // `git ls-remote --tags --sort=-v:refname` gives newest-first without cloning.
  const output = execFileSync(
    "git",
    [
      "ls-remote",
      "--tags",
      "--refs",
      "--sort=-v:refname",
      `https://github.com/${url}.git`,
    ],
    { encoding: "utf8" },
  );
  const first = output.split("\n").find((line) => line.includes("refs/tags/"));
  return first ? first.split("refs/tags/")[1].trim() : "";
}

function resolveLatestTags(entries) {
  // Tests and offline runs inject the lookup result instead of hitting the network.
  const override = process.env.QUOIN_DRIFT_LATEST_TAGS;
  if (override) {
    return JSON.parse(override);
  }
  const tags = {};
  for (const entry of entries) {
    const url = entry.source?.url;
    if (!url || url in tags) continue;
    try {
      tags[url] = latestTagOf(url);
    } catch {
      // Leave it absent: comparePins reports `unknown`, not a false pass.
    }
  }
  return tags;
}

function readPackage() {
  return JSON.parse(readFileSync(join(repoRoot, "package.json"), "utf8"));
}

function readModules() {
  return parse(readFileSync(join(repoRoot, "default-modules.yaml"), "utf8"));
}

const commands = {
  paths() {
    for (const path of releasePaths(readPackage())) {
      console.log(path);
    }
  },

  check() {
    const paths = releasePaths(readPackage());
    let lastTag = "";
    try {
      lastTag = git(["describe", "--tags", "--abbrev=0"], { stdio: "pipe" });
    } catch {
      lastTag = "";
    }
    if (!lastTag) {
      console.log("No release tags yet; nothing to compare against.");
      return 0;
    }

    const changed = git([
      "diff",
      "--name-only",
      `${lastTag}..HEAD`,
      "--",
      ...paths,
    ]);
    if (!changed) {
      console.log(
        `✅ No release-relevant changes since ${lastTag} — main is released.`,
      );
      console.log(`Watched paths: ${paths.join(" ")}`);
      return 0;
    }

    console.log(
      `::error::Release-relevant files have changed since ${lastTag}. Tag a release (vX.Y.Z) so the published package matches main.`,
    );
    console.log(`Changed since ${lastTag}:`);
    console.log(changed);
    return 1;
  },

  pins() {
    const modules = readModules();
    const entries = modules.entries ?? [];
    const rows = comparePins(entries, resolveLatestTags(entries));

    const width = Math.max(4, ...rows.map((row) => row.name.length));
    console.log(
      `${"name".padEnd(width)}  ${"pinned".padEnd(10)}  ${"latest".padEnd(10)}  state`,
    );
    for (const row of rows) {
      console.log(
        `${row.name.padEnd(width)}  ${row.pinned.padEnd(10)}  ${(row.latest || "-").padEnd(10)}  ${row.state}`,
      );
    }

    for (const row of rows.filter((r) => r.state === "behind")) {
      console.log(
        `::warning::${row.name} is pinned at ${row.pinned} but ${row.url} has ${row.latest}.`,
      );
    }
    for (const row of rows.filter((r) => r.state === "unknown")) {
      console.log(
        `::warning::Could not resolve the latest tag for ${row.url} (${row.name}); pin currency is unverified.`,
      );
    }

    const behind = rows.filter((r) => r.state === "behind").length;
    const unknown = rows.filter((r) => r.state === "unknown").length;
    console.log(
      `\n${rows.length - behind - unknown} current, ${behind} behind, ${unknown} unresolved.`,
    );
    // Report-only: a deliberate lag must not fail the run.
    return 0;
  },

  help() {
    console.log("Release Drift - publish-readiness checks");
    console.log("");
    console.log("Usage: release-drift <command>");
    console.log("");
    console.log("Commands:");
    console.log("  check   Fail if release-relevant paths changed since the");
    console.log("          latest tag (paths derived from package.json files)");
    console.log("  paths   Print the derived release-relevant path list");
    console.log("  pins    Report default-modules.yaml pins vs latest tags");
    console.log("  help    Show this help message");
    return 0;
  },
};

// Only run when invoked directly, so the helpers above stay importable.
if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const cmd = process.argv[2];
  if (!cmd || cmd === "--help" || cmd === "-h") {
    process.exit(commands.help());
  } else if (!commands[cmd]) {
    console.error(`Error: Unknown command '${cmd}'`);
    console.error("");
    commands.help();
    process.exit(1);
  } else {
    process.exit(commands[cmd]() ?? 0);
  }
}
