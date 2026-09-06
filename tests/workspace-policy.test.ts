import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import {
  withRendered,
  type ModuleKind,
} from "./support/semantic-module-template.js";

// Trace: TC-1597, FR-083-AC-9
test("normal pnpm workspace execution excludes raw generation inputs without bypassing verification", () => {
  execFileSync(process.execPath, ["scripts/workspace-policy-selftest.mjs"], {
    timeout: 120_000,
    stdio: ["ignore", "pipe", "pipe"],
  });
}, 125_000);

// Trace: TC-1597, FR-083-AC-9
test.each<ModuleKind>(["artifact", "object", "mixed"])(
  "the rendered %s consumer still has an actual package manifest",
  (kind) => {
    withRendered({ kind }, ({ root }) => {
      const manifest = JSON.parse(
        readFileSync(join(root, "package.json"), "utf8"),
      ) as { name: string; scripts: Record<string, string> };
      expect(manifest.name).toBeTruthy();
      expect(manifest.scripts).toBeDefined();
    });
  },
);
