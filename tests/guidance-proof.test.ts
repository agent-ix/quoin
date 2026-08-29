import {
  evaluateGuidanceProof,
  guidancePopulation,
} from "../scripts/lib/guidance-proof.mjs";

function finding(index: number, kind: "remedy" | "diagnostic" = "diagnostic") {
  return {
    schemaVersion: "finding-envelope-v2",
    source: {
      class: "quire",
      producer: "quire",
      channel: "coverage.diagnostics",
    },
    kind: "example",
    identity: { family: "example", case: `case-${index}`, language: "rust" },
    subject: { state: "available", value: `subject ${index}` },
    locus: { state: "available", value: { path: "spec.md", line: index + 1 } },
    causalEvidence: { state: "available", value: `cause ${index}` },
    changeTarget: { state: "available", value: `spec.md:${index + 1}` },
    nextMove: {
      state: "available",
      value: { kind, text: `${kind} ${index % 2}` },
    },
    raw: { index },
  };
}

describe("independent guidance proof", () => {
  test("freezes a deterministic five plus every action template", () => {
    const population = guidancePopulation(
      Array.from({ length: 8 }, (_, i) => finding(i)),
    );
    expect(population).toHaveLength(1);
    expect(population[0].denominator).toBe(8);
    expect(population[0].selectedRecordIds.length).toBeGreaterThanOrEqual(5);
    expect(population[0].templateDigests).toHaveLength(2);
  });

  test("refuses denominator drift and requires executable outcomes", () => {
    const records = [finding(0, "remedy"), finding(1, "diagnostic")];
    const [partition] = guidancePopulation(records);
    const contract = {
      schemaVersion: "guidance-evaluator-contract-v1",
      partitions: [{ ...partition, records: undefined }],
    };
    const evidence = {
      schemaVersion: "guidance-independent-review-v1",
      templates: partition.records.map((item) => ({
        templateDigest: item.templateDigest,
        outcome: "pass",
      })),
      records: partition.records.map((item) => ({
        recordId: item.recordId,
        partition: partition.id,
        correctness: "pass",
        executableProof: {
          kind: item.template.kind === "remedy" ? "repair" : "diagnostic",
          outcome: "pass",
        },
      })),
    };
    const result = evaluateGuidanceProof(records, contract, evidence);
    expect(result.correctness.rate).toBe(1);
    expect(result.repairSuccess.rate).toBe(1);
    expect(result.diagnosticYield.rate).toBe(1);
    expect(() =>
      evaluateGuidanceProof([...records, finding(2)], contract, evidence),
    ).toThrow(/denominator drifted/);
  });
});
