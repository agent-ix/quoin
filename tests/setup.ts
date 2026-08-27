import { settings } from "@oclif/core";

// Production discovers commands from package.json's dist/commands entry. Oclif
// otherwise changes that contract under NODE_ENV=test and attempts to import
// the TypeScript source tree, producing ERR_UNKNOWN_FILE_EXTENSION warnings
// while a green suite is running.
settings.enableAutoTranspile = false;
