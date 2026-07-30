import { Args, Flags } from "@oclif/core";

import { QuoinCommand } from "../base.js";
import { loadCatalog } from "../catalog.js";
import { ensureDefaultModules } from "../modules.js";
import {
  createAuthoringPack,
  formatAuthoringPack,
  parseTypeList,
} from "../write.js";

export default class Write extends QuoinCommand {
  static summary =
    "Build an authoring pack for spec files an agent is about to create or edit.";
  static description = `Pass the target repository and the artifact/object types involved in the work.
quoin resolves those types from the active catalog and returns the local
skeletons, schemas, module roots, and Quire validation command for the repo.

Notes:
  - Type lookup is case-insensitive; FR and fr are the same type.
  - Types can be artifacts or objects from the default module set or modules.
  - Use the returned skeletons and schemas as the authoring contract.
  - Run the returned Quire command after editing spec files.`;

  static examples = [
    "quoin write . --types FR",
    "quoin write . --types FR,domain,entity",
    "quoin write ../my-service --types fr,DOMAIN --json",
  ];

  static args = {
    repo_dir: Args.string({ description: "Target repository directory." }),
  };

  static flags = {
    types: Flags.string({
      description: "Artifact/object type(s), comma-separated or repeated.",
      multiple: true,
    }),
    json: Flags.boolean({ description: "Emit the authoring pack as JSON." }),
  };

  async run(): Promise<void> {
    const { args, flags } = await this.parse(Write);
    const repoDir = args.repo_dir;
    if (!repoDir) throw new Error("write requires <repo_dir>");
    ensureDefaultModules();
    const catalog = loadCatalog();
    const pack = createAuthoringPack(
      catalog,
      repoDir,
      parseTypeList(flags.types),
    );
    this.log(
      flags.json ? JSON.stringify(pack, null, 2) : formatAuthoringPack(pack),
    );
  }
}
