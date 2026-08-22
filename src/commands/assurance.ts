import { Flags } from "@oclif/core";

import { QuoinCommand } from "../base.js";
import { loadMethodCatalog } from "../advisor/index.js";
import { audit } from "../auditor/index.js";
import { buildCase, renderCase } from "../assurance/index.js";
import { readBundleFrontmatter } from "../completeness/index.js";
import {
  latestRun,
  latestScan,
  listRecordedSuites,
  readIndependencePolicy,
  readBindings,
  readTrustDecisions,
  requireKnownPolicyObligations,
  assessTrust,
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
  static description = `A pile of green evidence is not an argument (FR-040). ISO 15026-2 / GSN turns
the trace graph into something reviewable: this claim holds BECAUSE these
sub-claims hold, EACH verified by a stated method, WITH evidence that is fresh.

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
    "independence-policy": Flags.string({
      description:
        "Normalized JSON projection of exact obligation/dimension requirements " +
        "selected by an AssuranceProfile.",
    }),
    json: Flags.boolean({ description: "Emit the case as JSON." }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(Assurance);

    const premise = checkVersionPremise(quireVersion());
    if (premise) this.error(premise.message, { exit: 2 });

    const args = ["coverage", "--scope", flags.repo, "--json"];
    if (flags.module) args.push("--module", flags.module);
    const parsed = parseCoverage(runQuire(args));
    if (!parsed.ok) this.error(parsed.error.message, { exit: 2 });
    const obligations = parsed.value.obligations ?? [];
    let independencePolicy;
    try {
      independencePolicy = flags["independence-policy"]
        ? readIndependencePolicy(flags["independence-policy"])
        : undefined;
      if (independencePolicy) {
        requireKnownPolicyObligations(
          independencePolicy,
          obligations.map((obligation) => obligation.id),
        );
      }
    } catch (cause) {
      this.error((cause as Error).message, { exit: 2 });
    }

    // The auditor's verdict is what makes an evidence leaf supported or open.
    // Re-deriving "is this fresh" here would be a second answer to a question
    // FR-032 already answers, and the two would disagree the first time either
    // changed.
    const report = audit({
      obligations,
      bindings: readBindings(flags.repo).bindings,
      runs: latestRuns(flags.repo),
      scans: latestScans(flags.repo),
      catalog: loadMethodCatalog(flags.module ? [flags.module] : undefined),
      independencePolicy,
    });

    const bundle = readBundleFrontmatter(`${flags.repo}/spec`);
    const trustUnreadable: string[] = [];
    const producerTrust = readTrustDecisions(flags.repo, trustUnreadable).map(
      assessTrust,
    );
    const assurance = buildCase({
      documents: bundle.documents,
      obligations,
      findings: report.findings,
      claimTypes: flags["claim-type"],
      unreadable: [
        ...bundle.unreadable,
        ...trustUnreadable.map((path) => ({
          path,
          reason: "trust decision is unreadable or invalid",
        })),
      ],
      producerTrust,
      evidenceIndependence: report.independence,
    });

    this.log(
      flags.json ? JSON.stringify(assurance, null, 2) : renderCase(assurance),
    );
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
