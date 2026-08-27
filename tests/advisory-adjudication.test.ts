import { readFileSync } from "node:fs";
import { join } from "node:path";

import {
  findingEnvelopeDigest,
  normalizeQuireFinding,
} from "../evals/lib/finding-envelope.mjs";
import {
  ADVISORY_ADJUDICATION_SCHEMA,
  ADVISORY_METRIC_VERSION,
  ADVISORY_RUBRIC_VERSION,
  digestIds,
  validateAdvisoryAdjudication,
  verifyAdvisoryAdjudicationSource,
} from "../scripts/lib/advisory-adjudication.mjs";

function fixture() {
  const finding = normalizeQuireFinding(
    {
      reason: "catch-all-universal",
      path: "spec/FR-001.md",
      line: 10,
      message: "one criterion has no specific property shape",
    },
    {
      producer: "quire",
      channel: "coverage.diagnostics",
      family: "catch-all-universal",
      corpus: "case-a",
      language: "python",
      declaration: "criteria",
    },
  );
  const id = findingEnvelopeDigest(finding);
  return {
    schema_version: ADVISORY_ADJUDICATION_SCHEMA,
    metric_version: ADVISORY_METRIC_VERSION,
    rubric: {
      version: ADVISORY_RUBRIC_VERSION,
      correct: "The diagnostic truthfully describes the source population.",
      incorrect: "The diagnostic makes a false claim about the source.",
      ambiguous: "Reviewers disagree under this rubric.",
      unresolved: "The retained evidence cannot settle the claim.",
    },
    population: {
      source: {
        revision: "73b0d5d",
        report_digest: `sha256:${"a".repeat(64)}`,
      },
      count: 1,
      digest: digestIds([id]),
    },
    findings: [{ id, finding }],
    rulings: [
      {
        finding_id: id,
        disposition: "correct",
        defect_owner: "none",
        rationale: "The criterion is extractable and names no property shape.",
        reviewer: "quoin-maintainers",
        rubric_version: ADVISORY_RUBRIC_VERSION,
        metric_version: ADVISORY_METRIC_VERSION,
        disagreements: [],
        follow_up: null,
      },
    ],
  };
}

describe("retained advisory adjudication", () => {
  it("TC-1099 validates content identity and retains every row-level ruling", () => {
    const loaded = validateAdvisoryAdjudication(fixture());
    expect(loaded.population.count).toBe(1);
    expect(loaded.counts).toEqual({
      correct: 1,
      incorrect: 0,
      ambiguous: 0,
      unresolved: 0,
    });
    expect(loaded.byFamily["catch-all-universal"]).toHaveLength(1);

    const root = process.cwd();
    const retained = JSON.parse(
      readFileSync(
        join(root, "bench", "advisory-adjudication-v1.json"),
        "utf8",
      ),
    );
    const source = readFileSync(
      join(
        root,
        "spec",
        "evidence",
        "adjudications",
        "tier1-20260827-advisory-source.json",
      ),
    );
    const actual = verifyAdvisoryAdjudicationSource(retained, source);
    expect(actual.population.count).toBe(108);
    expect(actual.counts).toEqual({
      correct: 108,
      incorrect: 0,
      ambiguous: 0,
      unresolved: 0,
    });
  });

  it("TC-1100 refuses incompatible metrics and incomplete review records", () => {
    const incompatible = fixture();
    incompatible.metric_version = "finding.precision.advisory-v2";
    expect(() => validateAdvisoryAdjudication(incompatible)).toThrow(
      /metric version.*incompatible/,
    );

    const incomplete = fixture();
    incomplete.rulings[0].rationale = "";
    expect(() => validateAdvisoryAdjudication(incomplete)).toThrow(
      /requires rationale and reviewer/,
    );

    const changed = fixture();
    changed.findings[0].finding.raw.message = "changed after adjudication";
    expect(() => validateAdvisoryAdjudication(changed)).toThrow(
      /finding id.*does not match/,
    );
  });
});
