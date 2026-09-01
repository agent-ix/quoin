/**
 * Quoin #322 — producer-facing change-assurance CLI surface (FR-068).
 */

import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { loadConfig } from "@agent-ix/ix-cli-core";
import type { Config } from "@oclif/core";
import fc from "fast-check";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import ChangeAssuranceIntake from "../src/commands/change-assurance/intake.js";
import ChangeAssuranceReceipt from "../src/commands/change-assurance/receipt.js";
import ChangeAssuranceRecover from "../src/commands/change-assurance/recover.js";
import ChangeAssuranceSchema from "../src/commands/change-assurance/schema.js";
import ChangeAssuranceSealAttestation from "../src/commands/change-assurance/seal-attestation.js";
import ChangeAssuranceSealRecord from "../src/commands/change-assurance/seal-record.js";
import ChangeAssuranceVerifyReceipt from "../src/commands/change-assurance/verify-receipt.js";
import {
  attestationPath,
  blake3Hex,
  canonicalizeJcs,
  changeAssuranceSchemaPath,
  hashIxFlowEvent,
  sealAttestation,
  sealChangeRecord,
  CHANGE_ASSURANCE_SCHEMA_NAMES,
  type ChangeAssuranceRecord,
  type ProofAttestation,
} from "../src/change-assurance/index.js";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const commandRoot = join(repoRoot, "src/commands/change-assurance");
const goldenRoot = join(repoRoot, "tests/fixtures/change-assurance-cli");
const HEX_A = "a".repeat(64);
const HEX_B = "b".repeat(64);
const OUTPUT = new TextEncoder().encode("ok\n");
/** The one sentence permitted to name what these commands do NOT establish. */
const DISCLAIMER =
  /Digests establish content integrity and recorded actor labels are attribution only; nothing here establishes authorization or non-repudiation, and no output is a certification\./;

let config: Config;
let repo: string;
let lines: string[];

beforeAll(async () => {
  config = await loadConfig({ root: repoRoot });
});

beforeEach(() => {
  repo = mkdtempSync(join(tmpdir(), "quoin-change-assurance-cli-"));
  lines = [];
  vi.spyOn(console, "log").mockImplementation((line) =>
    lines.push(String(line)),
  );
});

function recordBody(revision = 1): Omit<ChangeAssuranceRecord, "digest"> {
  return {
    schema_version: 1,
    record_type: "change_assurance",
    record_id: "change-1",
    revision,
    parent_digest: revision === 1 ? null : HEX_A,
    subject: {
      repository: "agent-ix/quoin",
      base_revision: "abc123",
      scope: ["src/change-assurance"],
    },
    source_connections: [
      {
        source_id: "FR-068",
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
        { id: "FR-068", statement: "expose it", source_ids: ["FR-068"] },
      ],
      preservation_constraints: [],
      proof_obligations: [
        {
          proof_id: "proof-1",
          statement: "tests pass",
          obligation_ids: ["FR-068-AC-1"],
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

function attestationBody(
  recordDigest: string,
  result: ProofAttestation["result"] = "passed",
): Omit<ProofAttestation, "digest" | "retained_output"> {
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
    result,
  };
}

function decisionsFile(recordDigest: string): string {
  const unsigned = {
    id: "event-1",
    ts: "2026-08-31T00:00:00Z",
    actor: { kind: "human" as const, id: "reviewer" },
    kind: "change_assurance.review_decided",
    payload: {
      schema_version: 1,
      record_id: "change-1",
      revision: 1,
      record_digest: recordDigest,
      decision: "approved",
    },
    prevHash: "0".repeat(64),
  };
  const path = join(repo, "decisions.json");
  writeFileSync(
    path,
    JSON.stringify({
      run_id: "run-1",
      events: [{ ...unsigned, hash: hashIxFlowEvent(unsigned) }],
    }),
  );
  return path;
}

function auditsFile(): string {
  const path = join(repo, "audits.json");
  writeFileSync(
    path,
    JSON.stringify([
      {
        proof_id: "proof-1",
        report_digest: HEX_A,
        report: { findings: [], healthy: ["FR-068-AC-1"], unevaluated: [] },
      },
    ]),
  );
  return path;
}

function write(name: string, value: unknown): string {
  const path = join(repo, name);
  writeFileSync(path, JSON.stringify(value));
  return path;
}

function writeBytes(name: string, bytes: Uint8Array): string {
  const path = join(repo, name);
  writeFileSync(path, bytes);
  return path;
}

/** Assert a command exited with the given oclif status. */
async function expectExit(
  invocation: Promise<unknown>,
  code: number,
): Promise<Error & { oclif?: { exit?: number } }> {
  let thrown: (Error & { oclif?: { exit?: number } }) | null = null;
  try {
    await invocation;
  } catch (error) {
    thrown = error as Error & { oclif?: { exit?: number } };
  }
  expect(thrown, "command was expected to exit non-zero").not.toBeNull();
  expect(thrown?.oclif?.exit).toBe(code);
  return thrown as Error & { oclif?: { exit?: number } };
}

/** Seal and retain one record through the command, returning its digest. */
async function sealRecordThroughCli(
  revision = 1,
): Promise<{ digest: string; path: string }> {
  lines.length = 0;
  const input = write("record.json", recordBody(revision));
  await ChangeAssuranceSealRecord.run(
    ["--repo", repo, "--input", input, "--json"],
    config,
  );
  const emitted = JSON.parse(lines.join("\n")) as {
    digest: string;
    path: string;
  };
  lines.length = 0;
  return emitted;
}

/** Seal an attestation through the command and retain it through intake. */
async function intakeAttestationThroughCli(
  recordDigest: string,
  result: ProofAttestation["result"] = "passed",
  output: Uint8Array = OUTPUT,
): Promise<ProofAttestation> {
  lines.length = 0;
  const bodyPath = write(
    "attestation-body.json",
    attestationBody(recordDigest, result),
  );
  const outputPath = writeBytes("output.bin", output);
  await ChangeAssuranceSealAttestation.run(
    [
      "--input",
      bodyPath,
      "--output",
      outputPath,
      "--media-type",
      "text/plain",
      "--json",
    ],
    config,
  );
  const attestation = JSON.parse(lines.join("\n")) as ProofAttestation;
  lines.length = 0;
  const sealedPath = writeBytes(
    "attestation.json",
    new TextEncoder().encode(canonicalizeJcs(attestation)),
  );
  await ChangeAssuranceIntake.run(
    [
      "--repo",
      repo,
      "--attestation",
      sealedPath,
      "--output",
      outputPath,
      "--json",
    ],
    config,
  );
  lines.length = 0;
  return attestation;
}

describe("FR-068 sealing commands", () => {
  it("TC-1317 seals and retains an explicit record and refuses a supplied digest", async () => {
    const sealed = await sealRecordThroughCli();
    expect(sealed.digest).toBe(sealChangeRecord(recordBody()).digest);
    expect(existsSync(sealed.path)).toBe(true);
    expect(JSON.parse(readFileSync(sealed.path, "utf8"))).toMatchObject({
      record_id: "change-1",
      digest: sealed.digest,
    });

    const withDigest = write("with-digest.json", {
      ...recordBody(),
      digest: HEX_A,
    });
    const error = await expectExit(
      ChangeAssuranceSealRecord.run(
        ["--repo", repo, "--input", withDigest, "--json"],
        config,
      ),
      2,
    );
    expect(error.message).toMatch(/must not supply digest/);

    const undeclared = write("undeclared.json", {
      ...recordBody(),
      extra: true,
    });
    await expectExit(
      ChangeAssuranceSealRecord.run(
        ["--repo", repo, "--input", undeclared, "--json"],
        config,
      ),
      2,
    );
    // The refusals retain nothing: only the one accepted record is stored.
    expect(readdirSync(dirname(sealed.path))).toEqual([
      `${sealed.digest}.json`,
    ]);
  });

  it("TC-1318 derives only the retained-output binding when sealing an attestation", async () => {
    const record = await sealRecordThroughCli();
    const bodyPath = write("body.json", attestationBody(record.digest));
    const outputPath = writeBytes("output.bin", OUTPUT);

    await ChangeAssuranceSealAttestation.run(
      [
        "--input",
        bodyPath,
        "--output",
        outputPath,
        "--media-type",
        "text/plain",
        "--json",
      ],
      config,
    );
    const attestation = JSON.parse(lines.join("\n")) as ProofAttestation;

    // Only the three retained-output fields are the command's; every other
    // field is the caller's, byte for byte.
    expect(attestation.retained_output).toEqual({
      media_type: "text/plain",
      digest: blake3Hex(OUTPUT),
      size_bytes: OUTPUT.byteLength,
    });
    const body = attestationBody(record.digest);
    for (const [key, value] of Object.entries(body)) {
      expect(attestation[key as keyof ProofAttestation]).toEqual(value);
    }
    expect(attestation.result).toBe("passed");

    const supplied = write("supplied.json", {
      ...body,
      retained_output: {
        media_type: "text/plain",
        digest: HEX_A,
        size_bytes: 1,
      },
    });
    const error = await expectExit(
      ChangeAssuranceSealAttestation.run(
        [
          "--input",
          supplied,
          "--output",
          outputPath,
          "--media-type",
          "text/plain",
        ],
        config,
      ),
      2,
    );
    expect(error.message).toMatch(/must not supply .*retained_output/);
  });
});

describe("FR-068 intake", () => {
  it("TC-1319 retains exact bytes, is idempotent, and refuses contradictions", async () => {
    const record = await sealRecordThroughCli();
    const attestation = await intakeAttestationThroughCli(record.digest);
    const directory = attestationPath(repo, attestation.digest);
    expect(readFileSync(join(directory, "output.bin"))).toEqual(
      Buffer.from(OUTPUT),
    );
    expect(
      JSON.parse(readFileSync(join(directory, "attestation.json"), "utf8")),
    ).toEqual(attestation);

    // Byte-identical re-intake succeeds and retains nothing new.
    const before = readdirSync(dirname(directory));
    const sealedPath = writeBytes(
      "attestation.json",
      new TextEncoder().encode(canonicalizeJcs(attestation)),
    );
    const outputPath = writeBytes("output.bin", OUTPUT);
    await ChangeAssuranceIntake.run(
      [
        "--repo",
        repo,
        "--attestation",
        sealedPath,
        "--output",
        outputPath,
        "--json",
      ],
      config,
    );
    expect(readdirSync(dirname(directory))).toEqual(before);

    // Output bytes that contradict the sealed digest are refused.
    const otherOutput = writeBytes(
      "other.bin",
      new TextEncoder().encode("different\n"),
    );
    const error = await expectExit(
      ChangeAssuranceIntake.run(
        [
          "--repo",
          repo,
          "--attestation",
          sealedPath,
          "--output",
          otherOutput,
          "--json",
        ],
        config,
      ),
      2,
    );
    expect(error.message).toMatch(/digest or size mismatch/);
    expect(readdirSync(dirname(directory))).toEqual(before);
  });

  it("TC-1325 recovers only interrupted staging and leaves retained pairs alone", async () => {
    const record = await sealRecordThroughCli();
    const attestation = await intakeAttestationThroughCli(record.digest);
    const parent = dirname(attestationPath(repo, attestation.digest));
    mkdirSync(join(parent, ".tmp-attestation-abc"), { recursive: true });
    writeFileSync(join(parent, ".tmp-attestation-abc", "output.bin"), "half");

    await ChangeAssuranceRecover.run(["--repo", repo, "--json"], config);
    expect(JSON.parse(lines.join("\n"))).toEqual({ removed: 1 });
    expect(readdirSync(parent)).toEqual([attestation.digest]);
    expect(
      readFileSync(join(parent, attestation.digest, "output.bin")),
    ).toEqual(Buffer.from(OUTPUT));

    lines.length = 0;
    await ChangeAssuranceRecover.run(["--repo", repo, "--json"], config);
    expect(JSON.parse(lines.join("\n"))).toEqual({ removed: 0 });
  });
});

describe("FR-068 receipt", () => {
  it("TC-1320 builds the verification input from named inputs only", async () => {
    const record = await sealRecordThroughCli();
    const attestation = await intakeAttestationThroughCli(record.digest);
    const decisions = decisionsFile(record.digest);
    const audits = auditsFile();

    await ChangeAssuranceReceipt.run(
      [
        "--repo",
        repo,
        "--record",
        record.digest,
        "--candidate-revision",
        "candidate-1",
        "--select",
        `proof-1=${attestation.digest}`,
        "--decisions",
        decisions,
        "--audits",
        audits,
        "--json",
      ],
      config,
    );
    const receipt = JSON.parse(lines.join("\n"));
    expect(receipt.outcome).toBe("valid");
    expect(receipt.proofs).toHaveLength(1);
    expect(receipt.proofs[0]).toMatchObject({
      proof_id: "proof-1",
      attestation_digest: attestation.digest,
      outcome: "valid",
    });

    // A second retained attestation that is not selected changes nothing.
    const unrelated = sealAttestation({
      ...attestationBody(record.digest),
      attestation_id: "att-2",
      retained_output: {
        media_type: "text/plain",
        digest: blake3Hex(OUTPUT),
        size_bytes: OUTPUT.byteLength,
      },
    });
    const unrelatedPath = writeBytes(
      "unrelated.json",
      new TextEncoder().encode(canonicalizeJcs(unrelated)),
    );
    const outputPath = writeBytes("output.bin", OUTPUT);
    await ChangeAssuranceIntake.run(
      [
        "--repo",
        repo,
        "--attestation",
        unrelatedPath,
        "--output",
        outputPath,
        "--json",
      ],
      config,
    );
    lines.length = 0;
    await ChangeAssuranceReceipt.run(
      [
        "--repo",
        repo,
        "--record",
        record.digest,
        "--candidate-revision",
        "candidate-1",
        "--select",
        `proof-1=${attestation.digest}`,
        "--decisions",
        decisions,
        "--audits",
        audits,
        "--json",
      ],
      config,
    );
    expect(JSON.parse(lines.join("\n"))).toEqual(receipt);

    // A selection naming no retained attestation is a usage error, not a gap
    // the command quietly verifies around.
    lines.length = 0;
    const error = await expectExit(
      ChangeAssuranceReceipt.run(
        [
          "--repo",
          repo,
          "--record",
          record.digest,
          "--candidate-revision",
          "candidate-1",
          "--select",
          `proof-1=${"c".repeat(64)}`,
          "--decisions",
          decisions,
          "--json",
        ],
        config,
      ),
      2,
    );
    expect(error.message).toMatch(/names no retained attestation/);
  });

  it("TC-1321 never converts unavailable, not-computed, or missing evidence into a pass", async () => {
    const record = await sealRecordThroughCli();
    const decisions = decisionsFile(record.digest);
    const audits = auditsFile();

    await fc.assert(
      fc.asyncProperty(
        fc.constantFrom<ProofAttestation["result"]>(
          "failed",
          "unavailable",
          "not_computed",
        ),
        async (result) => {
          const attestation = await intakeAttestationThroughCli(
            record.digest,
            result,
          );
          lines.length = 0;
          await expectExit(
            ChangeAssuranceReceipt.run(
              [
                "--repo",
                repo,
                "--record",
                record.digest,
                "--candidate-revision",
                "candidate-1",
                "--select",
                `proof-1=${attestation.digest}`,
                "--decisions",
                decisions,
                "--audits",
                audits,
                "--json",
              ],
              config,
            ),
            1,
          );
          const receipt = JSON.parse(lines.join("\n"));
          const expected = {
            failed: "result_failed",
            unavailable: "result_unavailable",
            not_computed: "result_not_computed",
          }[result];
          expect(receipt.proofs[0].reasons).toContain(expected);
          expect(receipt.proofs[0].outcome).not.toBe("valid");
          expect(receipt.outcome).not.toBe("valid");
        },
      ),
      { numRuns: 12 },
    );

    // Missing evidence stays missing: no selection at all is incomplete, not a
    // pass and not a failure of the command.
    lines.length = 0;
    await expectExit(
      ChangeAssuranceReceipt.run(
        [
          "--repo",
          repo,
          "--record",
          record.digest,
          "--candidate-revision",
          "candidate-1",
          "--decisions",
          decisions,
          "--audits",
          audits,
          "--json",
        ],
        config,
      ),
      1,
    );
    const receipt = JSON.parse(lines.join("\n"));
    expect(receipt.proofs[0].reasons).toContain("attestation_missing");
    expect(receipt.proofs[0].attestation_digest).toBeNull();
    expect(receipt.outcome).toBe("incomplete");
  });

  it("TC-1322 exits 0 for valid, 1 for invalid and incomplete, and 2 for usage errors", async () => {
    const record = await sealRecordThroughCli();
    const attestation = await intakeAttestationThroughCli(record.digest);
    const decisions = decisionsFile(record.digest);
    const audits = auditsFile();

    const valid = [
      "--repo",
      repo,
      "--record",
      record.digest,
      "--candidate-revision",
      "candidate-1",
      "--select",
      `proof-1=${attestation.digest}`,
      "--decisions",
      decisions,
      "--audits",
      audits,
      "--json",
    ];
    await ChangeAssuranceReceipt.run(valid, config);
    expect(JSON.parse(lines.join("\n")).outcome).toBe("valid");

    // A candidate revision the attestation is not bound to is invalid, and the
    // receipt is still emitted.
    lines.length = 0;
    await expectExit(
      ChangeAssuranceReceipt.run(
        valid.map((value) =>
          value === "candidate-1" ? "candidate-elsewhere" : value,
        ),
        config,
      ),
      1,
    );
    const invalid = JSON.parse(lines.join("\n"));
    expect(invalid.outcome).toBe("invalid");
    expect(invalid.proofs[0].reasons).toContain("candidate_revision_mismatch");

    // A well-formed digest naming no retained record is a usage error, not an
    // empty verification: exit 2, and no receipt is emitted to be mistaken for
    // a result.
    lines.length = 0;
    const missing = await expectExit(
      ChangeAssuranceReceipt.run(
        [...valid.slice(0, 2), "--record", "c".repeat(64), ...valid.slice(4)],
        config,
      ),
      2,
    );
    expect(missing.message).toMatch(/names no retained record/);
    expect(lines).toEqual([]);
  });

  it("TC-1323 re-verifies a sealed receipt and refuses an altered one", async () => {
    const record = await sealRecordThroughCli();
    const attestation = await intakeAttestationThroughCli(record.digest);
    const decisions = decisionsFile(record.digest);
    const audits = auditsFile();
    await ChangeAssuranceReceipt.run(
      [
        "--repo",
        repo,
        "--record",
        record.digest,
        "--candidate-revision",
        "candidate-1",
        "--select",
        `proof-1=${attestation.digest}`,
        "--decisions",
        decisions,
        "--audits",
        audits,
        "--json",
      ],
      config,
    );
    const receipt = JSON.parse(lines.join("\n"));
    lines.length = 0;

    const receiptPath = write("receipt.json", receipt);
    await ChangeAssuranceVerifyReceipt.run(
      ["--input", receiptPath, "--json"],
      config,
    );
    expect(JSON.parse(lines.join("\n"))).toMatchObject({
      digest: receipt.digest,
      outcome: "valid",
    });

    lines.length = 0;
    const altered = write("altered.json", {
      ...receipt,
      outcome: "valid",
      candidate_revision: "candidate-elsewhere",
    });
    const error = await expectExit(
      ChangeAssuranceVerifyReceipt.run(["--input", altered, "--json"], config),
      2,
    );
    expect(error.message).toMatch(/receipt refused/);
  });
});

describe("FR-068 packaged schemas", () => {
  it("TC-1324 lists and emits the packaged assets and refuses an unknown name", async () => {
    await ChangeAssuranceSchema.run(["--json"], config);
    expect(JSON.parse(lines.join("\n"))).toEqual({
      schemas: [...CHANGE_ASSURANCE_SCHEMA_NAMES],
    });

    for (const name of CHANGE_ASSURANCE_SCHEMA_NAMES) {
      lines.length = 0;
      await ChangeAssuranceSchema.run(["--name", name], config);
      expect(`${lines.join("\n")}\n`).toBe(
        readFileSync(changeAssuranceSchemaPath(name), "utf8"),
      );
    }

    await expectExit(
      ChangeAssuranceSchema.run(["--name", "not-a-schema.json"], config),
      2,
    );
  });
});

describe("FR-068 compatibility and boundaries", () => {
  it("TC-1326 reproduces the golden record, attestation, and receipt byte-identically", async () => {
    const record = await sealRecordThroughCli();
    const attestation = await intakeAttestationThroughCli(record.digest);
    const decisions = decisionsFile(record.digest);
    const audits = auditsFile();
    await ChangeAssuranceReceipt.run(
      [
        "--repo",
        repo,
        "--record",
        record.digest,
        "--candidate-revision",
        "candidate-1",
        "--select",
        `proof-1=${attestation.digest}`,
        "--decisions",
        decisions,
        "--audits",
        audits,
        "--json",
      ],
      config,
    );
    const receipt = lines.join("\n");

    // The goldens were produced by this surface once and committed. They fail
    // if a later change alters the canonical bytes a consumer already stored,
    // which is the compatibility claim — not a re-assertion of this run.
    expect(readFileSync(join(goldenRoot, "sealed-record.json"), "utf8")).toBe(
      `${canonicalizeJcs(sealChangeRecord(recordBody()))}\n`,
    );
    expect(
      readFileSync(join(goldenRoot, "sealed-attestation.json"), "utf8"),
    ).toBe(`${canonicalizeJcs(attestation)}\n`);
    expect(readFileSync(join(goldenRoot, "receipt.json"), "utf8")).toBe(
      `${receipt}\n`,
    );
  });

  it("TC-1327 executes nothing and claims no identity, authorization, or certification", () => {
    const sources = readdirSync(commandRoot).map((name) => ({
      name,
      text: readFileSync(join(commandRoot, name), "utf8"),
    }));
    expect(sources.length).toBeGreaterThan(6);

    for (const { name, text } of sources) {
      for (const forbidden of [
        "child_process",
        "execSync",
        "spawnSync",
        "spawn(",
        "fetch(",
        "simple-git",
        "https://",
      ]) {
        expect(text, `${name} must not reach for ${forbidden}`).not.toContain(
          forbidden,
        );
      }
      // The exact disclaimer is the ONLY permitted mention of these words.
      // Removing it first means any other occurrence is a claim, however it is
      // phrased, rather than a line the filter happened to spare.
      const claims = text.replace(/\s+/g, " ").replace(DISCLAIMER, "");
      for (const claim of [
        /\bauthenticat/i,
        /\bauthoriz/i,
        /\bcertif/i,
        /\bsignature\b/i,
        /\bnon-repudiation\b/i,
      ]) {
        expect(claim.test(claims), `${name} must make no ${claim} claim`).toBe(
          false,
        );
      }
    }

    const topic = readFileSync(join(commandRoot, "index.ts"), "utf8");
    expect(topic.replace(/\s+/g, " ")).toMatch(DISCLAIMER);
  });
});
