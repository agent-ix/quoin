import { QuoinCommand } from "../../base.js";

export default class Evidence extends QuoinCommand {
  static summary =
    "The evidence store — what actually ran, and against which statement.";
  static description = `The artifact of record for verification (FR-030). quoin TRANSCRIBES; the
consumer's CI executes — nothing here runs a test.

Layout, under spec/evidence/:

  suites.md        authored suite registry     (validated corpus document)
  inspections.md   authored inspection acts    (validated corpus document)
  bindings.json    obligation -> hash-at-binding -> evidence
  baseline.json    the accepted violation set the ratchet compares against
  runs/<SUITE-N>/<commit12>.json   one file = one run of one suite
  measurements/<PLAN-ID>/<identity>.json   one policy-free observation

Subcommands:
  quoin evidence record     transcribe a suite run
  quoin evidence measure    transcribe policy-free observations
  quoin evidence affirm     re-affirm a binding after its statement changed
  quoin evidence audit      read the store and report
  quoin evidence baseline   accept the current findings as the ratchet baseline
  quoin evidence gc         drop run records nothing references`;

  static examples = [
    "quoin evidence record --suite SUITE-001 --commit $(git rev-parse HEAD) --tool 'cargo test' --results run.json",
    "quoin evidence measure --help",
    "quoin evidence affirm --obligation FR-001-AC-1 --who @reviewer",
    "quoin evidence audit --ratchet --strict",
    "quoin evidence baseline",
    "quoin evidence gc --dry-run",
  ];

  async run(): Promise<void> {
    this.log(Evidence.description);
  }
}
