import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

import { Flags } from "@oclif/core";

import { QuoinCommand } from "../base.js";
import { loadMethodCatalog } from "../advisor/index.js";
import { audit } from "../auditor/index.js";
import {
  buildAuthoredArgumentView,
  buildCase,
  renderAuthoredArgument,
  renderCase,
} from "../assurance/index.js";
import type {
  DischargeReport,
  SufficiencyDecision,
} from "../assurance/index.js";
import { readBundleFrontmatter } from "../completeness/index.js";
import {
  latestRun,
  latestScan,
  listRecordedSuites,
  mockInspectionInput,
  readBindings,
} from "../evidence/index.js";
import type { FindingRecord, RunRecord } from "../evidence/index.js";
import {
  checkVersionPremise,
  parseCoverage,
  quireVersion,
  runQuire,
} from "../quire/index.js";

export default class Assurance extends QuoinCommand {
  static summary =
    "Render the assurance case: claim → argument → evidence, from the store.";
  static description = `A pile of green evidence is not an argument (FR-040). This view makes the
reasoning reviewable: this claim holds BECAUSE these sub-claims hold, EACH
verified by a stated method, WITH evidence that is fresh.

Strictly a view. It runs nothing, collects nothing and writes nothing to the
store — where the case cannot be built, the missing data is a finding against
the spec, not something this fabricates.

Gaps render as OPEN nodes (◇), not as omissions. An undischarged obligation, a
suspect binding or a claim nothing traces to stays in the tree, because a case
that quietly narrows to what it can prove reads exactly like a complete one.`;

  static examples = [
    "quoin assurance",
    "quoin assurance --claim-type StR --claim-type hazard",
    "quoin assurance > docs/assurance.md",
  ];

  static flags = {
    repo: Flags.string({ description: "Repository root.", default: "." }),
    module: Flags.string({
      description: "Module directory supplying the traceability model.",
    }),
    "claim-type": Flags.string({
      description:
        "Artifact type that is a top-level claim. Repeatable; matched " +
        "case-insensitively. REPLACES the StR default rather than adding to " +
        "it — pass --claim-type StR as well if StRs should stay claims. A " +
        "safety or security bundle argues from a declared hazard or threat.",
      multiple: true,
    }),
    argument: Flags.string({
      description:
        "Render the authored AssuranceArgument with this frontmatter id. " +
        "Requires --decisions and --as-of; cannot be combined with --claim-type.",
    }),
    decisions: Flags.string({
      description: "JSON array of explicit sufficiency decisions.",
    }),
    discharge: Flags.string({
      description: "Optional clause-discharge-v1 report JSON path.",
    }),
    "as-of": Flags.string({
      description:
        "Explicit ISO-8601 instant for authored argument evaluation.",
    }),
    json: Flags.boolean({ description: "Emit the case as JSON." }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(Assurance);

    if (flags.argument) {
      if (flags["claim-type"]?.length) {
        this.error("--argument cannot be combined with --claim-type", {
          exit: 2,
        });
      }
      if (!flags.decisions || !flags["as-of"]) {
        this.error("--argument requires --decisions and --as-of", { exit: 2 });
      }
      const bundle = readBundleFrontmatter(`${flags.repo}/spec`);
      const matches = bundle.documents.filter(
        (document) => document.frontmatter.id === flags.argument,
      );
      if (matches.length !== 1) {
        this.error(
          matches.length === 0
            ? `AssuranceArgument ${flags.argument} was not found under ${flags.repo}/spec`
            : `AssuranceArgument ${flags.argument} is declared more than once`,
          { exit: 2 },
        );
      }
      let decisions: unknown;
      let discharge: DischargeReport | undefined;
      try {
        decisions = jsonFile(flags.decisions, "sufficiency decisions");
        discharge = flags.discharge
          ? (jsonFile(flags.discharge, "discharge report") as DischargeReport)
          : undefined;
      } catch (error) {
        const detail = error instanceof Error ? error.message : String(error);
        this.error(detail, { exit: 2 });
      }
      if (!Array.isArray(decisions)) {
        this.error("sufficiency decisions must be a JSON array", { exit: 2 });
      }
      try {
        const view = buildAuthoredArgumentView({
          argument: matches[0].frontmatter,
          decisions: decisions as SufficiencyDecision[],
          asOf: flags["as-of"],
          ...(discharge ? { discharge } : {}),
        });
        this.log(
          flags.json
            ? JSON.stringify(view, null, 2)
            : renderAuthoredArgument(view),
        );
        return;
      } catch (error) {
        const detail = error instanceof Error ? error.message : String(error);
        this.error(detail, { exit: 2 });
      }
    }

    const premise = checkVersionPremise(quireVersion());
    if (premise) this.error(premise.message, { exit: 2 });

    const args = ["coverage", "--scope", flags.repo, "--json"];
    if (flags.module) args.push("--module", flags.module);
    const parsed = parseCoverage(runQuire(args));
    if (!parsed.ok) this.error(parsed.error.message, { exit: 2 });
    const obligations = parsed.value.obligations ?? [];

    // The auditor's verdict is what makes an evidence leaf supported or open.
    // Re-deriving "is this fresh" here would be a second answer to a question
    // FR-032 already answers, and the two would disagree the first time either
    // changed.
    const head = headCommit(flags.repo);
    const mockInspections = mockInspectionInput(flags.repo, head);
    const report = audit({
      obligations,
      bindings: readBindings(flags.repo).bindings,
      runs: latestRuns(flags.repo),
      scans: latestScans(flags.repo),
      injections: mockInspections.injections,
      mockInspectionSuites: mockInspections.suites,
      catalog: loadMethodCatalog(flags.module ? [flags.module] : undefined),
    });

    const bundle = readBundleFrontmatter(`${flags.repo}/spec`);
    const assurance = buildCase({
      documents: bundle.documents,
      obligations,
      findings: report.findings,
      claimTypes: flags["claim-type"],
      unreadable: bundle.unreadable,
    });

    this.log(
      flags.json ? JSON.stringify(assurance, null, 2) : renderCase(assurance),
    );
  }
}

function jsonFile(path: string, label: string): unknown {
  try {
    return JSON.parse(readFileSync(path, "utf8")) as unknown;
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    throw new Error(`${label} are not readable JSON: ${detail}`);
  }
}

function headCommit(repo: string): string | undefined {
  try {
    return execFileSync("git", ["-C", repo, "rev-parse", "HEAD"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
  } catch {
    return undefined;
  }
}

function latestScans(repo: string): FindingRecord[] {
  return listRecordedSuites(repo)
    .map((suite) => latestScan(repo, suite))
    .filter((s): s is FindingRecord => s !== null);
}

function latestRuns(repo: string): RunRecord[] {
  return listRecordedSuites(repo)
    .map((suite) => latestRun(repo, suite))
    .filter((r): r is RunRecord => r !== null);
}
