/** dependency-cruiser JSON to finding-shaped scan evidence (FR-045). */

import { createHash } from "node:crypto";

import { canonicalJson } from "../store.js";
import type { FindingResult } from "./sarif.js";
import { AdapterError } from "./types.js";

export function parseDependencyCruiser(raw: string): FindingResult {
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw) as unknown;
  } catch (cause) {
    throw new AdapterError(
      "dependency-cruiser",
      `input is not JSON: ${(cause as Error).message}`,
    );
  }
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new AdapterError("dependency-cruiser", "input is not an object");
  }
  const root = parsed as Record<string, unknown>;
  const summary = root.summary;
  if (
    summary === null ||
    typeof summary !== "object" ||
    Array.isArray(summary)
  ) {
    throw new AdapterError("dependency-cruiser", "summary is missing");
  }
  const value = summary as Record<string, unknown>;
  if (typeof value.totalCruised !== "number" || value.totalCruised <= 0) {
    throw new AdapterError(
      "dependency-cruiser",
      "zero modules were traversed; missing language support or empty scope cannot be recorded as a clean scan",
    );
  }
  if (!Array.isArray(value.violations)) {
    throw new AdapterError(
      "dependency-cruiser",
      "summary.violations is not an array",
    );
  }
  const ruleSet =
    value.ruleSetUsed !== null && typeof value.ruleSetUsed === "object"
      ? (value.ruleSetUsed as Record<string, unknown>)
      : {};
  const ruleCount = ["forbidden", "allowed", "required"].reduce(
    (count, key) =>
      count + (Array.isArray(ruleSet[key]) ? ruleSet[key].length : 0),
    0,
  );
  if (ruleCount === 0) {
    throw new AdapterError(
      "dependency-cruiser",
      "no dependency rules were evaluated; the scan is vacuous",
    );
  }
  const environment =
    value.environment !== null && typeof value.environment === "object"
      ? (value.environment as Record<string, unknown>)
      : {};
  const version =
    typeof environment.version === "string" ? environment.version : undefined;
  const rulesetDigest = createHash("sha256")
    .update(canonicalJson(ruleSet))
    .digest("hex");

  return {
    ...(version ? { tool: `dependency-cruiser ${version}` } : {}),
    ruleset: `sha256:${rulesetDigest}`,
    rulesEvaluated: ruleCount,
    findings: value.violations.map((rawViolation, index) => {
      if (
        rawViolation === null ||
        typeof rawViolation !== "object" ||
        Array.isArray(rawViolation)
      ) {
        throw new AdapterError(
          "dependency-cruiser",
          `violation ${index + 1} is not an object`,
        );
      }
      const violation = rawViolation as Record<string, unknown>;
      const rule =
        violation.rule !== null &&
        typeof violation.rule === "object" &&
        !Array.isArray(violation.rule)
          ? (violation.rule as Record<string, unknown>)
          : {};
      if (
        typeof rule.name !== "string" ||
        rule.name === "" ||
        typeof violation.from !== "string" ||
        typeof violation.to !== "string"
      ) {
        throw new AdapterError(
          "dependency-cruiser",
          `violation ${index + 1} lacks rule/from/to identity`,
        );
      }
      return {
        ruleId: rule.name,
        ...(typeof rule.severity === "string"
          ? { severity: rule.severity }
          : {}),
        message: `${String(violation.type ?? "dependency")}: ${violation.from} -> ${violation.to}`,
        path: violation.from,
      };
    }),
  };
}
