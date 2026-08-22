#!/usr/bin/env node

import { execute } from "@agent-ix/ix-cli-core";

import {
  isVersionRequest,
  packageVersion,
  versionedConfig,
} from "../dist/cli.js";

const argv = process.argv.slice(2);

// Preserve quoin's bare build-time version output ahead of the oclif runner.
if (isVersionRequest(argv)) {
  console.log(packageVersion());
} else {
  // #196: the config carries the build-time version, so the `--help` VERSION
  // block agrees with `--version` instead of printing package.json's CI
  // placeholder.
  await execute({ loadOptions: await versionedConfig(import.meta.url) });
}
