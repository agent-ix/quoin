/**
 * FR-040 — the assurance-case view (TC-221..TC-230, TC-237, TC-238, TC-261,
 * TC-262).
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
    // A goal with no sub-goal and no evidence is open. Rendering it supported
    // would assure a claim on the strength of nobody having written anything
    // against it.
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

  // Trace: FR-040-AC-11
  it("shows a requirement that refines two claims under both", () => {
    // Sharing one visited-set across claims put it under the first only, and
    // the second reported "no sub-claim traces to this claim" — a FALSE
    // statement, in an assurance case, about the very edge its author wrote.
    // Cycle prevention and "already rendered somewhere" are different questions.
    const documents = [
      doc("StR-001", "StR", "One"),
      doc("StR-002", "StR", "Two"),
      doc("FR-001", "FR", "Shared", [
        ["StR-001", "traces_to"],
        ["StR-002", "traces_to"],
      ]),
    ];
    const result = buildCase({
      documents,
      obligations: [obligation("FR-001-AC-1")],
      findings: [],
    });
    expect(result.claims.map((c) => c.children.map((x) => x.id))).toEqual([
      ["FR-001"],
      ["FR-001"],
    ]);
    expect(result.claims.every((c) => c.because === undefined)).toBe(true);
  });

  // Trace: FR-040-AC-12
  it("stops at a cycle without dropping a legitimately shared child", () => {
    // A refines B refines A. The recursion must terminate, and it must do so by
    // path and not by "seen anywhere", or the fix above regresses.
    const documents = [
      doc("StR-001", "StR", "Root"),
      doc("FR-001", "FR", "A", [
        ["StR-001", "traces_to"],
        ["FR-002", "refines"],
      ]),
      doc("FR-002", "FR", "B", [["FR-001", "refines"]]),
    ];
    const result = buildCase({ documents, obligations: [], findings: [] });
    expect(result.claims).toHaveLength(1);
    expect(JSON.stringify(result.claims[0]).length).toBeLessThan(4000);
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

  // Trace: FR-040-AC-13
  // TC-261
  it("carries a machine-readable reason exactly when nothing is a claim", () => {
    // `--json` emits `buildCase`'s result verbatim, so this field is what lets
    // a pipeline tell "the case is clean" from "nothing matched, so nothing
    // was argued" — `claims: []` alone reads the same both ways (#170).
    const empty = buildCase({
      documents: bundle(),
      obligations: [obligation("FR-001-AC-1")],
      findings: [],
      claimTypes: ["hazard"],
    });
    expect(empty.claims).toHaveLength(0);
    expect(empty.reason).toContain("no document declares itself a top-level");
    // The SEARCHED types, as the caller spelled them: the reader of the JSON
    // needs to see that `hazard` is what argued nothing.
    expect(empty.reason).toContain("hazard");
    const json = JSON.parse(JSON.stringify(empty)) as { reason?: string };
    expect(json.reason).toBe(empty.reason);

    // And ABSENT — not empty-string — on a case with claims, so presence of
    // the key is itself the signal.
    const nonEmpty = buildCase({
      documents: bundle(),
      obligations: [],
      findings: [],
    });
    expect(nonEmpty.claims).toHaveLength(1);
    expect("reason" in nonEmpty).toBe(false);
  });

  // Trace: FR-040-AC-14
  // TC-262
  it("matches --claim-type case-insensitively", () => {
    // `str`, `STR` and `Hazard` all matched nothing under `===` and exited 0
    // with an empty case — silence indistinguishable from a clean corpus.
    const documents = [
      doc("HAZ-001", "hazard", "Uncommanded actuation"),
      doc("FR-001", "FR", "Interlock", [["HAZ-001", "mitigates"]]),
    ];
    const upper = buildCase({
      documents,
      obligations: [],
      findings: [],
      claimTypes: ["Hazard"],
    });
    expect(upper.claims.map((c) => c.id)).toEqual(["HAZ-001"]);

    // The default matches an authored `str` too — the vocabulary is module
    // data, and casing is not part of what a claim type means.
    const lowercased = buildCase({
      documents: [doc("StR-001", "str", "The system is usable")],
      obligations: [],
      findings: [],
    });
    expect(lowercased.claims.map((c) => c.id)).toEqual(["StR-001"]);
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
