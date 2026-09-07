/**
 * NFR-017 non-disruption gates for the semantic module contract (issue #293,
 * TASK-043). TC-1379 (every default module loads) and TC-1380 (warning-only
 * sweep) live with the code they gate; TC-1381 and TC-1382 are the change-set
 * and schema-shape gates.
 */

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");

function changedPaths(): string[] {
  const committed = execFileSync(
    "git",
    ["diff", "--name-only", "origin/main...HEAD"],
    { cwd: repoRoot, encoding: "utf8" },
  );
  const working = execFileSync(
    "git",
    ["status", "--porcelain", "--untracked-files=all"],
    { cwd: repoRoot, encoding: "utf8" },
  )
    .split("\n")
    .filter((line) => line.trim().length > 0)
    .map((line) => line.slice(3).trim());
  return [...new Set([...committed.split("\n"), ...working])].filter(
    (path) => path.length > 0,
  );
}

describe("NFR-017 non-disruptive manifest evolution", () => {
  // Trace: NFR-017-AC-3
  // Trace: TC-1381
  it("writes nothing into a corpus repository or the retained corpus mirror", () => {
    const paths = changedPaths();
    for (const path of paths) {
      expect(path.startsWith("corpus/"), path).toBe(false);
      expect(path.startsWith(".."), path).toBe(false);
      expect(path.startsWith("/"), path).toBe(false);
    }
    // The pinned config-service copy is the only corpus-derived file, and it
    // lives under tests/fixtures with provenance, never in the source repo.
    const corpusCopies = paths.filter((path) =>
      path.includes("config-service"),
    );
    for (const path of corpusCopies) {
      expect(
        path.startsWith("tests/fixtures/semantic-module/corpus/"),
        path,
      ).toBe(true);
    }
  });

  // Trace: NFR-017-AC-4
  // Trace: TC-1382
  it("leaves every pre-existing manifest schema `required` array unchanged", () => {
    // NFR-017-AC-4 is "the manifest schema's `required` arrays are unchanged",
    // and its Verification is "diff the manifest schema `required` arrays".
    //
    // This row used to assert that `package.json` and `pnpm-lock.yaml` were
    // absent from the branch change set. That is a different claim, traced to
    // this id, and it verified no acceptance criterion NFR-017 states: no AC
    // here mentions packaging. Worse, `changedPaths()` diffs `origin/main...HEAD`,
    // so as written it forbade every future dependency change anywhere in the
    // repository, forever. It blocked bumping the pinned quire CLI to the
    // release whose contract this repository vendors (TC-118), which is how it
    // was found.
    //
    // Non-disruption is about existing manifests still loading. A `required`
    // array that already existed must not gain a member -- that would reject a
    // manifest that used to be valid. A `required` array introduced under a
    // newly added optional subtree cannot reject anything, because nothing
    // reaches it without opting in; that is exactly how the `semantic` block
    // was added. So pre-existing arrays are compared exactly, and new ones are
    // reported rather than forbidden.
    const before = JSON.parse(
      readFileSync(
        join(
          repoRoot,
          "tests/fixtures/semantic-module/vendored/module-manifest.schema.pre-cr003.json",
        ),
        "utf8",
      ),
    ) as unknown;
    const after = JSON.parse(
      readFileSync(
        join(repoRoot, "src/semantic/schemas/module-manifest.schema.json"),
        "utf8",
      ),
    ) as unknown;

    const collect = (node: unknown, path: string): Map<string, string[]> => {
      const found = new Map<string, string[]>();
      if (Array.isArray(node)) {
        node.forEach((child, index) => {
          for (const [k, v] of collect(child, `${path}/${index}`)) found.set(k, v);
        });
        return found;
      }
      if (node === null || typeof node !== "object") return found;
      const record = node as Record<string, unknown>;
      if (Array.isArray(record.required)) {
        found.set(path, [...(record.required as string[])].sort());
      }
      for (const [key, value] of Object.entries(record)) {
        for (const [k, v] of collect(value, `${path}/${key}`)) found.set(k, v);
      }
      return found;
    };

    const priorArrays = collect(before, "#");
    const currentArrays = collect(after, "#");
    expect(priorArrays.size).toBeGreaterThan(0);

    for (const [path, required] of priorArrays) {
      expect(currentArrays.get(path), path).toEqual(required);
    }

    // A newly introduced `required` array is non-disruptive exactly when the
    // property carrying it is optional at its attachment point: an absent
    // optional property is never reached, so a manifest that omits it cannot
    // start failing. Attachment is the nearest enclosing `properties/<name>`,
    // and `<name>` must be absent from that parent object's own `required`.
    const at = (path: string): unknown => {
      let node: unknown = after;
      for (const segment of path.split("/").slice(1)) {
        if (Array.isArray(node)) node = node[Number(segment)];
        else if (node && typeof node === "object")
          node = (node as Record<string, unknown>)[segment];
        else return undefined;
      }
      return node;
    };

    const introduced = [...currentArrays.keys()].filter(
      (path) => !priorArrays.has(path),
    );
    expect(introduced.length).toBeGreaterThan(0);
    for (const path of introduced) {
      const segments = path.split("/");
      const index = segments.lastIndexOf("properties");
      expect(index, `${path} has no enclosing property`).toBeGreaterThan(0);
      const property = segments[index + 1];
      const parent = at(segments.slice(0, index).join("/")) as {
        required?: string[];
      } | null;
      expect(parent?.required ?? [], `${path} is gated on ${property}`).not.toContain(
        property,
      );
    }
  });
});
