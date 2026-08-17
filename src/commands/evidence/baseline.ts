import { execFileSync } from "node:child_process";

import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import { loadMethodCatalog } from "../../advisor/index.js";
import { audit, findingKey } from "../../auditor/index.js";
import {
  baselinePath,
  latestRun,
  listRecordedSuites,
  readBindings,
  writeBaseline,
} from "../../evidence/index.js";
import type { RunRecord } from "../../evidence/index.js";
import {
  checkVersionPremise,
  parseCoverage,
  quireVersion,
  runQuire,
} from "../../quire/index.js";

export default class EvidenceBaseline extends QuoinCommand {
  static summary = "Accept the current findings as the ratchet baseline.";
  static description = `Writes spec/evidence/baseline.json: every finding the audit reports right now,
as <kind>:<obligation>, plus the commit it was accepted at.

\`--ratchet\` reads this file and fails only on findings NOT in it. Without it,
a first run of the gate reports the entire existing backlog and gets switched
off within a week — so accepting a baseline is the deliberate act that makes
the ratchet usable, and its diff is the record of what was accepted and by whom.

Re-run this at a release to move the ratchet forward. Never run it to make a
new failure go away: the diff is reviewed, and a baseline that grows between
releases is the gate quietly being disabled one entry at a time.`;

  static examples = [
    "quoin evidence baseline",
    "quoin evidence baseline --dry-run",
  ];

  static flags = {
    repo: Flags.string({ description: "Repository root.", default: "." }),
    module: Flags.string({
      description: "Module directory supplying the traceability model.",
    }),
    "dry-run": Flags.boolean({
      description: "Print what would be accepted and write nothing.",
    }),
    json: Flags.boolean({ description: "Emit the outcome as JSON." }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(EvidenceBaseline);

    const premise = checkVersionPremise(quireVersion());
    if (premise) this.error(premise.message, { exit: 2 });

    const args = ["coverage", "--scope", flags.repo, "--json"];
    if (flags.module) args.push("--module", flags.module);
    const parsed = parseCoverage(runQuire(args));
    if (!parsed.ok) this.error(parsed.error.message, { exit: 2 });

    const report = audit({
      obligations: parsed.value.obligations ?? [],
      bindings: readBindings(flags.repo).bindings,
      runs: latestRuns(flags.repo),
      catalog: loadMethodCatalog(flags.module ? [flags.module] : undefined),
      headCommit: headCommit(flags.repo),
    });

    const accepted = report.findings.map(findingKey).sort();
    const commit = headCommit(flags.repo) ?? "";

    if (!flags["dry-run"]) {
      writeBaseline(flags.repo, { schemaVersion: 1, commit, accepted });
    }

    if (flags.json) {
      this.log(
        JSON.stringify(
          { accepted, commit, dryRun: Boolean(flags["dry-run"]) },
          null,
          2,
        ),
      );
      return;
    }

    const byKind = new Map<string, number>();
    for (const key of accepted) {
      const kind = key.slice(0, key.indexOf(":"));
      byKind.set(kind, (byKind.get(kind) ?? 0) + 1);
    }
    this.log(
      `${flags["dry-run"] ? "would accept" : "accepted"} ${accepted.length} ` +
        `finding(s) at ${commit.slice(0, 12) || "an unknown commit"}`,
    );
    for (const [kind, n] of [...byKind].sort()) this.log(`  ${kind}: ${n}`);
    if (!flags["dry-run"]) this.log(baselinePath(flags.repo));
  }
}

function latestRuns(repo: string): RunRecord[] {
  return listRecordedSuites(repo)
    .map((suite) => latestRun(repo, suite))
    .filter((r): r is RunRecord => r !== null);
}

function headCommit(repo: string): string | undefined {
  try {
    return execFileSync("git", ["-C", repo, "rev-parse", "HEAD"], {
      encoding: "utf8",
    }).trim();
  } catch {
    return undefined;
  }
}
