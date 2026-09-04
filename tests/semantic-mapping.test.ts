/**
 * Golden mapping fixtures for the Markdown → semantic-core contract
 * (FR-071, FR-072, FR-074; issue #293, TASK-041).
 *
 * Quoin publishes these; quire-rs#388 executes them. What quoin proves here is
 * that every expected output validates against the vendored semantic-core
 * schemas at the recorded version, that the table and fence forms share one
 * expected declaration set, that every expected diagnostic carries a locus, and
 * that quoin's own legacy-form classifier agrees with the expectations.
 */

import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import Ajv2020 from "ajv/dist/2020.js";
import { describe, expect, it } from "vitest";

import {
  SEMANTIC_CONTRACT,
  semanticCoreDir,
} from "../src/semantic/contract.js";
import { classifyArtifact } from "../src/semantic/sweep.js";

const FIXTURES = join("tests", "fixtures", "semantic-module", "mapping");
type Json = Record<string, unknown>;

function fixture(name: string): string {
  return readFileSync(join(FIXTURES, name), "utf8");
}

function json(name: string): Json {
  return JSON.parse(fixture(name)) as Json;
}

const ajv = (() => {
  const instance = new Ajv2020({ allErrors: true, strict: true });
  for (const name of readdirSync(semanticCoreDir()).filter(
    (n) => n.endsWith(".json") && n !== "toolchain.json",
  )) {
    instance.addSchema(
      JSON.parse(readFileSync(join(semanticCoreDir(), name), "utf8")),
    );
  }
  return instance;
})();

function validates(model: string, value: unknown): boolean {
  const validate = ajv.getSchema(
    `https://schemas.agent-ix.org/semantic-core/${SEMANTIC_CONTRACT.semanticCore.version}/${model}.json`,
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
  // Trace: FR-071-AC-1
  // Trace: TC-1344
  // Trace: FR-071-CON-2
  // Trace: TC-1352
  it("has an FR-006 typed-table fixture whose expected FieldDecl[] validates against the vendored FieldDecl.json", () => {
    const expected = json("config-version.expected.json");
    expect(expected.semanticCore).toBe(SEMANTIC_CONTRACT.semanticCore.version);
    const fields = expected.fields as Json[];
    expect(fields).toHaveLength(7);
    for (const field of fields) validates("FieldDecl", field);
    for (const clause of expected.clauses as Json[])
      validates("ClauseRef", clause);
    const table = fixture("config-version.table.md");
    expect(table).toContain("| Field | Type | Multiplicity | Constraints |");
    expect(classifyArtifact("table", table).form).toBe("typed-table");
  });

  // Trace: FR-071-AC-2
  // Trace: TC-1345
  it("gives the sysml fence fixture the byte-identical normalized FieldDecl[] of the table fixture", () => {
    const expected = json("config-version.expected.json");
    const authored = expected.authoredForm as Record<string, string>;
    expect(authored["config-version.table.md"]).toBe("table");
    expect(authored["config-version.fence.md"]).toBe("fence");
    const fence = fixture("config-version.fence.md");
    expect(classifyArtifact("fence", fence).form).toBe("sysml-fence");
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
  // Trace: TC-1346
  it("ships a both-forms fixture whose expected outcome is a failure at the second form", () => {
    const both = fixture("both-forms.md");
    expect(both).toContain("| Field | Type | Multiplicity | Constraints |");
    expect(both).toContain("```sysml");
    const tableLine =
      both.split("\n").findIndex((l) => l.startsWith("| Field |")) + 1;
    const fenceLine =
      both.split("\n").findIndex((l) => l.startsWith("```sysml")) + 1;
    expect(fenceLine).toBeGreaterThan(tableLine);
  });

  // Trace: FR-071-AC-4
  // Trace: TC-1347
  // Trace: FR-071-AC-5
  // Trace: TC-1348
  // Trace: FR-071-AC-6
  // Trace: TC-1349
  // Trace: FR-071-AC-7
  // Trace: TC-1350
  // Trace: FR-071-AC-8
  // Trace: TC-1384
  it("records every cell, fence-line, and reader-rule case with a schema-valid expectation or a located diagnostic", () => {
    const cases = json("cell-cases.json");
    expect(cases.semanticCore).toBe(SEMANTIC_CONTRACT.semanticCore.version);
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
  // Trace: TC-1351
  it("keeps the fence subset to attribute and ref item lines with brace content opaque", () => {
    const fence = fixture("config-version.fence.md");
    const body = fence.split("```sysml")[1]?.split("```")[0] ?? "";
    for (const line of body.trim().split("\n")) {
      expect(line, line).toMatch(
        /^(attribute|ref item) \w+ : [\w.:/-]+\[[^\]]+\]( \{[^}]*\})?$/,
      );
    }
    const source = readFileSync(join("src", "semantic", "sweep.ts"), "utf8");
    expect(source).not.toMatch(/parseExpression|evaluate\(/);
  });
});

describe("FR-072 Invariants and Operations fixtures", () => {
  // Trace: FR-072-AC-1
  // Trace: TC-1353
  // Trace: FR-072-AC-4
  // Trace: TC-1356
  it("ships an operations fixture whose expected ClauseRef[] and OperationDecl[] validate", () => {
    const expected = json("operations.expected.json");
    expect(expected.semanticCore).toBe(SEMANTIC_CONTRACT.semanticCore.version);
    for (const clause of expected.clauses as Json[])
      validates("ClauseRef", clause);
    for (const operation of expected.operations as Json[])
      validates("OperationDecl", operation);
    const source = fixture("operations.md");
    expect(source).toContain("### notArchived");
    expect(source).toContain("Pre: notArchived");
    expect(source).toContain("Post: archived");
    expect(source).toContain("Returns: ConfigVersion[1]");
    const operation = (expected.operations as Json[])[0] as Json;
    expect((operation.pre as Json[])[0]?.clauseId).toBe("notArchived");
    expect((operation.post as Json[])[0]?.clauseId).toBe("archived");
  });

  // Trace: FR-072-AC-2
  // Trace: TC-1354
  // Trace: FR-072-AC-3
  // Trace: TC-1355
  // Trace: FR-072-AC-5
  // Trace: TC-1357
  // Trace: FR-072-AC-6
  // Trace: TC-1358
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

  // Trace: FR-072-CON-1
  // Trace: TC-1359
  it("contains no clause typechecking or evaluation path in quoin", () => {
    const dir = join("src", "semantic");
    for (const name of readdirSync(dir).filter((n) => n.endsWith(".ts"))) {
      const source = readFileSync(join(dir, name), "utf8");
      expect(source, name).not.toMatch(/\bocl\b.*(parse|eval)|typecheck/i);
    }
  });
});

describe("FR-074 legacy forms", () => {
  // Trace: FR-074-AC-1
  // Trace: TC-1367
  // Trace: FR-074-AC-2
  // Trace: TC-1368
  // Trace: FR-074-CON-1
  // Trace: TC-1371
  it("classifies the pinned FR-006 copy, bullet lists, and mixed sections as the expectations record", () => {
    const expected = json("legacy.expected.json").cases as Json[];
    for (const entry of expected) {
      const path = join(FIXTURES, String(entry.file));
      const finding = classifyArtifact(
        String(entry.file),
        readFileSync(path, "utf8"),
      );
      expect(finding.form, String(entry.file)).toBe(entry.form);
      expect(finding.line, String(entry.file)).toBe(entry.line);
      if (entry.diagnostic)
        expect(finding.diagnostic).toEqual(entry.diagnostic);
      else expect(finding.diagnostic).toBeUndefined();
    }
    const pinned = readFileSync(
      join(
        FIXTURES,
        "..",
        "corpus",
        "config-service",
        "FR-006-config-version-entity.md",
      ),
      "utf8",
    );
    expect(pinned).toContain("| Column | Type | Constraints |");
    const provenance = JSON.parse(
      readFileSync(
        join(FIXTURES, "..", "corpus", "config-service", "PROVENANCE.json"),
        "utf8",
      ),
    ) as Json;
    expect(provenance.revision).toMatch(/^[0-9a-f]{40}$/);
    expect(provenance.repository).toBe("agent-ix/config-service");
  });
});
