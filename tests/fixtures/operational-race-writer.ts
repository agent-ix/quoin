import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import {
  writeOperationalPair,
  writeOperationalRecord,
} from "../../src/measurement/index.js";

const [repo, mode, capabilityPath, exercisePath] = process.argv.slice(2);
const capability = JSON.parse(readFileSync(capabilityPath, "utf8")) as unknown;
writeFileSync(join(repo, `${mode}.ready`), "ready");

if (mode === "standalone") {
  writeOperationalRecord(repo, capability);
} else if (mode === "pair") {
  const exercise = JSON.parse(readFileSync(exercisePath, "utf8")) as unknown;
  writeOperationalPair(repo, capability, exercise);
} else {
  throw new Error(`unknown operational race-writer mode ${mode}`);
}
