import { Flags } from "@oclif/core";

import { QuoinCommand } from "../base.js";
import { assessBundle } from "../completeness/index.js";
import type {
  BundleAssessment,
  CompletenessFinding,
} from "../completeness/index.js";

export default class Completeness extends QuoinCommand {
  static summary =
    "Report which declared vocabulary values no requirement owns, and judge the excuses.";
  static description = `The verdict half of declared-vocabulary coverage (FR-037). quire-rs FR-059
computes which declared values no document claims — a deterministic fact about
the spec. This decides what a gap is worth, and whether an exclusion was earned.

The first vocabulary is ISO 25010: a bundle can be 100% AC-covered and still
have no requirement anywhere for reliability, security or maintainability. The
characteristic list is module data (12 values, not the 9 the ticket assumed), so
nothing here is hardcoded — it is read from the same declaration the engine
reads.

A value may be excused. "This is a CLI that controls no physical process, so it
has no safety characteristic" is an answer, and a check that cannot accept one
forces either a permanent false finding or a requirement fabricated to silence
it. But the engine accepts a bare list: adding one frontmatter line took this
repository from 7 findings to 5 with no reason written anywhere. So an exclusion
must carry a written reason, in the same document, in a table row naming the
value.

Severities are deliberately asymmetric. An unowned characteristic is medium —
an admitted gap a reader can see. An unjustified exclusion is high: an assertion
of completeness with nothing behind it, which also removes the finding that
would have prompted the work.

Advisory by default. --strict makes an admitted gap fail too; it does not invent
one.`;

  static examples = [
    "quoin completeness",
    "quoin completeness --strict",
    "quoin completeness --json",
    "quoin completeness --repo ../other-repo",
  ];

  static flags = {
    repo: Flags.string({ description: "Repository root.", default: "." }),
    bundle: Flags.string({
      description:
        "Bundle root to read. Defaults to <repo>/spec, the directory quire treats as the bundle.",
    }),
    module: Flags.string({
      description: "Module directory supplying the vocabulary declarations.",
    }),
    strict: Flags.boolean({
      description:
        "Exit non-zero when any value is unowned, not only on a high finding.",
    }),
    json: Flags.boolean({ description: "Emit the report as JSON." }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(Completeness);

    const report = assessBundle({
      // `<repo>/spec` and not `<repo>`: CR-045 bounds the document walk to the
      // bundle, and a typed registry at the repository root minted nothing
      // while the identical file under `spec/` was walked and validated.
      bundleRoot: flags.bundle ?? `${flags.repo}/spec`,
      strict: flags.strict,
      moduleRoots: flags.module ? [flags.module] : undefined,
    });

    if (flags.json) {
      this.log(JSON.stringify(report, null, 2));
    } else {
      this.render(report);
    }

    // A high finding fails whatever --strict says: an unjustified exclusion is
    // not an admitted gap, it is a claim with nothing behind it.
    //
    // `UNCHECKED` fails only under --strict. A repository that has not adopted
    // the vocabulary should not have its build broken by installing quoin; one
    // that asked for strict completeness gets told that it cannot be known.
    if (report.verdict === "FAIL") this.exit(1);
    if (report.verdict === "UNCHECKED" && flags.strict) this.exit(1);
  }

  private render(report: BundleAssessment): void {
    if (report.vocabularies.length === 0) {
      // Not a pass. A repository whose modules declare no vocabulary coverage
      // has not been checked, and printing PASS over it is exactly the green
      // matrix over dead links this program exists to stop.
      this.warn(
        "no active module declares `traceability.vocabulary_coverage`, so " +
          "nothing was checked. Install one (e.g. spec-artifacts-iso) or pass " +
          "--module.",
      );
    }

    for (const bad of report.unresolved) {
      this.warn(`vocabulary '${bad.name}' not resolved: ${bad.reason}`);
    }
    for (const bad of report.unreadable) {
      this.warn(`frontmatter unreadable in ${bad.path}: ${bad.reason}`);
    }

    for (const rollup of report.rollups) {
      this.log(
        `${rollup.vocabulary}: ${rollup.owned}/${rollup.declared} owned, ` +
          `${rollup.excused} excused, ${rollup.unowned} unowned`,
      );
    }

    for (const finding of report.findings) {
      this.log(`  ${label(finding)} ${finding.message}`);
    }

    this.log(
      `\n${report.verdict} — ${count(report.findings, "high")} high, ` +
        `${count(report.findings, "medium")} medium (bundle ${report.bundleRoot})`,
    );
  }
}

function label(finding: CompletenessFinding): string {
  const where = finding.document ? ` ${finding.document}:` : "";
  return `[${finding.severity}]${where}`;
}

function count(findings: CompletenessFinding[], severity: string): number {
  return findings.filter((f) => f.severity === severity).length;
}
