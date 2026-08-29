import { readFileSync } from "node:fs";

import { Flags } from "@oclif/core";

import { QuoinCommand } from "../base.js";
import {
  buildDischargeReport,
  renderDischargeReport,
  type DischargeFact,
} from "../assurance/index.js";
import { parseClauseBinding } from "../quire/index.js";

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

    const parsedBinding = parseClauseBinding(read(flags.binding));
    if (!parsedBinding.ok) {
      this.error(parsedBinding.error.message, { exit: 2 });
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
        binding: parsedBinding.value,
        facts: facts as DischargeFact[],
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
