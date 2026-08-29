import { createHash } from "node:crypto";

function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${canonical(value[key])}`)
      .join(",")}}`;
  }
  return JSON.stringify(value);
}

function digest(value) {
  return `sha256:${createHash("sha256").update(canonical(value)).digest("hex")}`;
}

function partitionOf(record) {
  return [
    record.source.class,
    record.source.producer,
    record.source.channel,
    record.identity?.family ?? record.kind,
  ].join("/");
}

function abstractIdentity(record) {
  return {
    partition: partitionOf(record),
    kind: record.kind,
    identity: record.identity ?? {},
  };
}

function templateOf(record) {
  return {
    kind: record.nextMove?.value?.kind ?? "unavailable",
    text: record.nextMove?.value?.text ?? "",
  };
}

export function guidancePopulation(records) {
  const groups = new Map();
  for (const record of records) {
    const partition = partitionOf(record);
    if (!groups.has(partition)) groups.set(partition, []);
    groups.get(partition).push(record);
  }
  return [...groups.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([partition, partitionRecords]) => {
      const occurrences = new Map();
      const recordsWithIds = partitionRecords
        .map((record) => ({ record, identity: abstractIdentity(record) }))
        .sort((a, b) =>
          canonical(a.identity).localeCompare(canonical(b.identity)),
        )
        .map(({ record, identity }) => {
          const key = canonical(identity);
          const occurrence = occurrences.get(key) ?? 0;
          occurrences.set(key, occurrence + 1);
          const recordId = digest({ identity, occurrence });
          return {
            recordId,
            templateDigest: digest(templateOf(record)),
            template: templateOf(record),
            record,
          };
        });
      const byHash = [...recordsWithIds].sort((a, b) =>
        digest(a.recordId).localeCompare(digest(b.recordId)),
      );
      const selected =
        recordsWithIds.length <= 5 ? [...recordsWithIds] : byHash.slice(0, 5);
      const selectedIds = new Set(selected.map((item) => item.recordId));
      for (const templateDigest of [
        ...new Set(recordsWithIds.map((item) => item.templateDigest)),
      ]) {
        const witness = recordsWithIds.find(
          (item) => item.templateDigest === templateDigest,
        );
        if (witness && !selectedIds.has(witness.recordId)) {
          selected.push(witness);
          selectedIds.add(witness.recordId);
        }
      }
      selected.sort((a, b) => a.recordId.localeCompare(b.recordId));
      return {
        id: partition,
        denominator: recordsWithIds.length,
        populationDigest: digest(
          recordsWithIds.map((item) => item.recordId).sort(),
        ),
        templateDigests: [
          ...new Set(recordsWithIds.map((item) => item.templateDigest)),
        ].sort(),
        selectedRecordIds: selected.map((item) => item.recordId),
        records: recordsWithIds,
      };
    });
}

export function evaluateGuidanceProof(records, contract, evidence) {
  if (contract?.schemaVersion !== "guidance-evaluator-contract-v1") {
    throw new Error(
      "guidance evaluator contract has unsupported schemaVersion",
    );
  }
  if (evidence?.schemaVersion !== "guidance-independent-review-v1") {
    throw new Error("guidance review evidence has unsupported schemaVersion");
  }
  const population = guidancePopulation(records);
  const declared = new Map(
    (contract.partitions ?? []).map((row) => [row.id, row]),
  );
  const reviews = new Map(
    (evidence.records ?? []).map((row) => [row.recordId, row]),
  );
  const templateReviews = new Map(
    (evidence.templates ?? []).map((row) => [row.templateDigest, row]),
  );
  const selected = [];
  for (const partition of population) {
    const frozen = declared.get(partition.id);
    if (!frozen)
      throw new Error(`guidance contract omits partition ${partition.id}`);
    for (const key of ["denominator", "populationDigest"]) {
      if (frozen[key] !== partition[key]) {
        throw new Error(
          `guidance partition ${partition.id} ${key} drifted: ` +
            `expected ${frozen[key]}, observed ${partition[key]}`,
        );
      }
    }
    if (
      canonical(frozen.templateDigests) !== canonical(partition.templateDigests)
    ) {
      throw new Error(
        `guidance partition ${partition.id} action templates drifted`,
      );
    }
    if (
      canonical(frozen.selectedRecordIds) !==
      canonical(partition.selectedRecordIds)
    ) {
      throw new Error(
        `guidance partition ${partition.id} deterministic sample drifted`,
      );
    }
    for (const item of partition.records.filter((row) =>
      partition.selectedRecordIds.includes(row.recordId),
    )) {
      const review = reviews.get(item.recordId);
      const templateReview = templateReviews.get(item.templateDigest);
      if (!review || review.partition !== partition.id) {
        throw new Error(
          `guidance review omits selected record ${item.recordId}`,
        );
      }
      if (!templateReview || templateReview.outcome !== "pass") {
        throw new Error(
          `guidance review omits passing template ${item.templateDigest}`,
        );
      }
      selected.push({ ...item, review, templateReview });
    }
  }
  if (declared.size !== population.length) {
    throw new Error("guidance contract contains a disappeared partition");
  }
  const correctness = selected.filter(
    (item) =>
      item.review.correctness === "pass" &&
      item.templateReview.outcome === "pass",
  ).length;
  const remedies = selected.filter((item) => item.template.kind === "remedy");
  const diagnostics = selected.filter(
    (item) => item.template.kind === "diagnostic",
  );
  const repairSuccess = remedies.filter(
    (item) =>
      item.review.executableProof?.kind === "repair" &&
      item.review.executableProof?.outcome === "pass",
  ).length;
  const diagnosticYield = diagnostics.filter(
    (item) =>
      item.review.executableProof?.kind === "diagnostic" &&
      item.review.executableProof?.outcome === "pass",
  ).length;
  const metric = (definitionVersion, numerator, denominator, misses) => ({
    definitionVersion,
    numerator,
    denominator,
    rate: denominator === 0 ? null : numerator / denominator,
    namedMisses: misses,
  });
  return {
    contractDigest: digest(contract),
    evidenceDigest: digest(evidence),
    selectedRecords: selected.length,
    correctness: metric(
      "guidance.correctness-v1",
      correctness,
      selected.length,
      selected
        .filter((item) => item.review.correctness !== "pass")
        .map((item) => item.recordId),
    ),
    repairSuccess: metric(
      "guidance.repair-success-v1",
      repairSuccess,
      remedies.length,
      remedies
        .filter((item) => item.review.executableProof?.outcome !== "pass")
        .map((item) => item.recordId),
    ),
    diagnosticYield: metric(
      "guidance.diagnostic-yield-v1",
      diagnosticYield,
      diagnostics.length,
      diagnostics
        .filter((item) => item.review.executableProof?.outcome !== "pass")
        .map((item) => item.recordId),
    ),
  };
}

export { digest as guidanceDigest };
