import { Flags } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import { loadMethodCatalog, methodClasses } from "../../advisor/index.js";
import { ensureDefaultModules } from "../../modules.js";

export default class CatalogMethods extends QuoinCommand {
  static summary = "List the merged verification-method catalog.";
  static description = `The 29119-4-shaped answer to "how should this requirement be verified"
(quire-rs FR-054), merged first-wins across the active modules exactly as the
engine merges it — so the advisor recommends from the same catalog the auditor
checks conformance against.

Before this catalog existed the knowledge was prose: IADT lived in two lint
rules, techniques in a traceability vocabulary, and the method table in a skill
markdown file that no manifest declared and no code read.`;

  static examples = [
    "quoin catalog methods",
    "quoin catalog methods --json",
    "quoin catalog methods --class Analysis",
  ];

  static flags = {
    json: Flags.boolean({ description: "Emit the merged catalog as JSON." }),
    class: Flags.string({
      description: "Only methods of this class (e.g. Test, Analysis).",
    }),
  };

  async run(): Promise<void> {
    const { flags } = await this.parse(CatalogMethods);
    ensureDefaultModules();
    const catalog = loadMethodCatalog();

    const methods = flags.class
      ? catalog.methods.filter(
          (m) => m.class.toLowerCase() === flags.class!.toLowerCase(),
        )
      : catalog.methods;

    if (flags.json) {
      this.log(JSON.stringify({ ...catalog, methods }, null, 2));
      return;
    }

    if (methods.length === 0) {
      this.log(
        flags.class
          ? `no methods of class \`${flags.class}\` in the merged catalog`
          : "no active module declares a verification_catalog",
      );
      return;
    }

    for (const method of methods) {
      const kind = method.evidenceKind ? ` → ${method.evidenceKind}` : "";
      this.log(`${method.id}  [${method.class}]${kind}`);
      this.log(`    ${method.name} — ${method.definition.trim()}`);
      for (const [rule, values] of Object.entries(method.applicability)) {
        this.log(`    when ${rule}: ${values.join(", ")}`);
      }
    }
    this.log("");
    this.log(
      `${methods.length} method(s) across ${methodClasses(catalog).join(", ")}`,
    );
    for (const dup of catalog.duplicates) {
      this.log(
        `  duplicate \`${dup.id}\` from ${dup.modules.join(", ")} (first wins)`,
      );
    }
  }
}
