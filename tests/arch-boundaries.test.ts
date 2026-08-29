/**
 * FR-036 — quoin's own architectural boundaries, enforced (TC-202, TC-203).
 *
 * This is the dogfood half of FR-036: the boundaries quoin's specs and ADR-0011
 * state, checked mechanically rather than documented and left to rot.
 *
 * **Why these are tests and not audit scripts.** The prior art for this method
 * is `quire-rs/scripts/audits/` — bash, because a Rust repo has no import-graph
 * linter and its checks must survive a broken build. Neither holds here. Written
 * as shell, the executor check below reported five violations on its first run,
 * every one of them `LINE.exec(line)` — `RegExp.exec` read as `child_process.exec`
 * by a regex that could not tell them apart. The compiler API can, so it is used.
 *
 * These run in `make test`, and their JUnit output is what
 * `quoin evidence record --adapter junit` binds to FR-036-AC-7 and FR-036-AC-8.
 */

import { readdirSync, readFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import ts from "typescript";
import { describe, expect, it } from "vitest";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const srcRoot = join(repoRoot, "src");

interface SourceFile {
  /** Path relative to the repo root, for readable failure messages. */
  rel: string;
  ast: ts.SourceFile;
}

function typescriptFilesUnder(root: string): string[] {
  return readdirSync(root, { recursive: true, encoding: "utf8" })
    .filter((entry) => entry.endsWith(".ts"))
    .map((entry) => join(root, entry))
    .sort();
}

const sources: SourceFile[] = typescriptFilesUnder(srcRoot).map((path) => ({
  rel: relative(repoRoot, path),
  ast: ts.createSourceFile(
    path,
    readFileSync(path, "utf8"),
    ts.ScriptTarget.ESNext,
    /* setParentNodes */ true,
  ),
}));

function lineOf(file: SourceFile, node: ts.Node): number {
  return (
    file.ast.getLineAndCharacterOfPosition(node.getStart(file.ast)).line + 1
  );
}

/** Every module specifier the file imports or re-exports from. */
function moduleSpecifiers(
  file: SourceFile,
): Array<{ text: string; line: number }> {
  const out: Array<{ text: string; line: number }> = [];
  for (const statement of file.ast.statements) {
    const specifier =
      (ts.isImportDeclaration(statement) ||
        ts.isExportDeclaration(statement)) &&
      statement.moduleSpecifier;
    if (specifier && ts.isStringLiteral(specifier)) {
      out.push({ text: specifier.text, line: lineOf(file, specifier) });
    }
  }
  return out;
}

describe("quoin's declared boundaries", () => {
  // Trace: FR-036-AC-7
  it("no library module imports from src/commands/", () => {
    // Commands are leaves. The library is what the plugin API, the skills and
    // every consumer import; oclif is a delivery surface layered on top of it.
    // An import in the other direction makes the library unusable without the
    // CLI framework, and does it silently — `tsc` is content with a cycle that
    // only bites at runtime, in whichever host loads the library first.
    const violations = sources
      .filter((file) => !file.rel.startsWith("src/commands/"))
      .flatMap((file) =>
        moduleSpecifiers(file)
          .filter(({ text }) => /(^|\/)commands\//.test(text))
          .map(({ text, line }) => `${file.rel}:${line} imports ${text}`),
      );
    expect(violations).toEqual([]);
  });

  // Trace: FR-036-AC-8
  it("executes only git, quire and ix-flow", () => {
    // ADR-0011 invariant 1 made mechanical: quoin transcribes, the consumer's CI
    // executes. A run record's claim is "this ran in your CI" — true only while
    // quoin is not the one running it. NFR-007 names `quire` and `ix-flow`;
    // `git` is the `rev-parse HEAD` the evidence store is keyed on.
    //
    // Asserted as the full sorted set rather than a subset, so a NEW binary
    // fails by default. A membership test would pass until someone remembered
    // to extend it, which is the failure mode of every denylist.
    const executed = new Set<string>();
    const unreadable: string[] = [];

    for (const file of sources) {
      // Resolve executors by import, not by call name, and follow `as` aliases
      // to the local binding — `import { execFileSync as run }` is still an
      // executor, and matching the exported name would let a rename walk past.
      const executors = new Set<string>();
      for (const statement of file.ast.statements) {
        if (
          !ts.isImportDeclaration(statement) ||
          !ts.isStringLiteral(statement.moduleSpecifier) ||
          statement.moduleSpecifier.text !== "node:child_process"
        ) {
          continue;
        }
        const bindings = statement.importClause?.namedBindings;
        if (bindings && ts.isNamedImports(bindings)) {
          for (const element of bindings.elements) {
            executors.add(element.name.text);
          }
        }
      }
      if (executors.size === 0) continue;

      const visit = (node: ts.Node): void => {
        if (
          ts.isCallExpression(node) &&
          ts.isIdentifier(node.expression) &&
          executors.has(node.expression.text)
        ) {
          const [first] = node.arguments;
          if (first && ts.isStringLiteral(first)) {
            executed.add(first.text);
          } else if (
            file.rel === "src/quire/exec.ts" &&
            first &&
            ts.isCallExpression(first) &&
            ts.isIdentifier(first.expression) &&
            first.expression.text === "quireExecutable" &&
            first.arguments.length === 0
          ) {
            // Quire is resolved once to an absolute, real path and can be
            // digest-locked by QUOIN_EXPECTED_QUIRE_SHA256. Keep this narrow:
            // no other computed executable or call site is admitted.
            executed.add("quire");
          } else {
            // A computed binary is not readable here and therefore must not
            // pass: the check would be asserting over a set it cannot see.
            unreadable.push(
              `${file.rel}:${lineOf(file, node)} ${node.expression.text}() has no string-literal binary`,
            );
          }
        }
        ts.forEachChild(node, visit);
      };
      visit(file.ast);
    }

    expect(unreadable).toEqual([]);
    expect([...executed].sort()).toEqual(["git", "ix-flow", "quire"]);
  });
});
