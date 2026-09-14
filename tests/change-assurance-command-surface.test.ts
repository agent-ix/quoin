/**
 * The `quoin change-assurance` command surface over the `quoin-core` boundary
 * (FR-068, quoin#457).
 *
 * `src/change-assurance/` is gone: the FR-063 record, FR-064 attestation,
 * retained-output intake and FR-065 receipt contracts are decided by
 * `quoin-change-assurance` and reached through `change_assurance.*`
 * (FR-096). The criteria about what those contracts DECIDE are restated in
 * `rust/crates/quoin-core/tests/tc_457_change_assurance_boundary.rs`, which
 * hands every operation to the real binary by its wire spelling.
 *
 * What is left here is what stays this side of the boundary and nowhere else:
 * the 0/1/2 exit grammar, the packaged schema assets, the canonical goldens a
 * consumer already stored, and the static boundaries over the command sources.
 *
 * Nothing here is conditional. `tests/global-setup.ts` already fails the whole
 * suite when `quoin-core` is unreachable — deliberately, and not as a skip —
 * so a guard here would only be a way for these criteria to stop being checked
 * without anyone noticing.
 */

import { mkdtempSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { loadConfig } from "@agent-ix/ix-cli-core";
import type { Config } from "@oclif/core";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import ChangeAssuranceIntake from "../src/commands/change-assurance/intake.js";
import ChangeAssuranceReceipt from "../src/commands/change-assurance/receipt.js";
import ChangeAssuranceSchema from "../src/commands/change-assurance/schema.js";
import ChangeAssuranceSealAttestation from "../src/commands/change-assurance/seal-attestation.js";
import ChangeAssuranceSealRecord from "../src/commands/change-assurance/seal-record.js";
import ChangeAssuranceVerifyReceipt from "../src/commands/change-assurance/verify-receipt.js";
import { CHANGE_ASSURANCE_SCHEMA_NAMES } from "../src/core/change-assurance-schemas.js";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const commandRoot = join(repoRoot, "src/commands/change-assurance");
const goldenRoot = join(repoRoot, "tests/fixtures/change-assurance-cli");
const assetRoot = join(repoRoot, "rust/crates/quoin-change-assurance/schemas");
const OUTPUT = new TextEncoder().encode("ok\n");
/** The one sentence permitted to name what these commands do NOT establish. */
const DISCLAIMER =
  /Digests establish content integrity and recorded actor labels are attribution only; nothing here establishes authorization or non-repudiation, and no output is a certification\./;

/** The committed inputs the goldens were produced from. */
function golden(name: string): string {
  return join(goldenRoot, name);
}

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

/** Seal and retain the committed record through the command. */
async function sealRecord(): Promise<{ digest: string; path: string }> {
  lines.length = 0;
  await ChangeAssuranceSealRecord.run(
    ["--repo", repo, "--input", golden("record-body.json"), "--json"],
    config,
  );
  const emitted = JSON.parse(lines.join("\n")) as {
    digest: string;
    path: string;
  };
  lines.length = 0;
  return emitted;
}

/** Seal the committed attestation body and retain the pair. */
async function intakeAttestation(): Promise<{ digest: string; text: string }> {
  lines.length = 0;
  const outputPath = writeBytes("output.bin", OUTPUT);
  await ChangeAssuranceSealAttestation.run(
    [
      "--input",
      golden("attestation-body.json"),
      "--output",
      outputPath,
      "--media-type",
      "text/plain",
      "--json",
    ],
    config,
  );
  const text = lines.join("\n");
  const attestation = JSON.parse(text) as { digest: string };
  lines.length = 0;
  const sealedPath = writeBytes(
    "attestation.json",
    new TextEncoder().encode(`${text}\n`),
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
  return { digest: attestation.digest, text };
}

/** The argv of a receipt run over the retained committed evidence. */
function receiptArgv(
  recordDigest: string,
  attestationDigest: string,
): string[] {
  return [
    "--repo",
    repo,
    "--record",
    recordDigest,
    "--candidate-revision",
    "candidate-1",
    "--select",
    `proof-1=${attestationDigest}`,
    "--decisions",
    golden("decisions.json"),
    "--audits",
    golden("audits.json"),
    "--json",
  ];
}

describe("FR-068 exit grammar", () => {
  /**
   * Exit status is 0 for a `valid` receipt, 1 for `invalid` and `incomplete`,
   * and 2 for a usage, parse or integrity error — with the receipt still
   * emitted for the first two and NOT emitted for the third.
   *
   * The 1 and the 2 are the whole point of the split. `quoin-core` refuses an
   * unretained record with exit 2 of its own and refuses a malformed document
   * with 3; both reach the user as this surface's 2, because 1 means "the
   * receipt is not valid" and nothing else.
   *
   * Trace: FR-068-AC-6
   * Provenance: agent-ix/quoin#457
   */
  it("exits 0 for valid, 1 for invalid and incomplete, and 2 for usage errors", async () => {
    const record = await sealRecord();
    const attestation = await intakeAttestation();
    const valid = receiptArgv(record.digest, attestation.digest);

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

    // Nothing selected is incomplete, not a pass: also a 1, with the receipt.
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
          golden("decisions.json"),
          "--json",
        ],
        config,
      ),
      1,
    );
    expect(JSON.parse(lines.join("\n")).outcome).toBe("incomplete");

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
    expect(missing.message).toMatch(/CORE_REFUSED/);
    expect(lines).toEqual([]);

    // A document that is not a receipt at all is also a 2, never a 3 leaking
    // the engine's own taxonomy out of the command.
    lines.length = 0;
    await expectExit(
      ChangeAssuranceVerifyReceipt.run(
        ["--input", writeBytes("not-a-receipt.json", OUTPUT), "--json"],
        config,
      ),
      2,
    );
    expect(lines).toEqual([]);
  });
});

describe("FR-068 compatibility", () => {
  /**
   * The goldens a consumer already stored are reproduced byte-identically by
   * the commands: the retained record file, the emitted attestation, and the
   * emitted receipt.
   *
   * The record is compared as the bytes the store now holds rather than as
   * something this test re-derived, which is the compatibility claim — the
   * file a consumer reads, not a recomputation agreeing with itself.
   *
   * Trace: FR-068-AC-10, FR-068-CON-4
   * Provenance: agent-ix/quoin#457
   */
  it("reproduces the golden record, attestation, and receipt byte-identically", async () => {
    const record = await sealRecord();
    const attestation = await intakeAttestation();
    await ChangeAssuranceReceipt.run(
      receiptArgv(record.digest, attestation.digest),
      config,
    );
    const receipt = lines.join("\n");

    // The goldens are the canonical document plus the newline a text file
    // ends with; the store holds the canonical document exactly, with no
    // terminator — which is what the digest is taken over.
    expect(`${readFileSync(record.path, "utf8")}\n`).toBe(
      readFileSync(golden("sealed-record.json"), "utf8"),
    );
    expect(`${attestation.text}\n`).toBe(
      readFileSync(golden("sealed-attestation.json"), "utf8"),
    );
    expect(`${receipt}\n`).toBe(readFileSync(golden("receipt.json"), "utf8"));
  });
});

describe("FR-068 packaged schemas", () => {
  /**
   * `schema` lists the three normative asset names and emits each
   * byte-identically to the packaged asset; an unknown name is refused with
   * exit 2.
   *
   * The assets are DATA, and since quoin#503 they are data the engine carries:
   * compiled into `quoin-change-assurance` with `include_str!` and emitted
   * verbatim, so what a consumer validates against is the same file the
   * sealing tests run over. Compared here against the crate's own bytes rather
   * than against a copy, which is the drift the move removed.
   *
   * Trace: FR-068-AC-8
   * Provenance: agent-ix/quoin#457, agent-ix/quoin#503
   */
  it("lists and emits the packaged assets and refuses an unknown name", async () => {
    await ChangeAssuranceSchema.run(["--json"], config);
    expect(JSON.parse(lines.join("\n"))).toEqual({
      schemas: [...CHANGE_ASSURANCE_SCHEMA_NAMES],
    });

    for (const name of CHANGE_ASSURANCE_SCHEMA_NAMES) {
      lines.length = 0;
      await ChangeAssuranceSchema.run(["--name", name], config);
      expect(`${lines.join("\n")}\n`).toBe(
        readFileSync(join(assetRoot, name), "utf8"),
      );
    }

    await expectExit(
      ChangeAssuranceSchema.run(["--name", "not-a-schema.json"], config),
      2,
    );
  });
});

describe("FR-068 boundaries", () => {
  /**
   * No command runs the attested command, invokes Git, or reaches the network,
   * and no output or help text makes an identity, authorization,
   * non-repudiation or certification claim.
   *
   * The one process any command starts is the FR-096 engine, and it is started
   * through `src/core/exec.ts` — which resolves a real path, pins the expected
   * digest and applies a declared output ceiling. That is asserted positively
   * as well as negatively: forbidding `child_process` while a command reached
   * the engine some other way would be a scan that passes for the wrong
   * reason. FR-068-CON-1 was amended for this by quoin#457.
   *
   * Trace: FR-068-AC-11, FR-068-CON-1, FR-068-CON-3
   * Provenance: agent-ix/quoin#457
   */
  it("executes nothing but the engine and claims no identity or certification", () => {
    const sources = readdirSync(commandRoot).map((name) => ({
      name,
      text: readFileSync(join(commandRoot, name), "utf8"),
    }));
    expect(sources.length).toBeGreaterThan(6);

    for (const { name, text } of sources) {
      for (const forbidden of [
        "child_process",
        "execSync",
        "execFileSync",
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

    // Exactly one module owns the crossing, and every command that needs the
    // engine goes through it.
    const common = readFileSync(join(commandRoot, "common.ts"), "utf8");
    expect(common).toContain('from "../../core/exec.js"');
    const askers = sources.filter(({ text }) => text.includes("askCore("));
    expect(askers.map(({ name }) => name).sort()).toEqual([
      "common.ts",
      "intake.ts",
      "receipt.ts",
      "recover.ts",
      "seal-attestation.ts",
      "seal-record.ts",
      "verify-receipt.ts",
    ]);

    const topic = readFileSync(join(commandRoot, "index.ts"), "utf8");
    expect(topic.replace(/\s+/g, " ")).toMatch(DISCLAIMER);
  });
});
