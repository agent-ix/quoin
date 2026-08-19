/**
 * FR-040 — the assurance-case view (TC-221..TC-230).
 */

import { describe, expect, it } from "vitest";

import {
  buildCase,
  renderCase,
  requirementOf,
} from "../src/assurance/index.js";
import type { BundleDocument } from "../src/completeness/index.js";
import type { Finding } from "../src/auditor/index.js";
import type { Obligation } from "../src/quire/index.js";

const doc = (
  id: string,
  type: string,
  title: string,
  parents: Array<[string, string]> = [],
): BundleDocument => ({
  path: `spec/${id}.md`,
  frontmatter: {
    id,
    type,
    title,
    relationships: parents.map(([target, kind]) => ({
      target: `ix://agent-ix/quoin/${target}`,
      type: kind,
    })),
  },
  body: "",
});

const obligation = (id: string, statement = "It holds."): Obligation =>
  ({
    source: "acceptance-criterion",
    id,
    document: "spec/x.md",
    statement,
    statement_hash: "h",
  }) as Obligation;

const finding = (obligationId: string, kind: string): Finding =>
  ({
    kind,
    obligation: obligationId,
    severity: "medium",
    summary: `${obligationId} has a problem`,
  }) as Finding;

/** StR-001 ← FR-001 (traces_to), with one criterion. */
function bundle(): BundleDocument[] {
  return [
    doc("StR-001", "StR", "The system is usable"),
    doc("FR-001", "FR", "Parse the input", [["StR-001", "traces_to"]]),
  ];
}

describe("building the case", () => {
  // Trace: FR-040-AC-1
  it("argues from the claim down to its evidence", () => {
    const result = buildCase({
      documents: bundle(),
      obligations: [obligation("FR-001-AC-1")],
      findings: [],
    });
    expect(result.claims).toHaveLength(1);
    const claim = result.claims[0];
    expect(claim.id).toBe("StR-001");
    expect(claim.status).toBe("supported");
    expect(claim.children[0].id).toBe("FR-001");
    expect(claim.children[0].children[0]).toMatchObject({
      id: "FR-001-AC-1",
      kind: "solution",
      status: "supported",
    });
  });

  // Trace: FR-040-AC-2
  it("keeps an obligation with a finding in the tree, as an open node", () => {
    // The point of the whole view. Dropping it produces a case that reads as
    // complete, which is the failure mode an assurance case invites most.
    const result = buildCase({
      documents: bundle(),
      obligations: [obligation("FR-001-AC-1")],
      findings: [finding("FR-001-AC-1", "undischarged")],
    });
    const leaf = result.claims[0].children[0].children[0];
    expect(leaf.status).toBe("open");
    expect(leaf.because).toContain("undischarged");
  });

  // Trace: FR-040-AC-3
  it("propagates open upward, so a claim is not supported by a broken branch", () => {
    const result = buildCase({
      documents: bundle(),
      obligations: [obligation("FR-001-AC-1"), obligation("FR-001-AC-2")],
      findings: [finding("FR-001-AC-2", "suspect-link")],
    });
    expect(result.claims[0].status).toBe("open");
    expect(result.claims[0].children[0].status).toBe("open");
  });

  // Trace: FR-040-AC-4
  it("marks a claim nothing traces to as undeveloped, not as met", () => {
    // A goal with no sub-goal and no evidence is GSN-undeveloped. Rendering it
    // supported would assure a claim on the strength of nobody having written
    // anything against it.
    const result = buildCase({
      documents: [doc("StR-002", "StR", "Nothing argues for this")],
      obligations: [],
      findings: [],
    });
    expect(result.claims[0].status).toBe("open");
    expect(result.claims[0].because).toContain("no sub-claim");
  });

  // Trace: FR-040-AC-5
  it("reports requirements no claim reaches instead of rendering around them", () => {
    // A case drawn only over the reachable half reads as complete. The orphan
    // has obligations, which is what makes its absence from the argument a gap
    // rather than a document nobody needed.
    const result = buildCase({
      documents: [...bundle(), doc("FR-009", "FR", "Orphaned")],
      obligations: [obligation("FR-001-AC-1"), obligation("FR-009-AC-1")],
      findings: [],
    });
    expect(result.unreachable).toEqual(["FR-009"]);
  });

  // Trace: FR-040-AC-6
  it("reads the claim type from the caller, never a constant", () => {
    // A safety bundle argues from a declared hazard. The vocabulary is module
    // data, so a built-in `StR` would make this view useless to exactly the
    // bundles that need an assurance case most.
    const documents = [
      doc("HAZ-001", "hazard", "Uncommanded actuation"),
      doc("FR-001", "FR", "Interlock", [["HAZ-001", "mitigates"]]),
    ];
    const withDefault = buildCase({ documents, obligations: [], findings: [] });
    expect(withDefault.claims).toHaveLength(0);
    const withHazard = buildCase({
      documents,
      obligations: [],
      findings: [],
      claimTypes: ["hazard"],
    });
    expect(withHazard.claims[0].id).toBe("HAZ-001");
    expect(withHazard.claims[0].children[0].id).toBe("FR-001");
  });

  // Trace: FR-040-AC-7
  it("derives an obligation's owner from its id", () => {
    expect(requirementOf("FR-001-AC-3")).toBe("FR-001");
    expect(requirementOf("NFR-010-M-2")).toBe("NFR-010");
    expect(requirementOf("TC-EV-001")).toBe(
      "TC-EV-001".match(/^([A-Za-z]+-\d+)/)?.[1] ?? "TC-EV-001",
    );
  });
});

describe("rendering the case", () => {
  // Trace: FR-040-AC-8
  it("is byte-identical over unchanged inputs", () => {
    // A store view, never a hand-maintained document: a diff must mean the
    // evidence changed, not that somebody re-ran it.
    const input = {
      documents: bundle(),
      obligations: [obligation("FR-001-AC-1")],
      findings: [],
    };
    expect(renderCase(buildCase(input))).toBe(renderCase(buildCase(input)));
  });

  // Trace: FR-040-AC-9
  it("emits mermaid that cannot be broken by a statement's punctuation", () => {
    // Each of these renders a broken diagram rather than a wrong one, which is
    // worse: nobody reviews a diagram that will not draw.
    const result = buildCase({
      documents: bundle(),
      obligations: [
        obligation("FR-001-AC-1", 'Rejects (bad) input; logs "why"'),
      ],
      findings: [],
    });
    const mermaid = renderCase(result).split("```mermaid")[1].split("```")[0];
    expect(mermaid).toContain("FR_001_AC_1");
    expect(mermaid).not.toMatch(/FR-001-AC-1\(/);
    expect(mermaid).not.toContain(";");
    expect(mermaid).not.toMatch(/"[^"\n]*"[^"\n]*"/);
  });

  // Trace: FR-040-AC-10
  it("says there is no case rather than rendering an empty one", () => {
    // An empty document reads as "nothing to argue". The truth is that nothing
    // declared itself a claim, and those are different problems.
    const rendered = renderCase(
      buildCase({ documents: [], obligations: [], findings: [] }),
    );
    expect(rendered).toContain("No document declares itself a top-level claim");
  });
});
