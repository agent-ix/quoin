import { Flags } from "@oclif/core";
import { runSelfUpdate } from "@agent-ix/ix-cli-core";

import { QuoinCommand } from "../base.js";
import { packageVersion } from "../version.js";

// quoin is published to the public npm registry (see package.json
// publishConfig.registry). `update` defaults here so it resolves the package
// from public npm regardless of the ambient npm config; override with
// --registry for local-dev snapshots (e.g. http://npm.ix/).
const DEFAULT_UPDATE_REGISTRY = "https://registry.npmjs.org/";

export default class Update extends QuoinCommand {
  static summary = "Check for and install the latest published quoin.";
  static description = `quoin is distributed on the public npm registry as @agent-ix/quoin. This command
queries the registry for the latest version, compares it to the running version,
and (unless --check) runs npm install -g to upgrade in place.`;

  static examples = [
    "quoin update",
    "quoin update --check",
    "quoin update --registry http://npm.ix/",
  ];

  static flags = {
    check: Flags.boolean({
      description: "Report whether an update is available; do not install.",
      default: false,
    }),
    registry: Flags.string({
      description:
        "Force an npm registry to query/install from. Defaults to the public npm registry (https://registry.npmjs.org/), where @agent-ix/quoin is published. Pass http://npm.ix/ for local dev snapshots.",
    }),
  };

  // `update` itself must never trigger the new-version nudge.
  protected override skipUpdateNudge = true;

  async run(): Promise<void> {
    const { flags } = await this.parse(Update);
    await runSelfUpdate({
      packageName: "@agent-ix/quoin",
      currentVersion: packageVersion(),
      header: "quoin update",
      registry: flags.registry ?? DEFAULT_UPDATE_REGISTRY,
      check: flags.check === true,
    });
  }
}
