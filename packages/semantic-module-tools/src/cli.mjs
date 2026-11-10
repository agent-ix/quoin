import { readFileSync } from "node:fs";
import { relative, resolve } from "node:path";
import { parseArgs } from "node:util";
import { GenerateError, checkOutput, emit, writeOutput } from "./emit.mjs";

const USAGE = `usage: semantic-module-emit --package-dir <dir> [options]

  --package-dir <dir>            the module's package directory (holds manifest.yaml; schemas/ is written beneath it)
  --repo-root <dir>              repository root holding node_modules and typespec/ (default: cwd)
  --source-dir <dir>             TypeSpec source directory (default: <repo-root>/typespec)
  --manifest <file>              manifest path (default: <package-dir>/manifest.yaml)
  --identity <org/repo>          package identity (default: the name in <repo-root>/package.json without its leading @)
  --semantic-core-version <v>    semantic-core version (default: the installed @agent-ix/semantic-core)
  --check                        write nothing; fail on any difference from the committed output
`;

function readIdentity(repoRoot) {
  const { name } = JSON.parse(readFileSync(resolve(repoRoot, "package.json"), "utf8"));
  if (typeof name !== "string" || !name.startsWith("@") || !name.includes("/")) {
    throw new GenerateError(
      `package.json name ${JSON.stringify(name)} is not @org/repo; pass --identity`,
    );
  }
  return name.slice(1);
}

export function main(argv) {
  let values;
  try {
    ({ values } = parseArgs({
      args: argv,
      options: {
        "package-dir": { type: "string" },
        "repo-root": { type: "string" },
        "source-dir": { type: "string" },
        manifest: { type: "string" },
        identity: { type: "string" },
        "semantic-core-version": { type: "string" },
        check: { type: "boolean", default: false },
      },
      strict: true,
    }));
  } catch (error) {
    console.error(`${error.message}\n${USAGE}`);
    return 2;
  }
  if (!values["package-dir"]) {
    console.error(`--package-dir is required\n${USAGE}`);
    return 2;
  }
  const repoRoot = resolve(values["repo-root"] ?? process.cwd());
  const packageDir = resolve(repoRoot, values["package-dir"]);
  const outputDir = resolve(packageDir, "schemas");
  try {
    const rendered = emit({
      repoRoot,
      sourceDir: resolve(repoRoot, values["source-dir"] ?? "typespec"),
      manifestPath: resolve(repoRoot, values.manifest ?? resolve(packageDir, "manifest.yaml")),
      identity: values.identity ?? readIdentity(repoRoot),
      semanticCoreVersion: values["semantic-core-version"],
    });
    if (values.check) {
      const problems = checkOutput(rendered, outputDir, repoRoot);
      if (problems.length > 0) {
        console.error(
          `emitted schemas differ from the committed output:\n  ${problems.join("\n  ")}\n` +
            "Run `make schemas` and commit the result.",
        );
        return 1;
      }
      console.log(`schemas-check: ${rendered.size} schema(s) match the committed output`);
      return 0;
    }
    writeOutput(rendered, outputDir);
    console.log(`schemas: wrote ${rendered.size} schema(s) to ${relative(repoRoot, outputDir)}`);
    return 0;
  } catch (error) {
    if (error instanceof GenerateError) {
      console.error(error.message);
      return 1;
    }
    throw error;
  }
}
