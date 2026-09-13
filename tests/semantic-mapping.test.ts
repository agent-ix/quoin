/**
 * Golden mapping fixtures for the Markdown → semantic-core contract
 * (FR-071, FR-072, FR-074; issue #293, TASK-041).
 *
 * Quoin publishes these; quire-rs#388 executes them. What quoin proves here is
 * that every expected output validates against the vendored semantic-core
 * schemas at the recorded version, that the table and fence forms share one
 * expected declaration set, and that every expected diagnostic carries a locus.
 *
 * The classifier half of this file moved to Rust at the cutover (quoin#452):
 * `rust/crates/quoin-semantic/tests/tc_452_sweep_criteria.rs` replays the same
 * `legacy.expected.json` against `classify_artifact`. What is left here is the
 * part with a JavaScript consumer — the published fixtures and the vendored
 * schemas they validate against.
 */

import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import Ajv2020 from "ajv/dist/2020.js";
import { describe, expect, it } from "vitest";

const FIXTURES = join("tests", "fixtures", "semantic-module", "mapping");
type Json = Record<string, unknown>;

// The vendored semantic-core bundle, read straight off the shipped tree. The
// TypeScript resolver that used to answer this was deleted at the cutover
// (quoin#452) — the DATA stays exactly where it was, and this test is a
// JavaScript consumer of it, so it resolves the path itself rather than
// crossing the quoin-core boundary to ask.
const SEMANTIC_CORE_DIR = join("src", "semantic", "schemas", "semantic-core");

const SEMANTIC_CORE_VERSION = (() => {
  const toolchain = JSON.parse(
    readFileSync(join(SEMANTIC_CORE_DIR, "toolchain.json"), "utf8"),
  ) as Json;
  // The bundle records the base URI it was minted under, and every `$id` in it
  // is relative to that. Reading the version off the base rather than hard-
  // coding it keeps this test honest across a bundle refresh.
  const match =
    /^https:\/\/schemas\.agent-ix\.org\/semantic-core\/([^/]+)\/$/.exec(
      String(toolchain.base),
    );
  if (!match)
    throw new Error(`unexpected semantic-core base ${String(toolchain.base)}`);
  return match[1]!;
})();

// `attribute|ref item <name> : <Type>[<mult>]` with optional brace-delimited
// constraint text. Built from a string: a regex literal with braces trips the
// Quire trace scanner and unbinds every tag in this file (SR-127 FND-210).
const FENCE_LINE = new RegExp(
  String.raw`^(attribute|ref item) \w+ : [\w.:/-]+\[[^\]]+\]( \{[^}]*\})?$`,
);

function fixture(name: string): string {
  return readFileSync(join(FIXTURES, name), "utf8");
}

function json(name: string): Json {
  return JSON.parse(fixture(name)) as Json;
}

const ajv = (() => {
  const instance = new Ajv2020({ allErrors: true, strict: true });
  for (const name of readdirSync(SEMANTIC_CORE_DIR).filter(
    (n) => n.endsWith(".json") && n !== "toolchain.json",
  )) {
    instance.addSchema(
      JSON.parse(readFileSync(join(SEMANTIC_CORE_DIR, name), "utf8")),
    );
  }
  return instance;
})();

function validates(model: string, value: unknown): boolean {
  const validate = ajv.getSchema(
    `https://schemas.agent-ix.org/semantic-core/${SEMANTIC_CORE_VERSION}/${model}.json`,
  );
  if (!validate) throw new Error(`no vendored schema for ${model}`);
  const ok = validate(value) as boolean;
  if (!ok)
    throw new Error(
      `${model} rejected ${JSON.stringify(value)}: ${JSON.stringify(validate.errors)}`,
    );
  return ok;
}

function canonical(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value as Json)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${canonical((value as Json)[key])}`)
      .join(",")}}`;
  }
  return JSON.stringify(value);
}

describe("FR-071 typed Properties table and sysml fence fixtures", () => {
  // Trace: FR-071-AC-1, FR-071-CON-2
  it("has an FR-006 typed-table fixture whose expected FieldDecl[] validates against the vendored FieldDecl.json", () => {
    const expected = json("config-version.expected.json");
    expect(expected.semanticCore).toBe(SEMANTIC_CORE_VERSION);
    const fields = expected.fields as Json[];
    expect(fields).toHaveLength(7);
    for (const field of fields) validates("FieldDecl", field);
    for (const clause of expected.clauses as Json[]) {
      validates("ClauseRef", clause);
      const span = clause.sourceSpan as Json;
      const lines = fixture(String(span.path)).split("\n");
      expect(lines[Number(span.startLine) - 1]).toBe("```ocl");
      expect((expected.clauseText as Json)[String(clause.clauseId)]).toBe(
        lines[Number(span.startLine)],
      );
    }
    const table = fixture("config-version.table.md");
    expect(table).toContain("| Field | Type | Multiplicity | Constraints |");
  });

  // Trace: FR-071-AC-2
  it("gives the sysml fence fixture the byte-identical normalized FieldDecl[] of the table fixture", () => {
    const expected = json("config-version.expected.json");
    const authored = expected.authoredForm as Record<string, string>;
    expect(authored["config-version.table.md"]).toBe("table");
    expect(authored["config-version.fence.md"]).toBe("fence");
    const fence = fixture("config-version.fence.md");
    // One expected array serves both artifacts by construction; assert the
    // fence declares exactly the table's rows in the same order.
    const fenceNames = [
      ...fence.matchAll(/^(?:attribute|ref item) (\w+) :/gm),
    ].map((m) => m[1]);
    const tableNames = (expected.fields as Json[]).map((f) => f.name);
    expect(fenceNames).toEqual(tableNames);
    expect(canonical(expected.fields)).toBe(
      canonical(JSON.parse(JSON.stringify(expected.fields))),
    );
  });

  // Trace: FR-071-AC-3
  it("ships a both-forms fixture whose expected outcome is a failure at the second form", () => {
    const both = fixture("both-forms.md");
    expect(both).toContain("| Field | Type | Multiplicity | Constraints |");
    expect(both).toContain("```sysml");
    const tableLine =
      both.split("\n").findIndex((l) => l.startsWith("| Field |")) + 1;
    const fenceLine =
      both.split("\n").findIndex((l) => l.startsWith("```sysml")) + 1;
    expect(fenceLine).toBeGreaterThan(tableLine);
    const expected = json("both-forms.expected.json");
    expect(expected.semanticCore).toBe(SEMANTIC_CORE_VERSION);
    expect(expected.firstForm).toEqual({
      form: "typed-table",
      line: tableLine,
    });
    expect(expected.expectedDiagnostic).toMatchObject({
      severity: "error",
      line: fenceLine,
      secondForm: "sysml-fence",
    });
  });

  // Trace: FR-071-AC-4, FR-071-AC-5, FR-071-AC-6, FR-071-AC-7, FR-071-AC-8
  it("records every cell, fence-line, and reader-rule case with a schema-valid expectation or a located diagnostic", () => {
    const cases = json("cell-cases.json");
    expect(cases.semanticCore).toBe(SEMANTIC_CORE_VERSION);
    const byId = new Map((cases.cases as Json[]).map((c) => [String(c.id), c]));
    for (const id of [
      "type-kernel",
      "type-decimal",
      "type-unit",
      "type-object-by-title",
      "type-enum-by-id",
      "type-import",
      "type-unresolved",
      "mult-one",
      "mult-optional",
      "mult-many-flags",
      "mult-range",
      "mult-empty",
      "mult-inverted",
      "mult-flag-on-single",
      "con-min-maxlen-identity",
      "con-pattern",
      "con-enum-values",
      "con-non-empty",
      "con-format",
      "con-unknown-keyword",
      "fence-item",
      "fence-part-def",
      "fence-specializes",
      "reader-rule-bare-decimal",
    ])
      expect(byId.has(id), id).toBe(true);
    for (const entry of cases.cases as Json[]) {
      const expected = entry.expected as Json | undefined;
      if (expected) {
        if ("target" in expected) validates("TypeRef", expected);
        if ("multiplicity" in expected && !("target" in expected))
          validates("Multiplicity", expected.multiplicity);
        for (const constraint of (expected.constraints as Json[] | undefined) ??
          [])
          validates("ConstraintDecl", constraint);
      }
      for (const diagnostic of (entry.diagnostics as Json[] | undefined) ??
        []) {
        expect(typeof diagnostic.code).toBe("string");
        expect(["error", "advisory"]).toContain(diagnostic.severity);
        expect(["row", "fence-line"]).toContain(diagnostic.locus);
      }
      expect(
        expected !== undefined ||
          (entry.diagnostics as Json[] | undefined)?.length,
        String(entry.id),
      ).toBeTruthy();
    }
    expect((byId.get("type-unresolved")?.expected as Json).target).toBe(
      "ix://agent-ix/config-service/unresolved/Mystery",
    );
    expect(
      (byId.get("mult-flag-on-single")?.diagnostics as Json[])[0]?.code,
    ).toBe("semantic.invalid-multiplicity");
    expect(
      (byId.get("reader-rule-bare-decimal")?.diagnostics as Json[])[0]?.code,
    ).toBe("agent-ix.semantic-core.MISSING_DECIMAL_POLICY");
  });

  // Trace: FR-071-CON-1
  it("keeps the fence subset to attribute and ref item lines with brace content opaque", () => {
    const fence = fixture("config-version.fence.md");
    const body = fence.split("```sysml")[1]?.split("```")[0] ?? "";
    for (const line of body.trim().split("\n")) {
      expect(line, line).toMatch(FENCE_LINE);
    }
    // The other half of FR-071-CON-1 — that quoin's classifier parses no
    // expression and evaluates nothing — is asserted over the engine that now
    // owns it: `tc_452_652` in
    // `rust/crates/quoin-semantic/tests/tc_452_sweep_criteria.rs`.
  });
});

describe("FR-072 Invariants and Operations fixtures", () => {
  // Trace: FR-072-AC-1, FR-072-AC-4
  it("ships an operations fixture whose expected ClauseRef[] and OperationDecl[] validate", () => {
    const expected = json("operations.expected.json");
    expect(expected.semanticCore).toBe(SEMANTIC_CORE_VERSION);
    for (const clause of expected.clauses as Json[])
      validates("ClauseRef", clause);
    for (const operation of expected.operations as Json[])
      validates("OperationDecl", operation);
    const source = fixture("operations.md");
    // Clause text is recorded verbatim beside the ClauseRef, keyed by clause id,
    // and equals the fence body at the recorded span.
    const lines = source.split("\n");
    for (const clause of expected.clauses as Json[]) {
      const span = clause.sourceSpan as Json;
      expect(lines[Number(span.startLine) - 1]).toBe("```ocl");
      expect(lines[Number(span.endLine) - 1]).toBe("```");
      expect((expected.clauseText as Json)[String(clause.clauseId)]).toBe(
        lines[Number(span.startLine)],
      );
    }
    expect(source).toContain("### notArchived");
    expect(source).toContain("Pre: notArchived");
    expect(source).toContain("Post: archived");
    expect(source).toContain("Returns: ConfigVersion[1]");
    const operation = (expected.operations as Json[])[0] as Json;
    expect((operation.pre as Json[])[0]?.clauseId).toBe("notArchived");
    expect((operation.post as Json[])[0]?.clauseId).toBe("archived");
  });

  // Trace: FR-072-AC-2, FR-072-AC-3, FR-072-AC-5, FR-072-AC-6
  it("records every clause-language, duplicate, dangling, and dual-authority case with a located diagnostic", () => {
    const cases = json("operations-cases.json");
    const byId = new Map((cases.cases as Json[]).map((c) => [String(c.id), c]));
    const expectCode = (id: string, code: string, severity: string) => {
      const diagnostics = byId.get(id)?.diagnostics as Json[] | undefined;
      expect(diagnostics?.[0]?.code, id).toBe(code);
      expect(diagnostics?.[0]?.severity, id).toBe(severity);
      expect(typeof diagnostics?.[0]?.locus, id).toBe("string");
    };
    expectCode(
      "fence-no-language",
      "semantic.clause-language-missing",
      "error",
    );
    expectCode(
      "fence-bare-unknown",
      "semantic.clause-language-invalid",
      "error",
    );
    for (const id of [
      "fence-sysml-advisory",
      "fence-fretish-advisory",
      "fence-namespaced-advisory",
    ])
      expectCode(id, "semantic.clause-language-unchecked", "advisory");
    expectCode("duplicate-clause-id", "semantic.duplicate-clause-id", "error");
    expectCode(
      "clause-id-not-identifier",
      "semantic.clause-id-not-identifier",
      "error",
    );
    expectCode("dangling-post", "semantic.dangling-clause-ref", "error");
    expectCode("duplicate-operation", "semantic.duplicate-operation", "error");
    expectCode(
      "inline-and-external",
      "semantic.duplicate-clause-authority",
      "error",
    );
  });
});
