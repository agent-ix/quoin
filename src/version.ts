import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

// Baked at build time from `git describe` (see vite.config.ts). A bare semver
// means a clean tagged release; a `-<n>-g<sha>` suffix means the build is ahead
// of its tag. Empty for dev/test/no-git builds, which fall back to package.json.
declare const __QUOIN_VERSION__: string;

function readPackageJsonVersion(): string {
  const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
  const packageJson = JSON.parse(
    readFileSync(join(packageRoot, "package.json"), "utf8"),
  ) as { version?: unknown };
  if (typeof packageJson.version !== "string") {
    throw new Error("package.json version is missing");
  }
  return packageJson.version;
}

// Prefer the build-time baked version (truthful about drift); fall back to
// package.json when it is absent (dev/test builds, or a no-git build).
export function resolveVersion(baked: string): string {
  if (baked) return baked;
  return readPackageJsonVersion();
}

export function packageVersion(): string {
  // vite.config.ts defines __QUOIN_VERSION__ unconditionally — the git describe
  // string for a build, the empty string otherwise — so it is always a
  // substituted literal here and needs no typeof guard. resolveVersion owns the
  // empty case, and is tested directly for it.
  return resolveVersion(__QUOIN_VERSION__);
}
