import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import {
  QUOIN_ENV_BINDINGS,
  QUOIN_PLUGIN_ID,
  QuoinConfigSchema,
  ixSchema,
} from "../src/config-schema";

// FR-027-AC-6: strictness is what makes `config set` reject an unknown key
// instead of writing it, and registerPluginSchema rejects a non-strict schema
// outright.
describe("quoin config schema (FR-027-AC-6)", () => {
  // Trace: FR-027-AC-6
  test("accepts a non-empty org", () => {
    expect(QuoinConfigSchema.parse({ org: "acme" })).toEqual({ org: "acme" });
  });

  // Trace: FR-027-AC-6
  test("accepts an absent org rather than defaulting one", () => {
    expect(QuoinConfigSchema.parse({})).toEqual({});
  });

  // Trace: FR-027-AC-6
  test("rejects an empty org", () => {
    expect(() => QuoinConfigSchema.parse({ org: "" })).toThrow();
  });

  // Trace: FR-027-AC-6
  test("rejects an unrecognized key", () => {
    expect(() => QuoinConfigSchema.parse({ bogus: "x" })).toThrow();
  });
});

// FR-027-AC-7: the host's init hook looks for this exact named export.
describe("ixSchema convention (FR-027-AC-7)", () => {
  // Trace: FR-027-AC-7
  test("declares the plugin id, schema, and env binding", () => {
    expect(ixSchema.id).toBe(QUOIN_PLUGIN_ID);
    expect(ixSchema.id).toBe("quoin");
    expect(ixSchema.config).toBe(QuoinConfigSchema);
    expect(ixSchema.env).toEqual({ org: "QUOIN_ORG" });
    expect(QUOIN_ENV_BINDINGS.org).toBe("QUOIN_ORG");
  });
});

// FR-027-AC-8: the command group must delegate, not reimplement -- ix-cli-core
// owns the argument shapes and the file locking.
describe("config commands delegate to ix-cli-core (FR-027-AC-8)", () => {
  const dir = join(
    dirname(dirname(fileURLToPath(import.meta.url))),
    "src",
    "commands",
    "config",
  );

  // Trace: FR-027-AC-8
  test.each([
    ["get", "runConfigGet"],
    ["set", "runConfigSet"],
    ["edit", "runConfigEdit"],
    ["doctor", "runConfigDoctor"],
  ])("%s calls %s from the shared package", (file, handler) => {
    const src = readFileSync(join(dir, `${file}.ts`), "utf8");
    expect(src).toContain(`import { ${handler} } from "@agent-ix/ix-cli-core"`);
    expect(src).toContain(`${handler}(`);
  });

  // Trace: FR-027-AC-8
  test("each command registers quoin's schema before delegating", () => {
    // Standalone there is no host init hook, so an unregistered id would make
    // the shared handlers raise UnknownPluginError.
    for (const file of ["get", "set", "edit", "doctor"]) {
      expect(readFileSync(join(dir, `${file}.ts`), "utf8")).toContain(
        "registerQuoinSchema()",
      );
    }
  });
});
