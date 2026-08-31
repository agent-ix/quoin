import { existsSync, readFileSync, readdirSync } from "node:fs";
import { isAbsolute, join, relative } from "node:path";

import { parse as parseYaml } from "yaml";

import type { MeasurementPlan } from "./types.js";

const STAGES = new Set<MeasurementPlan["stage"]>([
  "observe",
  "baseline",
  "branch-comparison",
  "trend",
  "ratchet",
  "target",
  "gate",
]);

/** Load MeasurementPlans from either supported repository assurance root. */
export function loadMeasurementPlans(repo: string): MeasurementPlan[] {
  return assuranceFiles(repo)
    .map((path) => planFrom(path, repo))
    .filter((plan): plan is MeasurementPlan => plan !== null)
    .sort((a, b) => compare(a.metric, b.metric) || compare(a.id, b.id));
}

function assuranceFiles(repo: string): string[] {
  return [join(repo, "spec", "assurance"), join(repo, "assurance")]
    .filter(existsSync)
    .flatMap(markdownFiles)
    .sort(compare);
}

function planFrom(path: string, repo: string): MeasurementPlan | null {
  const text = readFileSync(path, "utf8");
  const match = /^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/.exec(text);
  if (!match) return null;
  const value = parseYaml(match[1]) as Record<string, unknown>;
  if (value.type !== "MeasurementPlan") return null;
  const required = [
    "id",
    "title",
    "status",
    "stage",
    "metric",
    "definition_version",
  ];
  for (const key of required) {
    if (typeof value[key] !== "string" || value[key].length === 0) {
      throw new Error(`${path}: MeasurementPlan requires non-empty \`${key}\``);
    }
  }
  if (!STAGES.has(value.stage as MeasurementPlan["stage"])) {
    throw new Error(
      `${path}: unknown measurement stage \`${String(value.stage)}\``,
    );
  }
  const status = value.status as MeasurementPlan["status"];
  if (!new Set(["proposed", "active", "retired"]).has(status)) {
    throw new Error(
      `${path}: unknown MeasurementPlan status \`${String(value.status)}\``,
    );
  }
  return {
    id: value.id as string,
    title: value.title as string,
    status,
    stage: value.stage as MeasurementPlan["stage"],
    metric: value.metric as string,
    definitionVersion: value.definition_version as string,
    path: isAbsolute(path) ? relative(repo, path) : path,
    ...(typeof value.owner === "string" && value.owner
      ? { owner: value.owner }
      : {}),
    ...(typeof value.action === "string" && value.action
      ? { action: value.action }
      : {}),
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
