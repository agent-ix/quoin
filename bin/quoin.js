#!/usr/bin/env node

import { Errors } from "@oclif/core";

import { isVersionRequest, main, packageVersion } from "../dist/cli.js";

const argv = process.argv.slice(2);

// Preserve quoin's bare build-time version output ahead of the oclif runner.
if (isVersionRequest(argv)) {
  console.log(packageVersion());
} else {
  // Route through main() rather than oclif's `execute` so the shipped CLI gets
  // the same unknown-command usage the library path does (FR-005-AC-1). main()
  // propagates (FR-026-AC-6), so the error handling `execute` would have done
  // is done here instead.
  try {
    // `import.meta.url` is the config source oclif's own `execute({dir})` used;
    // without it the loader resolves from dist/cli.js and finds no commands.
    await main(argv, import.meta.url);
  } catch (error) {
    Errors.handle(error);
  }
}
