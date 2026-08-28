import { evaluateSpanBreadth } from "../scripts/verify-span-breadth.mjs";

function label(index: number, repository: string, property: string) {
  const statement = `Subject ${index} when condition ${index} returns result ${index}`;
  return {
    id: `label-${index}`,
    repository,
    document: "spec.md",
    rowId: `AC-${index}`,
    property,
    statement,
    normalizedStatement: statement.toLowerCase(),
    challenge: [
      "truncated",
      "overbroad",
      "wrong-subject",
      "hyphenated",
      "nested",
      "coordinated",
    ][index % 6],
    expectedSpans: {
      domain: { start: 0, end: 7, text: "Subject" },
      precondition: { start: 10, end: 24, text: "when condition" },
      oracle: { start: 27, end: 41, text: "returns result" },
    },
  };
}

describe("broad span-grounding evidence", () => {
  test("requires unique breadth and exact outcomes", () => {
    const repositories = ["a", "b", "c"];
    const properties = [
      "invariant",
      "ordering",
      "round-trip",
      "idempotence",
      "error-case",
    ];
    const labels = Array.from({ length: 60 }, (_, index) =>
      label(index, repositories[index % 3], properties[index % 5]),
    );
    labels[59] = {
      ...labels[59],
      challenge: "justified-refusal",
      expectedSpans: undefined,
      expectedRefusal: "span:refused-weak-boundary",
    };
    const payloads = Object.fromEntries(
      repositories.map((repository) => [
        repository,
        labels
          .filter((item) => item.repository === repository)
          .map((item) => ({
            document: item.document,
            rowId: item.rowId,
            statement: item.statement,
            property: item.property,
            ...(item.expectedSpans ?? {
              domain: null,
              precondition: null,
              oracle: null,
            }),
            signals: item.expectedRefusal ? [item.expectedRefusal] : [],
          })),
      ]),
    );
    const result = evaluateSpanBreadth({ labels }, payloads);
    expect(result.rate).toBe(1);
    expect(result.uniqueNormalizedStatements).toBe(60);
    expect(result.outcomes.exact).toBe(59);
    payloads.a[0].oracle = null;
    expect(evaluateSpanBreadth({ labels }, payloads).namedMisses).toEqual(
      expect.arrayContaining([expect.objectContaining({ id: "label-0" })]),
    );
  });
});
