import { existsSync, readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";

import { parse as parseYaml } from "yaml";

/** Validate the public qa-corpus inventory envelope, not its source metadata. */
export function validateCanonicalInventory(payload) {
  if (
    !Array.isArray(payload?.cases) ||
    typeof payload?.bounds?.gap_count !== "number"
  ) {
    throw new Error(
      "bench-tier1: qa-corpus inventory must carry cases and numeric bounds",
    );
  }
  const required = ["id", "dir", "expect", "module", "language"];
  for (const [index, entry] of payload.cases.entries()) {
    const missing = required.filter(
      (key) => typeof entry?.[key] !== "string" || entry[key].length === 0,
    );
    if (missing.length) {
      throw new Error(
        `bench-tier1: qa-corpus inventory case ${index} lacks ${missing.join(", ")}`,
      );
    }
  }
  return payload;
}

/** Materialize scorer inputs from the canonical inventory and expectation files. */
export function loadCorpusData({ mapping, root, modulesRoot, inventory }) {
  modulesRoot ??= join(root, "modules");
  inventory = validateCanonicalInventory(inventory);
  const corpora = [];
  for (const meta of inventory.cases) {
    const expectPath = join(root, meta.expect);
    const inputPath = join(root, meta.dir, "input");
    if (!existsSync(expectPath) || !existsSync(inputPath)) {
      throw new Error(
        `bench-tier1: canonical case ${meta.id} resolves to missing ` +
          `${!existsSync(inputPath) ? "input" : "expectation"} under ${root}`,
      );
    }
    const expect = parseYaml(readFileSync(expectPath, "utf8")) ?? {};
    assertReasonsMapped(meta, expect, mapping, meta.dir);
    const exactLabelPath = join(root, "labels", `${meta.id}.yaml`);
    const inheritedLabelPath = meta.case
      ? join(root, "labels", `${meta.case}.yaml`)
      : null;
    const labelPath = existsSync(exactLabelPath)
      ? exactLabelPath
      : inheritedLabelPath && existsSync(inheritedLabelPath)
        ? inheritedLabelPath
        : null;
    const inheritedLabel = labelPath !== null && labelPath !== exactLabelPath;
    const label = labelPath ? parseYaml(readFileSync(labelPath, "utf8")) : null;
    const defects = (label?.defects ?? defectsFrom(meta, expect, mapping)).map(
      (defect) => ({
        ...defect,
        id: inheritedLabel ? `${defect.id}-${meta.language}` : defect.id,
        actionable_fragments: actionableFragments(defect, expect),
      }),
    );
    const pendingPath = join(dirname(expectPath), "expect-pending.yaml");
    const pendingExpect = existsSync(pendingPath)
      ? (parseYaml(readFileSync(pendingPath, "utf8")) ?? {})
      : {};
    corpora.push({
      name: meta.id,
      mode: meta.mode,
      kind: meta.kind,
      findable: meta.findable,
      family: label?.family ?? familyOf(meta, expect, mapping),
      summary: label?.summary ?? meta.comment ?? "",
      defects,
      observations: {
        untracked_symbols: expect.untracked_symbols ?? [],
      },
      input: inputPath,
      module: resolveModule(join(modulesRoot, meta.module), meta, modulesRoot),
      language: meta.language,
      pending: meta.pending ?? null,
      rules: {
        present: expect.diagnostic_reasons ?? [],
        absent: expect.absent_diagnostic_reasons ?? [],
      },
      pendingReasons: meta.pending
        ? (pendingExpect.diagnostic_reasons ?? [])
        : [],
      hasPendingBlock: existsSync(pendingPath),
    });
  }
  return { corpora, modulesRoot, bounds: inventory.bounds };
}

function actionableFragments(defect, expect) {
  const reason = defect.expect_reason ?? defect.expect_suspicion ?? null;
  if (reason) {
    const direct =
      expect.diagnostic_message_contains?.[reason] ??
      expect.diagnostic_message_contains?.[reason.split("/").at(-1)];
    if (Array.isArray(direct)) return direct;
    const suspicion = (expect.suspicions ?? []).find(
      (item) => item.kind === reason,
    );
    if (suspicion?.message_contains?.length) return suspicion.message_contains;
  }
  return defect.command === "validate" ? (expect.validate_contains ?? []) : [];
}

/** Corpus-wide advisory rulings, authored once in corpus.yaml. */
export function standingAdjudications(root) {
  const path = join(root, "corpus.yaml");
  if (!existsSync(path)) return [];
  return (
    (parseYaml(readFileSync(path, "utf8")) ?? {}).standing_adjudications ?? []
  );
}

function resolveModule(path, meta, modulesRoot) {
  const usable =
    existsSync(join(path, "manifest.yaml")) ||
    (existsSync(path) &&
      readdirSync(path).some((child) =>
        existsSync(join(path, child, "manifest.yaml")),
      ));
  if (usable) return path;
  throw new Error(
    `bench-tier1: case \`${meta.id}\` binds module \`${meta.module}\`, which ` +
      `resolves under ${modulesRoot} and holds no \`manifest.yaml\``,
  );
}

function familyForReason(mapping, reason) {
  const key = reason.includes("/")
    ? reason.slice(reason.indexOf("/") + 1)
    : reason;
  return (
    Object.entries(mapping?.families ?? {}).find(
      ([, value]) => value.key === key,
    )?.[0] ?? null
  );
}

function assertReasonsMapped(meta, expect, mapping, where) {
  const unmapped = (expect.diagnostic_reasons ?? []).filter(
    (reason) => !familyForReason(mapping, reason),
  );
  if (!unmapped.length) return;
  throw new Error(
    `bench-tier1: ${where} (${meta.id}) expects diagnostic reason` +
      `${unmapped.length === 1 ? "" : "s"} ` +
      unmapped.map((reason) => `\`${reason}\``).join(", ") +
      " that no family in bench/tier1-mapping.json claims. " +
      "Declare an owning family (use `source: none` for a declared hole).",
  );
}

function defectsFrom(meta, expect, mapping) {
  if (meta.kind !== "failure") return [];
  for (const reason of expect.diagnostic_reasons ?? []) {
    return [
      {
        id: `${initials(meta.id)}-${ordinal(meta.id)}`,
        family: familyForReason(mapping, reason),
        location: expect.diagnostic_paths?.[reason] ?? null,
        findable: meta.findable !== false,
        expect_reason: reason,
        confirmed_at: "derived from the case's own expect.yaml",
        collateral: (expect.diagnostic_reasons ?? [])
          .filter((candidate) => candidate !== reason)
          .map((candidate) => {
            const family = familyForReason(mapping, candidate);
            return family
              ? {
                  family,
                  reason: candidate,
                  note:
                    "derived: the case expectation names both reasons as " +
                    "consequences of one seeded defect",
                }
              : null;
          })
          .filter(Boolean),
        note: "Derived from qa-corpus expect.yaml rather than separately adjudicated.",
      },
    ];
  }
  return [];
}

function familyOf(meta, expect, mapping) {
  if (meta.kind !== "failure") return "none";
  for (const reason of expect.diagnostic_reasons ?? []) {
    const family = familyForReason(mapping, reason);
    if (family) return family;
  }
  return "none";
}

function ordinal(id) {
  let hash = 0;
  for (const character of id) {
    hash = (hash * 31 + character.charCodeAt(0)) % 997;
  }
  return hash;
}

function initials(id) {
  const parts = id.split(/[^a-z0-9]+/i).filter(Boolean);
  return (
    (parts[0]?.[0] ?? "X") + (parts[1]?.[0] ?? parts[0]?.[1] ?? "X")
  ).toUpperCase();
}
