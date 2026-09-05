import { describe, expect, it } from "vitest";

import { LEDGER } from "../src/measurement/tool-defects.js";
import {
  PartitionError,
  identity,
  partition,
  type Finding,
} from "../src/measurement/partition.js";

const finding = (over: Partial<Finding> = {}): Finding => ({
  repository: "spec-objects-business",
  path: "spec/functional/FR-001.md",
  check: "frontmatter",
  code: "ValidationError",
  message: "title is a required property",
  ...over,
});

describe("TC-1540..1547 the partition drops nothing and hides nothing", () => {
  // TC-1540
  it("identifies a finding without its line, so a moved line keeps its class", () => {
    const a = finding();
    const b = finding();
    expect(identity(a)).toBe(identity(b));
    expect(identity(a)).not.toContain("line");
    expect(identity(finding({ check: "missing" }))).not.toBe(identity(a));
  });

  // TC-1541
  it("classifies an undeclared failure as unknown rather than absorbing it", () => {
    const result = partition({ findings: [finding()], ledger: LEDGER });
    expect(result.tally.unknown).toBe(1);
    expect(result.unknown).toBe(1);
    expect(result.undispositioned).toBe(1);
    expect(result.classified[0]?.classification).toBe("unknown");
  });

  // TC-1542
  it("classifies a ledger-covered failure with its citation", () => {
    const result = partition({
      findings: [finding({ path: "spec/tests.md", check: "trace-resolution" })],
      ledger: LEDGER,
    });
    expect(result.tally["tool-defect"]).toBe(1);
    expect(result.classified[0]?.citation).toBe("agent-ix/quire-rs#402");
    expect(result.unknown).toBe(0);
  });

  // TC-1543
  it("reports ledger entries that matched nothing", () => {
    const result = partition({ findings: [finding()], ledger: LEDGER });
    // Nothing matched, so every entry is reported unmatched. An entry covering
    // no finding is fixed, mis-scoped, or was never real.
    expect(result.unmatchedLedgerEntries).toHaveLength(LEDGER.length);
    expect(result.unmatchedLedgerEntries).toContain("range-ids-unresolvable");
  });

  // TC-1544
  it("refuses a bare role as an owner", () => {
    const authored = new Map([
      [
        identity(finding()),
        {
          classification: "malformed-document" as const,
          disposition: { owner: "the team", kind: "fix-in-this-campaign" as const },
        },
      ],
    ]);
    expect(() =>
      partition({ findings: [finding()], ledger: LEDGER, authored }),
    ).toThrow(/bare role "the team"/);
  });

  // TC-1545
  it("refuses a deferral that does not name the campaign", () => {
    const authored = new Map([
      [
        identity(finding()),
        {
          classification: "legitimate-undeclared-value" as const,
          disposition: {
            owner: "agent-ix/spec-objects-business",
            kind: "deferred-to-later-campaign" as const,
          },
        },
      ],
    ]);
    expect(() =>
      partition({ findings: [finding()], ledger: LEDGER, authored }),
    ).toThrow(PartitionError);
  });

  // TC-1546
  it("accepts an authored classification that names its owner and nomination", () => {
    const authored = new Map([
      [
        identity(finding()),
        {
          classification: "malformed-document" as const,
          disposition: {
            owner: "agent-ix/spec-objects-business",
            kind: "deferred-to-later-campaign" as const,
            nomination: "corpus normalization campaign",
          },
        },
      ],
    ]);
    const result = partition({ findings: [finding()], ledger: LEDGER, authored });
    expect(result.tally["malformed-document"]).toBe(1);
    expect(result.undispositioned).toBe(0);
  });

  // TC-1547
  it("asserts the classes sum to the finding count", () => {
    const findings = [
      finding(),
      finding({ path: "spec/tests.md", check: "trace-resolution" }),
      finding({ path: "spec/b.md", check: "missing" }),
    ];
    const result = partition({ findings, ledger: LEDGER });
    const sum = Object.values(result.tally).reduce((n, x) => n + x, 0);
    expect(sum).toBe(findings.length);
  });
});
