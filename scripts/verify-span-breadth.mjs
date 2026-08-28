#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const labelPath = resolve(ROOT, "bench", "span-breadth-v1-labels.json");

function run(command, args, cwd, allowFailure = false) {
  const done = spawnSync(command, args, {
    cwd,
    encoding: "utf8",
    maxBuffer: 128 * 1024 * 1024,
    timeout: 120_000,
  });
  if (done.error || (!allowFailure && done.status !== 0)) {
    throw new Error(
      `${command} ${args.join(" ")} failed: ${done.stderr || done.error}`,
    );
  }
  return done.stdout.trim();
}

export function evaluateSpanBreadth(labels, payloads) {
  const misses = [];
  const outcomes = { exact: 0, safeRefusal: 0 };
  for (const label of labels.labels ?? []) {
    const criteria = payloads[label.repository] ?? [];
    const observed = criteria.find(
      (criterion) =>
        criterion.document === label.document &&
        criterion.rowId === label.rowId &&
        criterion.statement === label.statement &&
        criterion.property === label.property,
    );
    if (!observed) {
      misses.push({
        id: label.id,
        reason: "labeled criterion disappeared or moved",
      });
      continue;
    }
    if (label.expectedSpans) {
      const same = ["domain", "precondition", "oracle"].every(
        (field) =>
          JSON.stringify(observed[field]) ===
          JSON.stringify(label.expectedSpans[field]),
      );
      if (same) outcomes.exact += 1;
      else
        misses.push({
          id: label.id,
          reason: "expected exact spans moved",
          observed,
        });
    } else {
      const noSpans = ["domain", "precondition", "oracle"].every(
        (field) => observed[field] == null,
      );
      if (noSpans && observed.signals.includes(label.expectedRefusal)) {
        outcomes.safeRefusal += 1;
      } else {
        misses.push({ id: label.id, reason: "safe refusal moved", observed });
      }
    }
  }
  const normalized = new Set(
    (labels.labels ?? []).map((label) => label.normalizedStatement),
  );
  const shapes = new Set((labels.labels ?? []).map((label) => label.property));
  const repositories = new Set(
    (labels.labels ?? []).map((label) => label.repository),
  );
  const challenges = new Set(
    (labels.labels ?? []).map((label) => label.challenge),
  );
  for (const required of [
    "exact",
    "truncated",
    "overbroad",
    "wrong-subject",
    "hyphenated",
    "nested",
    "coordinated",
    "justified-refusal",
  ]) {
    if (required === "exact") continue;
    if (!challenges.has(required))
      misses.push({
        id: "population",
        reason: `missing ${required} challenge`,
      });
  }
  if (normalized.size < 60)
    misses.push({
      id: "population",
      reason: `only ${normalized.size} unique normalized statements`,
    });
  if (shapes.size < 5)
    misses.push({
      id: "population",
      reason: `only ${shapes.size} property shapes`,
    });
  if (repositories.size < 3)
    misses.push({
      id: "population",
      reason: `only ${repositories.size} repositories`,
    });
  if (outcomes.exact < 20)
    misses.push({
      id: "population",
      reason: `only ${outcomes.exact} exact-grounding cases`,
    });
  const denominator = labels.labels?.length ?? 0;
  const numerator = outcomes.exact + outcomes.safeRefusal;
  return {
    definitionVersion: "property.span-breadth-v1",
    numerator,
    denominator,
    rate: denominator === 0 ? null : numerator / denominator,
    outcomes,
    uniqueNormalizedStatements: normalized.size,
    propertyShapes: [...shapes].sort(),
    repositories: [...repositories].sort(),
    challenges: [...challenges].sort(),
    namedMisses: misses,
  };
}

async function main() {
  const quireArg = process.argv.indexOf("--quire");
  const quire = resolve(
    quireArg < 0 ? (process.env.QUIRE ?? "") : process.argv[quireArg + 1],
  );
  if (!quire.startsWith("/"))
    throw new Error("verify-span-breadth: --quire must be absolute");
  const labels = JSON.parse(readFileSync(labelPath, "utf8"));
  const payloads = {};
  for (const [name, source] of Object.entries(labels.repositories)) {
    const root =
      name === "quoin"
        ? ROOT
        : resolve(
            process.env[`${name.toUpperCase().replaceAll("-", "_")}_ROOT`] ??
              resolve(ROOT, "..", name),
          );
    const status = run(
      "git",
      ["status", "--porcelain=v1", "--untracked-files=all"],
      root,
    );
    if (status)
      throw new Error(`verify-span-breadth: ${name} checkout is dirty`);
    const head = run("git", ["rev-parse", "HEAD"], root);
    if (name !== "quoin" && head !== source.revision) {
      throw new Error(
        `verify-span-breadth: ${name} revision ${head} != ${source.revision}`,
      );
    }
    const raw = run(
      quire,
      ["properties", "--scope", root, "--json", "spec/**/*.md"],
      root,
      name === "quire-rs",
    );
    const report = JSON.parse(raw);
    payloads[name] = report.documents.flatMap((document) =>
      (document.criteria ?? []).map((criterion) => ({
        ...criterion,
        rowId: criterion.row_id,
        document: relative(root, document.document).replaceAll("\\", "/"),
      })),
    );
  }
  const result = evaluateSpanBreadth(labels, payloads);
  if (process.argv.includes("--json"))
    console.log(JSON.stringify(result, null, 2));
  else
    console.log(
      `span-breadth: ${result.numerator}/${result.denominator} exact or safely refused; ${result.uniqueNormalizedStatements} unique; ${result.propertyShapes.length} shapes; ${result.repositories.length} repositories`,
    );
  if (result.rate !== 1 || result.namedMisses.length > 0) {
    for (const miss of result.namedMisses)
      console.error(`span-breadth: ${miss.id}: ${miss.reason}`);
    process.exit(1);
  }
}

if (
  resolve(process.argv[1] ?? "") === resolve(fileURLToPath(import.meta.url))
) {
  await main();
}
