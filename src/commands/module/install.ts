import { Args } from "@oclif/core";

import { QuoinCommand } from "../../base.js";
import { installPlugin } from "../../plugins.js";

export default class ModuleInstall extends QuoinCommand {
  static summary = "Install or update a user/community spec module.";
  static description = `Supported install sources:
  path:<dir>                        Use a local directory containing manifest.yaml
  github:<owner>/<repo>             Clone a GitHub repository (manifest at root)
  github:<owner>/<repo>@<ref>       Pin to a tag, branch, or sha
  github:<owner>/<repo>//<subdir>   Manifest in a monorepo subdirectory (@<ref> ok)
  package:<name>                    Install a module from an npm package (manifest at root)
  package:<name>@<version>          Pin to an npm version`;

  static examples = [
    "quoin module install path:../spec-objects-custom",
    "quoin module install github:agent-ix/spec-objects-custom",
    "quoin module install github:agent-ix/spec-objects-security//spec_objects_security@v0.1.1",
    "quoin module install package:@agent-ix/spec-objects-security@0.4.0",
  ];

  static args = {
    source: Args.string({
      description: "Install source (path:..., github:..., or package:...).",
    }),
  };

  async run(): Promise<void> {
    const { args } = await this.parse(ModuleInstall);
    const source = args.source;
    if (!source) throw new Error("module install requires <source>");
    this.log(JSON.stringify(installPlugin(source), null, 2));
  }
}
