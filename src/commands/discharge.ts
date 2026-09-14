import { readFileSync } from "node:fs";

import { Flags } from "@oclif/core";

import { QuoinCommand } from "../base.js";
import {
  buildDischargeReport,
  renderDischargeReport,
} from "../core/assurance.js";
import type { ClauseBindingReport } from "../core/types.js";

export default class Discharge extends QuoinCommand {
  static summary =
    "Partition binding clauses into direct evidence, dispositions, and open work.";
  static description = `Consumes a validated Quire clause-binding report and explicit discharge facts.
Applicability and discharge remain separate: unresolved applicability is shown
outside the binding partition, and no aggregate score is emitted.`;

  static flags = {
    binding: Flags.string({
      description: "Quire clause-binding-v1 JSON path, or - for stdin.",
      required: true,
    }),
    facts: Flags.string({
      description: "JSON array of discharge facts.",
      required: true,
    }),
    "as-of": Flags.string({
      description: "Explicit ISO-8601 evaluation instant.",
      required: true,
    }),
    json: Flags.boolean({ description: "Emit the complete report as JSON." }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(Discharge);
    if (flags.binding === "-" && flags.facts === "-") {
      this.error("--binding and --facts cannot both read stdin", { exit: 2 });
    }

    // The clause-binding report is handed to the boundary as it was read.
    // Validating it here against a vendored copy of quire's schema was the
    // second half of a contract that no longer has two sides: `quoin-core`
    // links the engine that emits this shape, so the reader that accepts it
    // is the one that defined it.
    let binding: unknown;
    try {
      binding = JSON.parse(read(flags.binding)) as unknown;
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      this.error(`clause binding is not JSON: ${detail}`, { exit: 2 });
    }

    let facts: unknown;
    try {
      facts = JSON.parse(read(flags.facts)) as unknown;
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      this.error(`discharge facts are not JSON: ${detail}`, { exit: 2 });
    }
    if (!Array.isArray(facts)) {
      this.error("discharge facts must be a JSON array", { exit: 2 });
    }

    try {
      const report = buildDischargeReport({
        binding: binding as ClauseBindingReport,
        facts,
        asOf: flags["as-of"],
      });
      this.log(
        flags.json
          ? JSON.stringify(report, null, 2)
          : renderDischargeReport(report),
      );
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      this.error(detail, { exit: 2 });
    }
  }
}

function read(path: string): string {
  return path === "-" ? readFileSync(0, "utf8") : readFileSync(path, "utf8");
}
