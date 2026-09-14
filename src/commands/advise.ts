import { Flags } from "@oclif/core";

import { QuoinCommand } from "../base.js";
import { advise } from "../core/auditor.js";
import type { Advice, PropertyShape } from "../core/auditor.js";
import { auditInputs } from "../core/evidence.js";
import { loadMethodCatalog } from "../method-catalog.js";
import {
  checkVersionPremise,
  parseCoverage,
  parseProperties,
  quireVersion,
  runQuire,
  runQuireAllowFailure,
} from "../quire/index.js";
import type { PropertiesReport } from "../quire/index.js";

export default class Advise extends QuoinCommand {
  static summary =
    "Recommend a verification method for each obligation, from the catalog.";
  static description = `The deterministic half of the test-plan advisor (FR-031). Rules match or they do
not; where they are inconclusive it says so and stops, rather than guessing.

Before this, the method table was skill-local prose and \`Verification\` columns
defaulted to Test by habit — nothing ever advised DAST for an attack surface,
monitors for a temporal property, or fault injection for a reliability NFR.

Obligations come from quire (coverage --json), and each criterion's FR-052
property shape from quire (properties --json), so the advice is over what the
spec actually states rather than over a document somebody pasted in.

Read the output as a recommendation, never a verdict. An LLM may judge the
residue afterwards — labelled as judgement (the FR-042 / ADR-0010 discipline).`;

  static examples = [
    "quoin advise",
    "quoin advise --mismatch-only",
    "quoin advise --json",
  ];

  static flags = {
    repo: Flags.string({ description: "Repository root.", default: "." }),
    module: Flags.string({
      description: "Module directory supplying the traceability model.",
    }),
    "mismatch-only": Flags.boolean({
      description:
        "Only genuine disagreements: the authored method is a declared " +
        "method or class and is not among the recommendations. Uncatalogued " +
        "values are not mismatches. Combines with --inconclusive-only as a " +
        "union.",
    }),
    "inconclusive-only": Flags.boolean({
      description:
        "Only obligations no rule matched. Combines with --mismatch-only as " +
        "a union.",
    }),
    json: Flags.boolean({ description: "Emit the advice as JSON." }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(Advise);

    const premise = checkVersionPremise(quireVersion());
    if (premise) this.error(premise.message, { exit: 2 });

    const scoped = (verb: string) => {
      const args = [verb, "--scope", flags.repo, "--json"];
      if (flags.module) args.push("--module", flags.module);
      return args;
    };

    const coverage = parseCoverage(runQuire(scoped("coverage")));
    if (!coverage.ok) this.error(coverage.error.message, { exit: 2 });

    const catalog = loadMethodCatalog(
      flags.module ? [flags.module] : undefined,
    );
    if (catalog.methods.length === 0) {
      this.error(
        "no active module declares a `verification_catalog`, so there is " +
          "nothing to advise from. Install one (e.g. spec-artifacts-process) " +
          "or pass --module.",
        { exit: 2 },
      );
    }
    for (const bad of catalog.unreadable) {
      this.warn(`skipped ${bad.moduleRoot}: ${bad.reason}`);
    }

    const shapes = propertyShapes(flags.repo);
    const obligations = coverage.value.obligations ?? [];
    if (obligations.length > 0 && shapes.size === 0) {
      // Said out loud rather than degraded silently. `property_shapes` is the
      // axis most catalog entries are keyed on, so without it `round-trip`
      // never reaches property-based or metamorphic testing and the advice is
      // characteristics-only — which looks like "no rule matched" and is not.
      this.warn(
        "no property shapes were read, so recommendations rest on statement " +
          "text alone. `quire properties` resolves archetypes from the whole " +
          "installed module set; check that the modules declaring FR/NFR/StR " +
          "are installed (quoin module list).",
      );
    }

    // The store is read ONCE, here, and handed in. The advisor performs no
    // I/O — an advisor that could reach the filesystem could also disagree
    // with the auditor about what it found (ADR-0011).
    const store = auditInputs(flags.repo);

    // ONE request for the whole population. The retained command built each
    // obligation's facts itself and called `advise` per obligation, which is
    // fine inside one process and wrong across a boundary: the unit of IPC is
    // a command-shaped operation, so this is one round trip and not N.
    //
    // The uncatalogued-method join (quoin#168) went with it: quire's own
    // diagnosis of which authored values the catalog never declared, keyed by
    // the `value` field byte-equal to `Obligation.method` (quire-rs CR-091).
    const advised = advise({
      catalog,
      obligations,
      shapes: Object.fromEntries(shapes),
      bindings: store.bindings,
      runs: store.runs,
      diagnostics: coverage.value.diagnostics,
    });
    const advice = advised.advice;
    if (advised.degraded) {
      this.warn(
        "engine predates vocabulary classification: this quire's " +
          "`uncatalogued-verification-method` diagnostics carry no `value`, " +
          "so an authored method the catalog never declared cannot be told " +
          "from a genuine disagreement. Every disagreement is reported as a " +
          "mismatch, as before. Update quire-cli to restore the three-state " +
          "split (quire-rs CR-091).",
      );
    }

    // `--*-only` flags combine as a UNION. Intersection made the pair
    // `--mismatch-only --inconclusive-only` a guaranteed zero rows (a mismatch
    // requires recommendations, inconclusive means none), which read as "all
    // clear" over a corpus full of findings.
    const only: Array<(a: Advice) => boolean> = [];
    if (flags["mismatch-only"]) only.push((a) => a.mismatch);
    if (flags["inconclusive-only"]) only.push((a) => a.inconclusive);
    const shown =
      only.length === 0
        ? advice
        : advice.filter((a) => only.some((wanted) => wanted(a)));

    if (flags.json) {
      this.log(JSON.stringify({ advice: shown }, null, 2));
      return;
    }
    this.render(shown, advice);
  }

  private render(shown: Advice[], all: Advice[]): void {
    for (const a of shown) {
      const head = a.recommended
        .slice(0, 3)
        .map((r) => `${r.method} (${r.reasons.map((x) => x.value).join(", ")})`)
        .join("; ");
      this.log(
        `${a.obligation}  authored=${a.authored ?? "—"}  ` +
          (a.inconclusive ? "inconclusive" : `recommend: ${head}`) +
          (a.mismatch ? "  ⚠ mismatch" : "") +
          (a.uncatalogued ? "  ⚠ uncatalogued" : ""),
      );
    }
    this.log("");
    // Tallies count the FULL population, whatever was filtered out: a footer
    // that tallied only shown rows understated the very totals the filter was
    // asked about (`0 of 33 shown — 0 mismatch, 0 inconclusive`).
    const count = (state: (a: Advice) => boolean) => all.filter(state).length;
    this.log(
      `${shown.length} of ${all.length} obligation(s) shown. ` +
        `Of all ${all.length}: ${count((a) => a.mismatch)} mismatch, ` +
        `${count((a) => a.uncatalogued)} uncatalogued, ` +
        `${count((a) => a.inconclusive)} inconclusive. ` +
        `Recommendations, not verdicts: confirm the method in spec review.`,
    );
  }
}

/**
 * The FR-052 property shape and archetype of each criterion, by row id.
 *
 * Read from `properties --json` because the coverage payload carries neither,
 * and `property_shapes` is the axis most catalog entries are keyed on — without
 * it, `round-trip` would never reach property-based or metamorphic testing.
 *
 * **Deliberately scoped, never `--module`.** `--module` selects the *one*
 * module supplying the traceability model, and classification needs the
 * archetype the document's `type:` names — which usually lives in a different
 * module. Passing `--module <process>` here resolves no `FR` archetype at all
 * and yields zero criteria, silently. Scoped discovery loads the whole
 * installed set, which is what classification requires.
 *
 * An NFR measurement row is an obligation and not a criterion, so it has no
 * entry here and is advised from its statement alone. That is a real limit, not
 * a gap to paper over: no classifier ran on it.
 */
function propertyShapes(repo: string): Map<string, PropertyShape> {
  const args = ["properties", "spec/**/*.md", "--scope", repo, "--json"];
  const out = new Map<string, PropertyShape>();
  // `properties` exits 1 when ANY input document fails to resolve — an asset
  // with no `type:`, say — while still writing a complete payload for every
  // document that did. Two untyped files must not cost the whole shape axis.
  const result = runQuireAllowFailure(args);
  if (!result.stdout.trim()) return out;
  const parsed = parseProperties(result.stdout);
  if (!parsed.ok) return out;
  for (const doc of (parsed.value as PropertiesReport).documents) {
    for (const c of doc.criteria) {
      if (c.row_id)
        out.set(c.row_id, { property: c.property, archetype: doc.archetype });
    }
  }
  return out;
}
