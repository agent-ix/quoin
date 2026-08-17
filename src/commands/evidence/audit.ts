import { execFileSync } from "node:child_process";

import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import { loadMethodCatalog } from "../../advisor/index.js";
import { audit, ratchet } from "../../auditor/index.js";
import {
  listRecordedSuites,
  listRuns,
  readBaseline,
  readBindings,
  readRun,
} from "../../evidence/index.js";
import type { RunRecord } from "../../evidence/index.js";
import { checkVersionPremise, parseCoverage } from "../../quire/index.js";

export default class EvidenceAudit extends QuoinCommand {
  static summary =
    "Audit the evidence store: suspect links, staleness, vacuity.";
  static description = `Reads the store and reports. RUNS NOTHING (ADR-0011 invariant 1) — the
consumer's CI refreshes evidence, and an auditor that could re-run a suite could
also make a finding disappear by re-running it.

Checks:
  suspect-link      the statement changed after the evidence was bound
  stale-evidence    bound to a missing run, or to a run behind HEAD
  vacuous-evidence  every bound symbol was skipped or absent from the run
  undischarged      no evidence is bound at all
  method-conformance   a non-test method discharged by a test run
  insufficient-multiplicity   criticality demands two independent suites

--ratchet compares against spec/evidence/baseline.json and fails only on NEW
violations. A gate that fails on the whole existing backlog gets disabled within
a week.`;

  static examples = [
    "quoin evidence audit",
    "quoin evidence audit --ratchet --strict",
    "quoin evidence audit --json",
  ];

  static flags = {
    repo: Flags.string({ description: "Repository root.", default: "." }),
    module: Flags.string({
      description: "Module directory supplying the traceability model.",
    }),
    ratchet: Flags.boolean({
      description: "Report only violations absent from the baseline.",
    }),
    strict: Flags.boolean({
      description: "Exit 1 when any reported finding remains.",
    }),
    json: Flags.boolean({ description: "Emit the report as JSON." }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(EvidenceAudit);

    const premise = checkVersionPremise(quireVersion());
    if (premise) this.error(premise.message, { exit: 2 });

    const args = ["coverage", "--scope", flags.repo, "--json"];
    if (flags.module) args.push("--module", flags.module);
    const parsed = parseCoverage(
      execFileSync("quire", args, {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "ignore"],
      }),
    );
    if (!parsed.ok) this.error(parsed.error.message, { exit: 2 });

    const report = audit({
      obligations: parsed.value.obligations ?? [],
      bindings: readBindings(flags.repo).bindings,
      runs: latestRuns(flags.repo),
      catalog: loadMethodCatalog(),
      headCommit: headCommit(flags.repo),
      // P0 is the ecosystem's highest priority; two independent methods is the
      // multiplicity rule criticality buys.
      multiplicityRequires: ["P0"],
    });

    const baseline = flags.ratchet ? readBaseline(flags.repo) : null;
    const reported = baseline ? ratchet(report, baseline) : report.findings;

    if (flags.json) {
      this.log(
        JSON.stringify(
          {
            findings: reported,
            healthy: report.healthy,
            ratchet: flags.ratchet,
          },
          null,
          2,
        ),
      );
    } else if (reported.length === 0) {
      this.log(
        `${report.healthy.length} obligation(s) with healthy evidence; nothing to report`,
      );
    } else {
      for (const finding of reported) {
        this.log(`[${finding.severity}] ${finding.kind}: ${finding.summary}`);
      }
      this.log("");
      this.log(
        `${reported.length} finding(s), ${report.healthy.length} healthy` +
          (flags.ratchet ? " (new violations only)" : ""),
      );
    }

    if (flags.strict && reported.length > 0) {
      this.exit(1);
    }
  }
}

/** The newest recorded run per suite. */
function latestRuns(repo: string): RunRecord[] {
  const out: RunRecord[] = [];
  for (const suite of listRecordedSuites(repo)) {
    const files = listRuns(repo, suite);
    const latest = files.at(-1);
    if (!latest) continue;
    const record = readRun(repo, suite, latest.replace(/\.json$/, ""));
    if (record) out.push(record);
  }
  return out;
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

function quireVersion(): string | null {
  try {
    return execFileSync("quire", ["--version"], { encoding: "utf8" });
  } catch {
    return null;
  }
}
