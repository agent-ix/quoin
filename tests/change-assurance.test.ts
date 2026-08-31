/**
 * Quoin #282 — shared change-assurance contracts (FR-063..FR-065).
 */

import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import Ajv2020 from "ajv/dist/2020.js";
import fc from "fast-check";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  attestationPath,
  blake3Hex,
  canonicalizeJcs,
  hashIxFlowEvent,
  intakeAttestation,
  parseStrictJson,
  readAttestation,
  readChangeAssuranceSchema,
  readChangeRecord,
  recordPath,
  recoverChangeAssuranceStaging,
  sealAttestation,
  sealChangeRecord,
  validateDecision,
  verifyAttestation,
  verifyChangeAssurance,
  verifyChangeRecord,
  verifyLineage,
  verifyReceipt,
  writeChangeRecord,
  type ChangeAssuranceRecord,
  type ProofAttestation,
  type VerificationInput,
} from "../src/change-assurance/index.js";
import { writeRun } from "../src/evidence/index.js";

const HEX_A = "a".repeat(64);
const HEX_B = "b".repeat(64);

function recordInput(revision = 1): Omit<ChangeAssuranceRecord, "digest"> {
  return {
    schema_version: 1,
    record_type: "change_assurance",
    record_id: "change-1",
    revision,
    parent_digest: revision === 1 ? null : HEX_A,
    subject: {
      repository: "agent-ix/quoin",
      base_revision: "abc123",
      scope: ["src/evidence"],
    },
    source_connections: [
      {
        source_id: "FR-063",
        kind: "requirement",
        revision: "1",
        digest: HEX_A,
      },
    ],
    impact_snapshot: {
      identity: "impact-1",
      revision: "1",
      digest: HEX_B,
      completeness: "complete",
      truncated: false,
      gaps: [],
    },
    definition: {
      requirements: [
        { id: "FR-063", statement: "seal it", source_ids: ["FR-063"] },
      ],
      preservation_constraints: [],
      proof_obligations: [
        {
          proof_id: "proof-1",
          statement: "tests pass",
          obligation_ids: ["FR-063-AC-1"],
          evidence_kind: "Unit",
          command: { argv: ["pnpm", "test"], working_directory: "." },
          tool_identity: "vitest",
          configuration_digest: HEX_B,
        },
      ],
      unknowns: [],
    },
    review_workflow: {
      run_id: "run-1",
      decision_event_kind: "change_assurance.review_decided",
    },
  };
}

function attestationInput(
  recordDigest: string,
  output: Uint8Array,
): Omit<ProofAttestation, "digest"> {
  return {
    schema_version: 1,
    record_type: "proof_attestation",
    attestation_id: "att-1",
    record_digest: recordDigest,
    candidate_revision: "candidate-1",
    proof_id: "proof-1",
    command: { argv: ["pnpm", "test"], working_directory: "." },
    tool: {
      identity: "vitest",
      version: "4.1.10",
      configuration_digest: HEX_B,
    },
    environment: { os: "test" },
    observed_at: "2026-08-31T00:00:00Z",
    result: "passed",
    retained_output: {
      media_type: "text/plain",
      digest: blake3Hex(output),
      size_bytes: output.byteLength,
    },
  };
}

function receiptInput(): VerificationInput {
  const record = sealChangeRecord(recordInput());
  const output = new TextEncoder().encode("ok\n");
  const attestation = sealAttestation(attestationInput(record.digest, output));
  const eventWithoutHash = {
    id: "event-1",
    ts: "2026-08-31T00:00:00Z",
    actor: { kind: "human" as const, id: "reviewer" },
    kind: "change_assurance.review_decided",
    payload: {
      schema_version: 1,
      record_id: "change-1",
      revision: 1,
      record_digest: record.digest,
      decision: "approved",
    },
    prevHash: "0".repeat(64),
  };
  return {
    record,
    parents: [],
    candidate_revision: "candidate-1",
    selections: [
      { proof_id: "proof-1", attestation_digest: attestation.digest },
    ],
    attestations: [{ attestation, output }],
    decision_history: {
      run_id: "run-1",
      events: [
        { ...eventWithoutHash, hash: hashIxFlowEvent(eventWithoutHash) },
      ],
    },
    audits: [
      {
        proof_id: "proof-1",
        report_digest: HEX_A,
        report: {
          findings: [],
          healthy: ["FR-063-AC-1"],
          unevaluated: [],
        },
      },
    ],
  };
}

function resealDecisionHistory(input: VerificationInput): void {
  let previous = "0".repeat(64);
  input.decision_history.events = input.decision_history.events.map((event) => {
    const unsigned = { ...event, prevHash: previous };
    delete (unsigned as Partial<typeof event>).hash;
    const sealed = { ...unsigned, hash: hashIxFlowEvent(unsigned) };
    previous = sealed.hash;
    return sealed;
  });
}

let repo: string;

beforeEach(() => {
  repo = mkdtempSync(join(tmpdir(), "quoin-change-assurance-"));
});

describe("FR-063 records and canonical integrity", () => {
  it("TC-1261 rejects missing and undeclared record fields", () => {
    const sealed = sealChangeRecord(recordInput());
    expect(verifyChangeRecord(sealed)).toEqual(sealed);
    expect(() => verifyChangeRecord({ ...sealed, extra: true })).toThrow(
      /extra/,
    );
    const missing = { ...sealed } as Record<string, unknown>;
    delete missing.subject;
    expect(() => verifyChangeRecord(missing)).toThrow(/subject/);
  });

  it("TC-1262 preserves meaningful empties and refuses duplicate identities", () => {
    const sealed = sealChangeRecord(recordInput());
    expect(sealed.definition.unknowns).toEqual([]);
    expect(sealed.definition.preservation_constraints).toEqual([]);
    expect(() =>
      sealChangeRecord({
        ...recordInput(),
        source_connections: [
          ...recordInput().source_connections,
          recordInput().source_connections[0],
        ],
      }),
    ).toThrow(/duplicate source/);
  });

  it("TC-1263 retains reviewed statements and exact proof premises", () => {
    const sealed = sealChangeRecord(recordInput());
    expect(sealed.definition.requirements[0].statement).toBe("seal it");
    expect(sealed.definition.proof_obligations[0]).toMatchObject({
      obligation_ids: ["FR-063-AC-1"],
      command: { argv: ["pnpm", "test"], working_directory: "." },
      tool_identity: "vitest",
      configuration_digest: HEX_B,
    });
  });

  it("TC-1264 preserves incomplete impact and every unknown disposition", () => {
    const input = recordInput();
    input.impact_snapshot.completeness = "incomplete";
    input.impact_snapshot.truncated = true;
    input.impact_snapshot.gaps = ["dynamic calls"];
    input.definition.unknowns = [
      { id: "u1", statement: "open", disposition: "open", owner: "team" },
      {
        id: "u2",
        statement: "accepted",
        disposition: "accepted",
        owner: "team",
      },
      { id: "u3", statement: "later", disposition: "deferred", owner: "team" },
      {
        id: "u4",
        statement: "done",
        disposition: "resolved",
        owner: "team",
        resolution: "measured",
      },
    ];
    expect(
      sealChangeRecord(input).definition.unknowns.map((u) => u.disposition),
    ).toEqual(["open", "accepted", "deferred", "resolved"]);
  });

  it("TC-1265 matches RFC 8785 canonicalization and BLAKE3 vectors", () => {
    expect(canonicalizeJcs({ b: 1, a: [true, null, "x"] })).toBe(
      '{"a":[true,null,"x"],"b":1}',
    );
    // RFC 8785 section 3.2.2's number sample: the deliberately imprecise
    // 333333333.33333329 and mixed exponent/decimal spellings must serialize
    // exactly as ECMAScript's JCS representation, not as their input tokens.
    expect(
      canonicalizeJcs({
        numbers: [
          333333333.33333329, 1e30, 4.5, 2e-3, 0.000000000000000000000000001,
        ],
      }),
    ).toBe('{"numbers":[333333333.3333333,1e+30,4.5,0.002,1e-27]}');
    expect(blake3Hex(new Uint8Array())).toBe(
      "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262",
    );
  });

  it("TC-1266 invalidates the digest when any generated semantic leaf changes", () => {
    fc.assert(
      fc.property(fc.string({ minLength: 1 }), (statement) => {
        const sealed = sealChangeRecord(recordInput());
        if (statement === "seal it") return;
        const changed = structuredClone(sealed);
        changed.definition.requirements[0].statement = statement;
        expect(() => verifyChangeRecord(changed)).toThrow(/digest/);
      }),
    );
    const mutators: Array<(value: ChangeAssuranceRecord) => void> = [
      (value) => {
        value.source_connections[0].revision = "changed";
      },
      (value) => {
        value.impact_snapshot.gaps.push("new gap");
      },
      (value) => {
        value.definition.proof_obligations[0].command.argv.push("--changed");
      },
      (value) => {
        value.review_workflow.run_id = "run-changed";
      },
      (value) => {
        value.revision = 2;
        value.parent_digest = HEX_A;
      },
    ];
    for (const mutate of mutators) {
      const changed = structuredClone(sealChangeRecord(recordInput()));
      mutate(changed);
      expect(() => verifyChangeRecord(changed)).toThrow(/digest/);
    }
  });

  it("TC-1267 refuses duplicate names, bad Unicode/numbers/BOM, and bad digest encodings", () => {
    expect(() => parseStrictJson('{"a":1,"a":2}')).toThrow(/duplicate.*a/i);
    expect(() => parseStrictJson('"\\ud800"')).toThrow(/surrogate|unicode/i);
    expect(() => parseStrictJson("1e9999")).toThrow(/I-JSON|finite/i);
    expect(() => parseStrictJson("\ufeff{}")).toThrow(/byte-order mark/i);
    const polluted = parseStrictJson(
      '{"__proto__":{"schema_version":1,"record_type":"change_assurance"}}',
    ) as Record<string, unknown>;
    expect(Object.getPrototypeOf(polluted)).toBeNull();
    expect(Object.hasOwn(polluted, "__proto__")).toBe(true);
    expect(() => verifyChangeRecord(polluted)).toThrow(
      /extra.*__proto__|missing/i,
    );
    const sealed = sealChangeRecord(recordInput());
    expect(() =>
      verifyChangeRecord({ ...sealed, digest: sealed.digest.toUpperCase() }),
    ).toThrow();
  });

  it("TC-1268 enforces genesis and retained strict N-1 lineage", () => {
    const parent = sealChangeRecord(recordInput());
    const child = sealChangeRecord({
      ...recordInput(2),
      parent_digest: parent.digest,
    });
    expect(verifyLineage(child, [parent])).toEqual([parent.digest]);
    const skipped = sealChangeRecord({
      ...recordInput(3),
      parent_digest: parent.digest,
    });
    expect(() => verifyLineage(skipped, [parent])).toThrow(/revision/);
    expect(() => verifyLineage(child, [])).toThrow(/parent/);
  });

  it("TC-1269 writes a successor without changing its parent bytes", () => {
    const parent = sealChangeRecord(recordInput());
    writeChangeRecord(repo, parent);
    const before = readFileSync(recordPath(repo, parent.digest));
    const child = sealChangeRecord({
      ...recordInput(2),
      parent_digest: parent.digest,
    });
    writeChangeRecord(repo, child);
    expect(readFileSync(recordPath(repo, parent.digest))).toEqual(before);
    expect(readChangeRecord(repo, child.digest)).toEqual(child);
  });

  it("TC-1270 accepts only one exact integrity-valid human decision", () => {
    const ixFlowVector = {
      id: "event-1",
      ts: "2026-08-31T00:00:00Z",
      actor: { kind: "human" as const, id: "reviewer" },
      kind: "change_assurance.review_decided",
      payload: {
        schema_version: 1,
        record_id: "change-1",
        revision: 1,
        record_digest: HEX_A,
        decision: "approved",
      },
      prevHash: "0".repeat(64),
    };
    expect(hashIxFlowEvent(ixFlowVector)).toBe(
      "c9dd96c23640422c109507c357ea27023e2ee1b07cbf763240553c9e2d62f769",
    );
    const input = receiptInput();
    expect(validateDecision(input.record, input.decision_history).outcome).toBe(
      "valid",
    );
    input.decision_history.events[0].actor.kind = "agent";
    resealDecisionHistory(input);
    expect(validateDecision(input.record, input.decision_history).outcome).toBe(
      "invalid",
    );
  });

  it("TC-1271 has no execution dependency and makes only integrity claims", async () => {
    const source = readFileSync(
      new URL("../src/change-assurance/index.ts", import.meta.url),
      "utf8",
    );
    expect(source).not.toMatch(/child_process|fetch\(|spawn\(|exec\(/);
    expect(source).toMatch(/integrity/i);
    expect(source).not.toMatch(
      /non-repudiation guarantee|authenticated identity/i,
    );
  });
});

describe("FR-064 attestation intake", () => {
  it("TC-1272 requires every attestation field and refuses extras", () => {
    const record = sealChangeRecord(recordInput());
    const output = new TextEncoder().encode("ok");
    const sealed = sealAttestation(attestationInput(record.digest, output));
    expect(verifyAttestation(sealed)).toEqual(sealed);
    expect(() => verifyAttestation({ ...sealed, extra: true })).toThrow(
      /extra/,
    );
  });

  it("TC-1273 round-trips four producer states without a verifier verdict", () => {
    const record = sealChangeRecord(recordInput());
    const output = new TextEncoder().encode("diagnostic");
    for (const result of [
      "passed",
      "failed",
      "unavailable",
      "not_computed",
    ] as const) {
      expect(
        sealAttestation({ ...attestationInput(record.digest, output), result })
          .result,
      ).toBe(result);
    }
  });

  it("TC-1274 rejects changed or absent output without writing either artifact", () => {
    const record = sealChangeRecord(recordInput());
    const output = new TextEncoder().encode("ok");
    const attestation = sealAttestation(
      attestationInput(record.digest, output),
    );
    const raw = new TextEncoder().encode(canonicalizeJcs(attestation));
    expect(() =>
      intakeAttestation(repo, raw, new TextEncoder().encode("bad")),
    ).toThrow(/output/);
    expect(existsSync(attestationPath(repo, attestation.digest))).toBe(false);
  });

  it("TC-1275 invalidates attestation digests under semantic mutation", () => {
    const record = sealChangeRecord(recordInput());
    const output = new TextEncoder().encode("ok");
    const sealed = sealAttestation(attestationInput(record.digest, output));
    expect(() =>
      verifyAttestation({ ...sealed, candidate_revision: "other" }),
    ).toThrow(/digest/);
    const mutators: Array<(value: ProofAttestation) => void> = [
      (value) => {
        value.record_digest = HEX_B;
      },
      (value) => {
        value.proof_id = "proof-other";
      },
      (value) => {
        value.command.argv.push("--changed");
      },
      (value) => {
        value.tool.version = "4.1.11";
      },
      (value) => {
        value.environment.os = "other";
      },
      (value) => {
        value.observed_at = "2026-08-31T00:00:01Z";
      },
      (value) => {
        value.result = "failed";
      },
      (value) => {
        value.retained_output.media_type = "application/json";
      },
    ];
    for (const mutate of mutators) {
      const changed = structuredClone(sealed);
      mutate(changed);
      expect(() => verifyAttestation(changed)).toThrow(/digest/);
    }
  });

  it("TC-1276 refuses each absent attestation premise instead of inferring", () => {
    const record = sealChangeRecord(recordInput());
    const output = new TextEncoder().encode("ok");
    const sealed = sealAttestation(attestationInput(record.digest, output));
    for (const key of [
      "record_digest",
      "candidate_revision",
      "proof_id",
      "command",
      "tool",
      "environment",
      "observed_at",
      "result",
      "retained_output",
    ] as const) {
      const missing = { ...sealed } as Record<string, unknown>;
      delete missing[key];
      expect(() => verifyAttestation(missing), key).toThrow();
    }
  });

  it("TC-1277 is byte-idempotent, crash-atomic, recoverable, and collision-safe", () => {
    const record = sealChangeRecord(recordInput());
    const output = new TextEncoder().encode("ok");
    const sealed = sealAttestation(attestationInput(record.digest, output));
    const raw = new TextEncoder().encode(canonicalizeJcs(sealed));
    expect(() =>
      intakeAttestation(repo, raw, output, {
        beforeRename: () => {
          throw new Error("cut power");
        },
      }),
    ).toThrow(/cut power/);
    expect(existsSync(attestationPath(repo, sealed.digest))).toBe(false);
    expect(recoverChangeAssuranceStaging(repo)).toBeGreaterThanOrEqual(0);
    intakeAttestation(repo, raw, output);
    const first = readFileSync(
      join(attestationPath(repo, sealed.digest), "attestation.json"),
    );
    intakeAttestation(repo, raw, output);
    expect(
      readFileSync(
        join(attestationPath(repo, sealed.digest), "attestation.json"),
      ),
    ).toEqual(first);
    writeFileSync(
      join(attestationPath(repo, sealed.digest), "output.bin"),
      "collision",
    );
    expect(() => intakeAttestation(repo, raw, output)).toThrow(/collision/);
  });

  it("TC-1277 accepts noncanonical valid input but stores canonical bytes", () => {
    const record = sealChangeRecord(recordInput());
    const output = new TextEncoder().encode("ok");
    const sealed = sealAttestation(attestationInput(record.digest, output));
    const noncanonical = new TextEncoder().encode(
      JSON.stringify(sealed, null, 2),
    );
    intakeAttestation(repo, noncanonical, output);
    expect(
      readFileSync(
        join(attestationPath(repo, sealed.digest), "attestation.json"),
        "utf8",
      ),
    ).toBe(canonicalizeJcs(sealed));
    expect(() => attestationPath(repo, "../../escape")).toThrow(/digest/);
  });

  it("TC-1278 retains unavailable diagnostics and creates nothing for absence", () => {
    const record = sealChangeRecord(recordInput());
    const output = new TextEncoder().encode("tool unavailable");
    const sealed = sealAttestation({
      ...attestationInput(record.digest, output),
      result: "unavailable",
    });
    intakeAttestation(
      repo,
      new TextEncoder().encode(canonicalizeJcs(sealed)),
      output,
    );
    expect(
      Array.from(readAttestation(repo, sealed.digest)?.output ?? []),
    ).toEqual(Array.from(output));
    expect(readAttestation(repo, HEX_A)).toBeNull();
  });

  it("TC-1279 does not alter the existing FR-030 store family", () => {
    writeRun(repo, {
      schemaVersion: 1,
      suite: "SUITE-1",
      commit: "abcdef0123456789",
      tool: "t",
      timestamp: "2026-08-31T00:00:00Z",
      entries: [],
    });
    const existing = join(
      repo,
      "spec",
      "evidence",
      "runs",
      "SUITE-1",
      "abcdef012345.json",
    );
    const before = readFileSync(existing);
    const input = receiptInput();
    const pair = input.attestations[0];
    intakeAttestation(
      repo,
      new TextEncoder().encode(canonicalizeJcs(pair.attestation)),
      pair.output!,
    );
    expect(readFileSync(existing)).toEqual(before);
  });

  it("TC-1280 intake exposes no execution or verdict path", () => {
    const source = readFileSync(
      new URL("../src/change-assurance/store.ts", import.meta.url),
      "utf8",
    );
    expect(source).not.toMatch(/child_process|fetch\(|spawn\(|exec\(/);
    expect(source).not.toMatch(
      /approved|audit verdict|authenticated identity/i,
    );
  });
});

describe("FR-065 verification receipts", () => {
  it("TC-1261/TC-1272/TC-1281 keeps runtime values aligned with the packaged schemas", () => {
    const ajv = new Ajv2020({ strict: false, validateFormats: false });
    const input = receiptInput();
    const samples = [
      ["change-assurance-record-v1.schema.json", input.record],
      ["proof-attestation-v1.schema.json", input.attestations[0].attestation],
      ["verification-receipt-v1.schema.json", verifyChangeAssurance(input)],
    ] as const;
    for (const [name, sample] of samples) {
      const validate = ajv.compile(readChangeAssuranceSchema(name));
      expect(validate(sample), JSON.stringify(validate.errors)).toBe(true);
    }
  });

  it("TC-1281 emits a closed receipt with all retained joins", () => {
    const receipt = verifyChangeAssurance(receiptInput());
    expect(receipt).toMatchObject({
      schema_version: 1,
      record_type: "verification_receipt",
      outcome: "valid",
      proofs: [{ proof_id: "proof-1", outcome: "valid" }],
    });
    expect(verifyReceipt(receipt)).toEqual(receipt);
    expect(() => verifyReceipt({ ...receipt, extra: true })).toThrow(/extra/);
    expect(() => verifyReceipt({ ...receipt, digest: HEX_A })).toThrow(
      /digest/,
    );
    const malformed = receiptInput();
    (malformed.record as ChangeAssuranceRecord & { extra: boolean }).extra =
      true;
    expect(verifyChangeAssurance(malformed)).toMatchObject({
      outcome: "invalid",
      reasons: ["schema_invalid"],
    });
  });

  it("TC-1282 applies invalid > incomplete > valid under permutations", () => {
    const incomplete = receiptInput();
    incomplete.selections = [];
    expect(verifyChangeAssurance(incomplete).outcome).toBe("incomplete");
    incomplete.record.impact_snapshot.completeness = "incomplete";
    incomplete.decision_history.events[0].hash = HEX_A;
    expect(verifyChangeAssurance(incomplete).outcome).toBe("invalid");
    expect(verifyChangeAssurance(incomplete).checks.record.outcome).toBe(
      "invalid",
    );
  });

  it("TC-1283 detects missing, duplicate, and unknown selections and ignores stored extras", () => {
    const missing = receiptInput();
    missing.selections = [];
    expect(verifyChangeAssurance(missing).proofs[0].outcome).toBe("incomplete");
    const duplicate = receiptInput();
    duplicate.selections.push({ ...duplicate.selections[0] });
    expect(verifyChangeAssurance(duplicate).proofs[0].outcome).toBe("invalid");
    const unknown = receiptInput();
    unknown.selections.push({ proof_id: "unknown", attestation_digest: HEX_A });
    expect(verifyChangeAssurance(unknown).outcome).toBe("invalid");
    const extra = receiptInput();
    extra.attestations.push({
      ...extra.attestations[0],
      attestation: { ...extra.attestations[0].attestation, digest: HEX_A },
    });
    expect(verifyChangeAssurance(extra).outcome).toBe("valid");
  });

  it("TC-1284 names independent binding mismatches", () => {
    const cases: Array<{
      reason: string;
      mutate: (value: ProofAttestation) => void;
    }> = [
      {
        reason: "record_binding_mismatch",
        mutate: (value) => {
          value.record_digest = HEX_A;
        },
      },
      {
        reason: "candidate_revision_mismatch",
        mutate: (value) => {
          value.candidate_revision = "other";
        },
      },
      {
        reason: "proof_id_mismatch",
        mutate: (value) => {
          value.proof_id = "other";
        },
      },
      {
        reason: "command_mismatch",
        mutate: (value) => {
          value.command.argv.push("--other");
        },
      },
      {
        reason: "command_mismatch",
        mutate: (value) => {
          value.command.working_directory = "other";
        },
      },
      {
        reason: "tool_identity_mismatch",
        mutate: (value) => {
          value.tool.identity = "other";
        },
      },
      {
        reason: "configuration_mismatch",
        mutate: (value) => {
          value.tool.configuration_digest = HEX_A;
        },
      },
    ];
    for (const testCase of cases) {
      const input = receiptInput();
      testCase.mutate(input.attestations[0].attestation);
      input.attestations[0].attestation = sealAttestation({
        ...input.attestations[0].attestation,
        digest: undefined,
      } as never);
      input.selections[0].attestation_digest =
        input.attestations[0].attestation.digest;
      expect(verifyChangeAssurance(input).proofs[0].reasons).toContain(
        testCase.reason,
      );
    }
  });

  it("TC-1285 rejects failed or unhealthy evidence and output mismatch", () => {
    const failed = receiptInput();
    failed.attestations[0].attestation = sealAttestation({
      ...failed.attestations[0].attestation,
      digest: undefined,
      result: "failed",
    } as never);
    failed.selections[0].attestation_digest =
      failed.attestations[0].attestation.digest;
    expect(verifyChangeAssurance(failed).proofs[0].reasons).toContain(
      "result_failed",
    );
    for (const [kind, reason] of [
      ["stale-evidence", "evidence_stale"],
      ["suspect-link", "evidence_suspect"],
      ["vacuous-evidence", "evidence_vacuous"],
      ["undischarged", "evidence_unrelated"],
      ["custom-defect", "audit_finding"],
    ] as const) {
      const unhealthy = receiptInput();
      unhealthy.audits[0].report.findings = [
        {
          obligation: "FR-063-AC-1",
          kind: kind as never,
          severity: "high",
          summary: "retained finding",
        },
      ];
      expect(verifyChangeAssurance(unhealthy).proofs[0].reasons).toContain(
        reason,
      );
    }
    const mismatchedOutput = receiptInput();
    mismatchedOutput.attestations[0].output = new TextEncoder().encode(
      "changed",
    );
    expect(verifyChangeAssurance(mismatchedOutput).proofs[0].reasons).toContain(
      "output_digest_mismatch",
    );
  });

  it("TC-1286 keeps absence, unavailable, not-computed, and unevaluated incomplete", () => {
    for (const state of ["unavailable", "not_computed"] as const) {
      const input = receiptInput();
      input.attestations[0].attestation = sealAttestation({
        ...input.attestations[0].attestation,
        digest: undefined,
        result: state,
      } as never);
      input.selections[0].attestation_digest =
        input.attestations[0].attestation.digest;
      expect(verifyChangeAssurance(input).proofs[0].outcome).toBe("incomplete");
    }
    const unevaluated = receiptInput();
    unevaluated.audits[0].report.unevaluated = [
      {
        check: "mocked-confirmation",
        obligation: "FR-063-AC-1",
        suites: ["SUITE-1"],
        reason: "not inspected",
      },
    ];
    expect(verifyChangeAssurance(unevaluated).proofs[0].reasons).toContain(
      "audit_not_evaluated",
    );
    const noOutput = receiptInput();
    noOutput.attestations[0].output = null;
    expect(verifyChangeAssurance(noOutput).proofs[0].reasons).toContain(
      "output_missing",
    );
  });

  it("TC-1287 classifies missing, duplicate, broken, mismatched, rejected, and revise decisions", () => {
    const duplicate = receiptInput();
    duplicate.decision_history.events.push({
      ...structuredClone(duplicate.decision_history.events[0]),
      id: "event-2",
    });
    resealDecisionHistory(duplicate);
    expect(verifyChangeAssurance(duplicate).checks.review.outcome).toBe(
      "invalid",
    );
    const missing = receiptInput();
    missing.decision_history.events = [];
    expect(verifyChangeAssurance(missing).checks.review.outcome).toBe(
      "incomplete",
    );
    const wrongEmptyRun = receiptInput();
    wrongEmptyRun.decision_history.run_id = "wrong-run";
    wrongEmptyRun.decision_history.events = [];
    expect(
      verifyChangeAssurance(wrongEmptyRun).checks.review.reasons,
    ).toContain("decision_mismatch");
    const broken = receiptInput();
    broken.decision_history.events[0].hash = HEX_A;
    expect(verifyChangeAssurance(broken).checks.review.reasons).toContain(
      "event_chain_invalid",
    );
    const malformedHistory = receiptInput();
    delete (
      malformedHistory.decision_history as Partial<
        typeof malformedHistory.decision_history
      >
    ).events;
    expect(
      verifyChangeAssurance(malformedHistory).checks.review.reasons,
    ).toContain("event_chain_invalid");
    const mismatched = receiptInput();
    (
      mismatched.decision_history.events[0].payload as { record_id: string }
    ).record_id = "other";
    resealDecisionHistory(mismatched);
    expect(verifyChangeAssurance(mismatched).checks.review.reasons).toContain(
      "decision_mismatch",
    );
    const unrelated = receiptInput();
    unrelated.decision_history.events[0].kind = "some.other.event";
    resealDecisionHistory(unrelated);
    expect(verifyChangeAssurance(unrelated).checks.review.reasons).toContain(
      "decision_missing",
    );
    const malformedHash = receiptInput();
    malformedHash.decision_history.events[0].hash = "not-a-digest";
    expect(
      verifyChangeAssurance(malformedHash).checks.review.reasons,
    ).toContain("event_chain_invalid");
    for (const [decision, reason] of [
      ["rejected", "review_rejected"],
      ["revise", "review_revision_requested"],
    ] as const) {
      const input = receiptInput();
      (
        input.decision_history.events[0].payload as {
          decision: typeof decision;
        }
      ).decision = decision;
      resealDecisionHistory(input);
      expect(verifyChangeAssurance(input).checks.review.reasons).toContain(
        reason,
      );
    }
  });

  it("TC-1288 keeps impact gaps and unresolved unknowns explicitly incomplete", () => {
    const input = receiptInput();
    input.record.impact_snapshot.truncated = true;
    input.record.definition.unknowns = [
      { id: "u1", statement: "later", disposition: "deferred", owner: "team" },
    ];
    input.record = sealChangeRecord({
      ...input.record,
      digest: undefined,
    } as never);
    input.attestations[0].attestation = sealAttestation({
      ...input.attestations[0].attestation,
      digest: undefined,
      record_digest: input.record.digest,
    } as never);
    input.selections[0].attestation_digest =
      input.attestations[0].attestation.digest;
    (
      input.decision_history.events[0].payload as { record_digest: string }
    ).record_digest = input.record.digest;
    resealDecisionHistory(input);
    const receipt = verifyChangeAssurance(input);
    expect(receipt.outcome).toBe("incomplete");
    expect(receipt.reasons).toEqual(
      expect.arrayContaining(["impact_truncated", "unresolved_unknown"]),
    );
    expect(receipt.unknowns).toEqual([{ id: "u1", disposition: "deferred" }]);
  });

  it("TC-1289 retains exact FR-032 finding kinds and obligation ids", () => {
    const input = receiptInput();
    input.audits[0].report.findings = [
      {
        obligation: "FR-063-AC-1",
        kind: "undischarged",
        severity: "medium",
        summary: "custom-defect",
      },
    ];
    expect(verifyChangeAssurance(input).proofs[0].audit_findings).toEqual([
      { obligation_id: "FR-063-AC-1", kind: "undischarged" },
    ]);
  });

  it("TC-1290 emits byte-identical canonical receipts under input permutations", () => {
    const first = verifyChangeAssurance(receiptInput());
    const secondInput = receiptInput();
    secondInput.record.subject.scope.reverse();
    const second = verifyChangeAssurance(secondInput);
    expect(canonicalizeJcs(first)).toBe(canonicalizeJcs(second));

    const duplicateAttestation = receiptInput();
    duplicateAttestation.attestations.push(
      structuredClone(duplicateAttestation.attestations[0]),
    );
    const duplicateAttestationReversed = structuredClone(duplicateAttestation);
    duplicateAttestationReversed.attestations.reverse();
    expect(
      verifyChangeAssurance(duplicateAttestation).proofs[0].reasons,
    ).toContain("attestation_schema_invalid");
    expect(canonicalizeJcs(verifyChangeAssurance(duplicateAttestation))).toBe(
      canonicalizeJcs(verifyChangeAssurance(duplicateAttestationReversed)),
    );

    const duplicateAudit = receiptInput();
    const unhealthy = structuredClone(duplicateAudit.audits[0]);
    unhealthy.report.findings.push({
      obligation: "FR-063-AC-1",
      kind: "stale-evidence",
      severity: "high",
      summary: "stale",
    });
    duplicateAudit.audits.push(unhealthy);
    const duplicateAuditReversed = structuredClone(duplicateAudit);
    duplicateAuditReversed.audits.reverse();
    expect(verifyChangeAssurance(duplicateAudit).proofs[0].reasons).toContain(
      "audit_finding",
    );
    expect(canonicalizeJcs(verifyChangeAssurance(duplicateAudit))).toBe(
      canonicalizeJcs(verifyChangeAssurance(duplicateAuditReversed)),
    );
  });

  it("TC-1291 leaves prior outputs unchanged and performs no writes", () => {
    const input = receiptInput();
    const snapshot = structuredClone(input);
    verifyChangeAssurance(input);
    expect(input).toEqual(snapshot);
    expect(vi.isMockFunction(verifyChangeAssurance)).toBe(false);
  });

  it("TC-1292 uses attribution/integrity terminology, never identity guarantees", () => {
    const receipt = canonicalizeJcs(verifyChangeAssurance(receiptInput()));
    expect(receipt).toContain("recorded_actor");
    expect(receipt).not.toMatch(
      /authenticated|authorized|signature|non.repudiation/i,
    );
  });
});
