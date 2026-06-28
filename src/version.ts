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
  return resolveVersion(
    typeof __QUOIN_VERSION__ === "string" ? __QUOIN_VERSION__ : "",
  );
}
