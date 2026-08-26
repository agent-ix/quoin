/**
 * FR-029 — the quire↔quoin JSON contract (TC-110..TC-118).
 *
 * The point of these is stated in quoin's own `spec/review.md` Finding 8: "no
 * contract test against quire". The shapes lived as prose in skill markdown,
 * so a drift surfaced as an agent failing mid-skill with nothing to diagnose.
 */

import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
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
  it("pins an exact source commit rather than a moving tag or branch", () => {
    expect(QUIRE_CONTRACT.sourceRevision).toMatch(/^[0-9a-f]{40}$/);
  });

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

  // TC-116
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
      undeclared_statuses: [
        {
          reference: "traces-to",
          document: "spec/tests.md",
          row_id: "TC-006",
          status: "\u26a0\ufe0f scale evidence deferred",
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
          line: 12,
        },
      ],
      diagnostic_reason_registry: ["archetype-matches-nothing"],
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
      implements: [
        {
          path: "src/lib.rs",
          symbol: "parse",
          trace_id: "FR-001",
          form: "rust-implements-line",
        },
      ],
      vocabulary_coverage: [
        {
          vocabulary: "verification-methods",
          archetype: "FR",
          field: "verification",
          check: "warning",
          value: "Inspection",
          state: "owned",
          documents: ["spec/functional/FR-001.md"],
        },
      ],
      shared_trace_ids: [
        {
          trace_id: "TC-001",
          symbols: [
            { path: "tests/parse.rs", symbol: "tc_001_parses" },
            { path: "tests/parse_more.rs", symbol: "tc_001_parses_again" },
          ],
        },
      ],
      excluded_source_files: 3,
      binding_census: [
        {
          language: "rust",
          candidates: 2,
          tagged: 1,
          bound: 1,
          forms: ["rust-verifies-line"],
          unbound_example: {
            path: "tests/parse.rs",
            line: 30,
            symbol: "parse_without_marker",
          },
          unmatched_example: {
            path: "tests/parse.rs",
            line: 30,
            symbol: "parse_without_marker",
          },
        },
      ],
      metrics: [
        {
          name: "coverage.backed",
          unit: "matrix row",
          method: "backed rows divided by all rows",
          shape: "ratio",
          state: "measured",
          value: 1,
          population: 2,
          examined: 2,
          matched: 1,
        },
      ],
      suspicions: [
        {
          kind: "vacuous-under-guard",
          path: "tests/parse.rs",
          symbol: "all_inputs",
          line: 50,
          message: "every assertion is guarded",
          evidence: "1 of 42 samples entered the assertion",
        },
      ],
      engine: {
        cli: "0.30.2",
        engine: "a14dcb2",
        capabilities: ["binding_census"],
      },
      totals: {
        backed: 1,
        total: 2,
        criteria: 2,
        property_shaped: 1,
        specific_shaped: 1,
      },
    };

    // "Every" is read off the schema, not off this fixture's memory of it:
    // when the schema gained `undeclared_statuses` and `implements` (v0.41.0),
    // this payload kept validating while its title silently narrowed to
    // "every optional key the fixture happened to know about" (#178). Now a
    // new optional key fails here until the fixture actually carries it.
    const schema = readSchema("coverage-v1.schema.json") as {
      properties: Record<string, unknown>;
      required: string[];
    };
    const optional = Object.keys(schema.properties).filter(
      (key) => !schema.required.includes(key),
    );
    for (const key of optional) {
      expect(
        Object.keys(full),
        `the "every optional key" payload no longer carries \`${key}\``,
      ).toContain(key);
    }

    const result = validateCoverage(full);
    expect(result.ok, JSON.stringify(result)).toBe(true);
  });

  // TC-116
  it("accepts the v0.41.0 optional keys, and rejects a malformed one", () => {
    // A vendored schema can drift from the engine in the one direction nothing
    // notices: a NEW optional key. The payload still validates because the key
    // is simply unknown — until `additionalProperties: false` rejects it, which
    // is what would happen here if the refresh had not been run.
    const withDrift = coveragePayload();
    withDrift.undeclared_statuses = [
      {
        reference: "traces-to",
        document: "spec/tests.md",
        row_id: "TC-006",
        status: "\u26a0\ufe0f scale evidence deferred",
      },
    ];
    withDrift.implements = [
      {
        path: "src/lib.rs",
        symbol: "parse",
        trace_id: "FR-001",
        form: "rust-implements-line",
      },
    ];
    expect(validateCoverage(withDrift).ok).toBe(true);

    // `status` is required, and the class is deliberately NOT carried — having
    // none is the finding.
    const malformed = coveragePayload();
    malformed.undeclared_statuses = [
      { reference: "traces-to", document: "spec/tests.md" },
    ];
    expect(validateCoverage(malformed).ok).toBe(false);
  });

  // TC-116
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
  it("a real `quire coverage --json` payload validates against the pinned schema", (ctx) => {
    if (installed === null) return ctx.skip();

    // **This is the check whose absence broke every coverage-reading command.**
    // quire-rs CR-080 added `implements` to the report; the vendored schema is
    // `additionalProperties: false` and was pinned two releases back, so the
    // real binary emitted a field the contract forbade and `quoin advise`,
    // `evidence audit` and `completeness` all failed at once. Nothing caught it
    // because the only coverage assertions in this file validate a HAND-BUILT
    // fixture, which by construction carries whatever the schema already allows.
    //
    // The bundle deliberately produces an `implements` edge. A payload without
    // one would validate against the old schema too, so the guard would pass
    // while proving nothing — the tautology this test exists to avoid.
    const root = mkdtempSync(join(tmpdir(), "quoin-tc118-cov-"));
    try {
      mkdirSync(join(root, "spec", "functional"), { recursive: true });
      writeFileSync(
        join(root, "spec", "functional", "FR-001-thing.md"),
        '---\nid: FR-001\ntype: FR\ntitle: "Thing"\n---\n\n' +
          "# FR-001: Thing\n\n## Description\n\nIt does the thing.\n\n" +
          "## Acceptance Criteria\n\n" +
          "| ID | Criteria | Verification |\n|----|----------|--------------|\n" +
          "| FR-001-AC-1 | It does the thing. | Test |\n",
      );
      mkdirSync(join(root, "src"), { recursive: true });
      writeFileSync(
        join(root, "src", "lib.rs"),
        "/// Implements: FR-001\npub fn do_the_thing() -> usize { 1 }\n",
      );

      const out = execFileSync(
        "quire",
        ["coverage", "--scope", root, "--json"],
        { encoding: "utf8", stdio: ["pipe", "pipe", "ignore"] },
      );

      const result = parseCoverage(out);
      expect(result.ok, out.slice(0, 400)).toBe(true);

      // The field that broke it is present, so a future contract narrowing
      // fails here rather than in a consumer command.
      const payload = JSON.parse(out) as {
        implements?: unknown[];
        diagnostics?: { declaration?: string; reason?: string }[];
      };

      // #174: the reconciled module set must load a traceability model that
      // carries `trace_targets`. Under a stale engine, a manifest key the
      // engine does not know (process v0.23.0's `source_exclude`) silently
      // dropped the whole payload, leaving the model declared-but-empty —
      // the engine says so with `model-mints-nothing`. Asserted by name so a
      // recurrence fails pointing at the model, not at a missing edge.
      const mintsNothing = (payload.diagnostics ?? []).filter(
        (d) => d.reason === "model-mints-nothing",
      );
      expect(
        mintsNothing,
        "the reconciled module set loaded no `trace_targets` — the engine " +
          "dropped or never read the traceability model (#174): " +
          out.slice(0, 600),
      ).toEqual([]);

      expect(
        (payload.implements ?? []).length,
        "the fixture must mint an `implements` edge, or this guard is vacuous: " +
          out.slice(0, 600),
      ).toBeGreaterThan(0);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
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
