/** FR-046 — use-specific producer trust and invalidation (TC-297..TC-302). */

import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { buildCase, renderCase } from "../src/assurance/index.js";
import {
  assessTrust,
  readTrustDecisions,
  trustDecisionPath,
  validateTrustDecision,
  writeTrustDecision,
  type ProducerContext,
  type TrustDecision,
} from "../src/evidence/index.js";

const context = (version = "1.0.0"): ProducerContext => ({
  name: "neutral-validator",
  version,
  configurationDigest: `sha256:${"a".repeat(64)}`,
  validationCorpusDigest: `sha256:${"b".repeat(64)}`,
  inputContract: "report-v1",
  environment: "linux-x64",
  adapter: { name: "report-reader", version: "2.0.0" },
});

function decision(
  id = "ETD-001",
  useId = "dependency-boundary-release-review",
): TrustDecision {
  return {
    schemaVersion: 1,
    id,
    use: {
      id: useId,
      intendedFunction: "detect dependency edges forbidden by project policy",
      permittedDecisions: ["inform release review"],
    },
    decision: "relied-upon",
    acceptedContext: context(),
    observedContext: context(),
    revalidateOn: [
      "producer-version",
      "configuration",
      "adapter",
      "validation-corpus",
      "input-contract",
      "environment",
    ],
    validationEvidence: [
      { id: "TV-001", reference: "spec/evidence/runs/TV-001.json" },
    ],
    limitations: [],
    owner: "platform-team",
    decidedAt: "2026-08-21T12:00:00Z",
  };
}

describe("TC-297 a decision is scoped to one use", () => {
  it("accepts an exact observed context without making a global tool badge", () => {
    expect(assessTrust(decision())).toMatchObject({
      id: "ETD-001",
      useId: "dependency-boundary-release-review",
      status: "accepted",
      triggeredBy: [],
    });
  });
});

describe("TC-298 changed context invalidates", () => {
  it("names every triggered field and never falls back to accepted", () => {
    const changed = decision();
    changed.observedContext = {
      ...context("1.1.0"),
      configurationDigest: `sha256:${"c".repeat(64)}`,
    };
    expect(assessTrust(changed)).toMatchObject({
      status: "invalidated",
      triggeredBy: ["producer-version", "configuration"],
    });
  });
});

describe("TC-299 absence is not acceptance or rejection", () => {
  it("reports an accepted decision with no observed context as unobserved", () => {
    const absent = decision();
    delete absent.observedContext;
    expect(assessTrust(absent).status).toBe("unobserved");
  });
});

describe("TC-300 the same producer can have different use decisions", () => {
  it("keeps use identity and accountable decision separate", () => {
    const accepted = decision("ETD-001", "advisory-review");
    const refused = decision("ETD-002", "automatic-release-approval");
    refused.decision = "not-relied-upon";
    expect(
      [assessTrust(accepted), assessTrust(refused)].map((x) => x.status),
    ).toEqual(["accepted", "not-accepted"]);
  });
});

describe("TC-301 trust records are canonical and invalid files stay visible", () => {
  it("round-trips valid decisions and reports skipped invalid records", () => {
    const repo = mkdtempSync(join(tmpdir(), "quoin-trust-"));
    const path = writeTrustDecision(repo, decision());
    expect(path).toBe(trustDecisionPath(repo, "ETD-001"));
    expect(readFileSync(path, "utf8")).toMatch(/\n$/);
    writeFileSync(join(path, "..", "ETD-002.json"), "{}\n");
    const skipped: string[] = [];
    expect(readTrustDecisions(repo, skipped).map((item) => item.id)).toEqual([
      "ETD-001",
    ]);
    expect(skipped[0]).toContain("ETD-002.json");
  });
});

describe("TC-302 assurance renders trust as context", () => {
  it("shows invalidation without changing claim support", () => {
    const changed = decision();
    changed.observedContext = context("1.1.0");
    const assurance = buildCase({
      documents: [],
      obligations: [],
      findings: [],
      producerTrust: [assessTrust(changed)],
    });
    expect(assurance.producerTrust[0].status).toBe("invalidated");
    expect(renderCase(assurance)).toContain(
      "invalidated — revalidate: producer-version",
    );
  });
});

describe("trust validation", () => {
  it("requires validation evidence and the minimum invalidation triggers", () => {
    const noEvidence = decision();
    noEvidence.validationEvidence = [];
    expect(() => validateTrustDecision(noEvidence)).toThrow(
      /must not be empty/,
    );
    const noConfigurationTrigger = decision();
    noConfigurationTrigger.revalidateOn = ["producer-version"];
    expect(() => validateTrustDecision(noConfigurationTrigger)).toThrow(
      /must include configuration/,
    );
  });
});
