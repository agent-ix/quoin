#!/usr/bin/env node

import { execute } from "@agent-ix/ix-cli-core";

import { isVersionRequest, packageVersion } from "../dist/cli.js";

const argv = process.argv.slice(2);

// Preserve quoin's bare build-time version output ahead of the oclif runner.
if (isVersionRequest(argv)) {
  console.log(packageVersion());
} else {
  await execute({ dir: import.meta.url });
}
