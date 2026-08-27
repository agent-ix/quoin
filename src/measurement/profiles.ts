import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";

import { parse as parseYaml } from "yaml";

export interface AssuranceProfileSummary {
  id: string;
  title: string;
  status: "proposed" | "active" | "retired";
  path: string;
}

/** Load active profiles from either supported repository assurance root. */
export function loadActiveAssuranceProfiles(
  repo: string,
): AssuranceProfileSummary[] {
  return assuranceFiles(repo)
    .map((path) => profileFrom(path, repo))
    .filter(
      (profile): profile is AssuranceProfileSummary =>
        profile !== null && profile.status === "active",
    )
    .sort((a, b) => compare(a.id, b.id) || compare(a.path, b.path));
}

function assuranceFiles(repo: string): string[] {
  return [join(repo, "spec", "assurance"), join(repo, "assurance")]
    .filter(existsSync)
    .flatMap(markdownFiles)
    .sort(compare);
}

function profileFrom(
  path: string,
  repo: string,
): AssuranceProfileSummary | null {
  const text = readFileSync(path, "utf8");
  const match = /^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/.exec(text);
  if (!match) return null;
  const value = parseYaml(match[1]) as Record<string, unknown>;
  if (value.type !== "AssuranceProfile") return null;
  for (const key of ["id", "title", "status"]) {
    if (typeof value[key] !== "string" || value[key].length === 0) {
      throw new Error(
        `${path}: AssuranceProfile requires non-empty \`${key}\``,
      );
    }
  }
  const status = value.status as AssuranceProfileSummary["status"];
  if (!new Set(["proposed", "active", "retired"]).has(status)) {
    throw new Error(`${path}: unknown AssuranceProfile status \`${status}\``);
  }
  return {
    id: value.id as string,
    title: value.title as string,
    status,
    path: relative(repo, path),
  };
}

function markdownFiles(root: string): string[] {
  const out: string[] = [];
  const pending = [root];
  while (pending.length > 0) {
    const dir = pending.pop() as string;
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const path = join(dir, entry.name);
      if (entry.isDirectory()) pending.push(path);
      else if (entry.isFile() && entry.name.endsWith(".md")) out.push(path);
    }
  }
  return out.sort(compare);
}

function compare(a: string, b: string): number {
  return a === b ? 0 : a < b ? -1 : 1;
}
