import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import {
  STORE_SCHEMA_VERSION,
  inspectMockInjections,
  writeMockInspection,
} from "../../evidence/index.js";

export default class EvidenceInspectMocks extends QuoinCommand {
  static summary =
    "Inspect test source and record explicit mock substitutions.";
  static description = `Scans source for explicit stand-in constructors used by test symbols and records
the observation for one suite at one commit. It runs no test and assigns no
verdict: evidence audit independently decides whether an injected identifier
overlaps the behaviour an obligation claims to verify.

Record an empty inspection too. That is how audit distinguishes "looked and
found no relevant injections" from "nobody looked" (agent-ix/quoin#204).`;

  static examples = [
    "quoin evidence inspect-mocks --suite SUITE-001 --commit $(git rev-parse HEAD)",
  ];

  static flags = {
    suite: Flags.string({
      description: "Suite id whose test source is being inspected.",
      required: true,
    }),
    commit: Flags.string({
      description: "Full commit sha whose source is being inspected.",
      required: true,
    }),
    repo: Flags.string({ description: "Repository root.", default: "." }),
    timestamp: Flags.string({
      description: "ISO-8601 inspection time. Defaults to now.",
    }),
    "dry-run": Flags.boolean({
      description: "Inspect and report without writing an observation record.",
    }),
    json: Flags.boolean({ description: "Emit the record outcome as JSON." }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(EvidenceInspectMocks);
    const injections = inspectMockInjections(flags.repo, flags.suite);
    const path = flags["dry-run"]
      ? null
      : writeMockInspection(flags.repo, {
          schemaVersion: STORE_SCHEMA_VERSION,
          suite: flags.suite,
          commit: flags.commit,
          tool: "quoin mock-inspection",
          timestamp: flags.timestamp ?? new Date().toISOString(),
          injections,
        });

    if (flags.json) {
      this.log(JSON.stringify({ path, injections }, null, 2));
      return;
    }
    this.log(
      `${flags["dry-run"] ? "inspected" : "recorded mock inspection"} ` +
        `${flags.suite} @ ${flags.commit.slice(0, 12)}` +
        (path ? ` -> ${path}` : " (dry run; wrote nothing)"),
    );
    this.log(`  injections: ${injections.length}`);
    for (const injection of injections) {
      this.log(
        `  ${injection.path ?? "(unknown)"}:${injection.line ?? 0} ` +
          `${injection.symbol} injects ${injection.injects.join(", ")}`,
      );
    }
  }
}
