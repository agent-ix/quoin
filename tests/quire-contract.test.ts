/**
 * FR-029 — the quire↔quoin JSON contract (TC-110..TC-117).
 *
 * The point of these is stated in quoin's own `spec/review.md` Finding 8: "no
 * contract test against quire". The shapes lived as prose in skill markdown,
 * so a drift surfaced as an agent failing mid-skill with nothing to diagnose.
 */

import { execFileSync } from "node:child_process";
import { describe, expect, it } from "vitest";

import {
  QUIRE_CONTRACT,
  checkVersionPremise,
  compareVersions,
  parseCliVersion,
  parseCoverage,
  parseProperties,
  readSchema,
  schemaHash,
  validateCoverage,
  validateProperties,
  type SchemaName,
} from "../src/quire/index.js";

/** A minimal payload that satisfies the coverage contract. */
function coveragePayload(): Record<string, unknown> {
  return {
    unbacked_rows: [],
    status_lies: [],
    untracked_symbols: [],
    groups: [
      { document: "spec/tests.md", target: "test-case", backed: 1, total: 2 },
    ],
    totals: { backed: 1, total: 2 },
  };
}

/** A minimal payload that satisfies the properties contract. */
function propertiesPayload(): Record<string, unknown> {
  return {
    documents: [
      {
        document: "spec/functional/FR-001.md",
        archetype: "FR",
        criteria: [
          {
            row_id: "FR-001-AC-1",
            statement: "Every finding defaults to warning.",
            line: 10,
            shape: "assertion",
            property: "universal",
            extractable: true,
            extraction: "extractable",
            domain: null,
            precondition: null,
            oracle: null,
            signals: ["universal:determiner"],
            obligation: null,
          },
        ],
      },
    ],
  };
}

describe("TC-110 the vendored schemas match their recorded provenance", () => {
  // TC-110
  it("hashes exactly what contract.ts records", () => {
    for (const [name, expected] of Object.entries(QUIRE_CONTRACT.hashes)) {
      expect(
        schemaHash(name as SchemaName),
        `${name} drifted from its recorded hash`,
      ).toBe(expected);
    }
  });

  it("carries a `$id` naming the pinned contract version", () => {
    for (const name of Object.keys(QUIRE_CONTRACT.hashes) as SchemaName[]) {
      const schema = readSchema(name) as { $id?: string };
      expect(schema.$id, `${name} has no $id`).toBeTruthy();
      expect(schema.$id).toContain(QUIRE_CONTRACT.contractVersion);
      expect(schema.$id).toContain(name);
    }
  });
});

describe("TC-111 a conformant payload validates", () => {
  // TC-111
  it("accepts a coverage payload", () => {
    const result = validateCoverage(coveragePayload());
    expect(result.ok, JSON.stringify(result)).toBe(true);
  });

  it("accepts a properties payload", () => {
    const result = validateProperties(propertiesPayload());
    expect(result.ok, JSON.stringify(result)).toBe(true);
  });
});

describe("TC-112 a drifted payload is rejected with the offending path", () => {
  // TC-112
  it("names a missing required key rather than failing later", () => {
    const payload = coveragePayload();
    delete payload.totals;
    const result = validateCoverage(payload);
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.errors.join(" ")).toContain("totals");
    // The message must place the blame correctly: this is a shape drift
    // between quire and the pinned contract, not a defect in the document.
    expect(result.error.message).toContain("shape drift");
  });

  it("rejects an added field, so the contract stays closed", () => {
    const payload = { ...coveragePayload(), surprise: true };
    expect(validateCoverage(payload).ok).toBe(false);
  });

  it("rejects a value outside a closed engine enum", () => {
    const payload = propertiesPayload();
    // `property` is closed by quire-rs FR-052-CON-3, so a value outside it is a
    // defect a consumer should hear about rather than absorb.
    (payload.documents as Record<string, unknown>[])[0].criteria = [
      {
        ...((payload.documents as never[])[0] as never)["criteria" as never][0],
        property: "vibes",
      },
    ];
    expect(validateProperties(payload).ok).toBe(false);
  });
});

describe("TC-113 unreadable output is a named diagnostic, not a throw", () => {
  // TC-113
  it("reports a JSON parse failure as a contract violation", () => {
    const result = parseCoverage("not json at all");
    expect(result.ok).toBe(false);
    if (result.ok) return;
    expect(result.error.kind).toBe("contract-violation");
    // The likeliest real cause, named, so the reader has somewhere to go.
    expect(result.error.message).toContain("stdout rather than stderr");
  });

  it("parses and validates in one step so no caller holds an unvalidated payload", () => {
    const result = parseProperties(JSON.stringify(propertiesPayload()));
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.value.documents[0].criteria[0].row_id).toBe("FR-001-AC-1");
  });
});

describe("TC-114 the version premise is enforced with a named diagnostic", () => {
  // TC-114
  it("passes a satisfying version", () => {
    expect(
      checkVersionPremise(`quire ${QUIRE_CONTRACT.minimumCli}`),
    ).toBeNull();
    expect(checkVersionPremise("quire 99.0.0")).toBeNull();
  });

  it("names the found and required versions when too old", () => {
    const failure = checkVersionPremise("quire 0.16.0");
    expect(failure).not.toBeNull();
    expect(failure?.found).toBe("0.16.0");
    expect(failure?.required).toBe(QUIRE_CONTRACT.minimumCli);
    // The consequence, not just the fact: an old binary is misread, not
    // rejected, which is why the premise is checked before anything parses.
    expect(failure?.message).toContain("misread");
  });

  it("treats an absent version as a premise failure, not a pass", () => {
    const failure = checkVersionPremise(null);
    expect(failure?.found).toBeNull();
    expect(failure?.message).toContain("could not determine");
  });
});

describe("TC-115 version parsing and comparison", () => {
  // TC-115
  it("reads the version out of the CLI banner", () => {
    expect(parseCliVersion("quire 0.21.0")).toBe("0.21.0");
    expect(parseCliVersion("nothing here")).toBeNull();
  });

  it("compares numerically, not lexically", () => {
    // The bug a string comparison produces: "0.9.0" > "0.21.0" lexically.
    expect(compareVersions("0.21.0", "0.9.0")).toBeGreaterThan(0);
    expect(compareVersions("0.21.0", "0.21.0")).toBe(0);
    expect(compareVersions("1.0.0", "0.99.99")).toBeGreaterThan(0);
  });
});

describe("TC-116 optional keys are optional and absence is not emptiness", () => {
  // TC-116
  it("accepts a payload omitting every optional key", () => {
    expect(validateCoverage(coveragePayload()).ok).toBe(true);
  });

  it("accepts a payload carrying every optional key", () => {
    const full = {
      ...coveragePayload(),
      no_symbol_rows: [
        {
          reference: "traces-to",
          document: "spec/tests.md",
          row_id: "TC-002",
          test_type: "Eval",
          target_ids: ["TC-002"],
        },
      ],
      criteria: [
        {
          document: "spec/functional/FR-001.md",
          archetype: "FR",
          criteria: 2,
          property_shaped: 1,
          by_property: { universal: 1, example: 1 },
        },
      ],
      diagnostics: [
        {
          declaration: "test-case",
          reason: "archetype-matches-nothing",
          message: "no document of archetype TestMatrix",
          path: null,
        },
      ],
      obligations: [
        {
          source: "acceptance-criterion",
          id: "FR-001-AC-1",
          document: "spec/functional/FR-001.md",
          statement: "The system shall do it.",
          statement_hash: "a".repeat(64),
          method: "Test",
          parameters: { threshold: "< 8ms" },
          criticality: "P1",
        },
      ],
      totals: { backed: 1, total: 2, criteria: 2, property_shaped: 1 },
    };
    const result = validateCoverage(full);
    expect(result.ok, JSON.stringify(result)).toBe(true);
  });

  it("rejects a malformed statement hash", () => {
    const payload = {
      ...coveragePayload(),
      obligations: [
        {
          source: "acceptance-criterion",
          id: "FR-001-AC-1",
          document: "spec/functional/FR-001.md",
          statement: "The system shall do it.",
          statement_hash: "not-a-sha",
        },
      ],
    };
    expect(validateCoverage(payload).ok).toBe(false);
  });
});

describe("TC-117 the eval harness floor tracks the contract", () => {
  // TC-117
  it("mirrors QUIRE_CONTRACT.minimumCli", async () => {
    // The harness restates the floor because it runs against sources rather
    // than `dist/`, and a build step between "run the evals" and "know which
    // quire you need" is a step that gets skipped. This is what stops the two
    // copies drifting.
    const harness = await import("../evals/lib/resolve.mjs");
    expect(harness.HARNESS_MIN_QUIRE).toBe(QUIRE_CONTRACT.minimumCli);
  });
});

describe("TC-118 the contract holds against the installed quire", () => {
  const installed = (() => {
    try {
      return execFileSync("quire", ["--version"], { encoding: "utf8" });
    } catch {
      return null;
    }
  })();

  // TC-118
  it("the installed CLI satisfies the pinned premise", (ctx) => {
    // `ctx.skip()` rather than `it.skipIf(cond)("title", …)`: the curried form
    // puts the title on a line the symbol extractor never reads, so the row
    // bound to nothing and the matrix read ✅ over it (agent-ix/quoin#124).
    if (installed === null) return ctx.skip();
    expect(checkVersionPremise(installed)).toBeNull();
  });

  // TC-118
  it("a real `quire properties --json` payload validates against the pinned schema", (ctx) => {
    if (installed === null) return ctx.skip();
    // Deliberately end-to-end: the schema is vendored, so the one thing a
    // unit test cannot show is that the real binary still emits this shape.
    const out = execFileSync(
      "quire",
      ["properties", "--json", "--archetype", "FR", "-"],
      {
        encoding: "utf8",
        input:
          "---\nid: FR-001\ntype: FR\ntitle: t\n---\n\n" +
          "## Acceptance Criteria\n\n" +
          "| ID | Criteria | Verification |\n|----|----------|--------------|\n" +
          "| FR-001-AC-1 | Every finding defaults to warning. | Test |\n",
        stdio: ["pipe", "pipe", "ignore"],
      },
    );
    const result = parseProperties(out);
    expect(result.ok, JSON.stringify(result)).toBe(true);
  });
});
