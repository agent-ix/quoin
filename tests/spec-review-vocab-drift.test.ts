import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { parse as parseYaml } from "yaml";
import { describe, expect, it } from "vitest";

import { filamentModulesDir } from "../src/catalog.js";

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const directSkill = readFileSync(
  join(repoRoot, "skills/spec-review/SKILL.md"),
  "utf8",
);
const workflowRoot = join(
  repoRoot,
  "skills/spec-review/workflow-assets/skills/review",
);
const workflowSkill = readFileSync(join(workflowRoot, "SKILL.md"), "utf8");
const workflowDefinition = parseYaml(
  readFileSync(join(workflowRoot, "workflows/review/def.yaml"), "utf8"),
) as {
  interviews: {
    request: {
      questions: Array<{ key: string; prompt: string }>;
    };
  };
};
const invariantSource = readFileSync(
  join(workflowRoot, "scripts/invariants.js"),
  "utf8",
);

const sorted = (values: Iterable<string>) => [...values].sort();

function directAnalysisSet(): Set<string> {
  return new Set(
    [
      ...directSkill.matchAll(
        /^\|\s*([a-z][a-z-]+)\s*\|\s*`spec-[^`]+`\s*\|$/gm,
      ),
    ].map((match) => match[1]),
  );
}

function workflowSkillSet(): Set<string> {
  const section = /- `all`[^\n]*analyses:([\s\S]*?)\n\s*- `subset`/.exec(
    workflowSkill,
  );
  expect(section, "workflow SKILL has no `all` analysis list").toBeTruthy();
  return new Set(
    [...(section?.[1] ?? "").matchAll(/`([a-z][a-z-]+)`/g)].map(
      (match) => match[1],
    ),
  );
}

function workflowTemplateSet(): Set<string> {
  const analysis = /^analysis: <([^>]+)>$/m.exec(workflowSkill);
  expect(
    analysis,
    "workflow SKILL has no SpecReview analysis reference",
  ).toBeTruthy();
  return new Set(
    (analysis?.[1] ?? "").split("|").filter((value) => value !== "base"),
  );
}

function workflowPromptSet(): Set<string> {
  const question = workflowDefinition.interviews.request.questions.find(
    ({ key }) => key === "selected_analyses",
  );
  expect(question, "workflow has no selected_analyses question").toBeTruthy();
  const list = /drawn from: ([^.]+)\./.exec(question?.prompt ?? "");
  expect(list, "selected_analyses prompt has no declared list").toBeTruthy();
  return new Set((list?.[1] ?? "").split(", "));
}

function invariantSet(): Set<string> {
  const array = /const ALL_ANALYSES = (\[[\s\S]*?\]);/.exec(invariantSource);
  expect(array, "invariant has no ALL_ANALYSES declaration").toBeTruthy();
  return new Set(
    [...(array?.[1] ?? "").matchAll(/"([a-z][a-z-]+)"/g)].map(
      (match) => match[1],
    ),
  );
}

describe("spec-review analysis vocabulary", () => {
  it("keeps the direct skill, workflow docs, intake, gate, and schema aligned", () => {
    const direct = directAnalysisSet();
    expect(direct.size).toBe(7);
    expect(direct).toContain("ears-conformance");
    expect(sorted(workflowSkillSet())).toEqual(sorted(direct));
    expect(sorted(workflowTemplateSet())).toEqual(sorted(direct));
    expect(sorted(workflowPromptSet())).toEqual(sorted(direct));
    expect(sorted(invariantSet())).toEqual(sorted(direct));

    const schema = JSON.parse(
      readFileSync(
        join(
          filamentModulesDir(),
          "spec-artifacts-process/schemas/spec-review-frontmatter.schema.json",
        ),
        "utf8",
      ),
    ) as { properties: { analysis: { enum: string[] } } };
    for (const analysis of direct) {
      expect(schema.properties.analysis.enum).toContain(analysis);
    }
  });

  it("blocks an all-set review until the EARS review document is recorded", async () => {
    const module = (await import(
      pathToFileURL(join(workflowRoot, "scripts/invariants.js")).href
    )) as {
      invariants: Record<
        string,
        (input: { instance: Record<string, unknown> }) =>
          | true
          | {
              ok: false;
              code: string;
              details: { missing: string[] };
            }
      >;
    };
    const direct = directAnalysisSet();
    const reviewDocs = [...direct]
      .filter((analysis) => analysis !== "ears-conformance")
      .map((analysis) => ({ analysis, path: `spec/reviews/${analysis}.md` }));
    const instance = {
      items: {
        operation_request: [{ interviewId: "request", review_set: "all" }],
        review_doc: reviewDocs,
      },
    };

    const incomplete = module.invariants.selected_analyses_covered({
      instance,
    });
    expect(incomplete).toMatchObject({
      ok: false,
      code: "selected_analyses_not_run",
      details: { missing: ["ears-conformance"] },
    });

    reviewDocs.push({
      analysis: "ears-conformance",
      path: "spec/reviews/ears-conformance.md",
    });
    expect(module.invariants.selected_analyses_covered({ instance })).toBe(
      true,
    );
  });
});
