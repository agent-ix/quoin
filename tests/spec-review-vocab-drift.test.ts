import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

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
  transitions: Array<{ from: string; invariants: string[] }>;
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

async function loadInvariants() {
  const source = invariantSource.replace(
    /^import \{ specInvariants \} from [^;]+;/m,
    "const specInvariants = {};",
  );
  return (await import(
    `data:text/javascript;base64,${Buffer.from(source).toString("base64")}`
  )) as {
    invariants: Record<
      string,
      (input: { instance: Record<string, unknown> }) =>
        | true
        | {
            ok: false;
            code: string;
            details: { missing?: string[]; unsupported?: string[] };
          }
    >;
  };
}

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
  // Trace: FR-043-AC-7 / TC-283
  it("keeps the direct skill, workflow docs, intake, gate, and schema aligned", () => {
    const direct = directAnalysisSet();
    expect(direct.size).toBe(7);
    expect(direct).toContain("ears-conformance");
    expect(sorted(workflowSkillSet())).toEqual(sorted(direct));
    expect(sorted(workflowTemplateSet())).toEqual(sorted(direct));
    expect(sorted(workflowPromptSet())).toEqual(sorted(direct));
    expect(sorted(invariantSet())).toEqual(sorted(direct));
    expect(
      workflowDefinition.transitions.find(({ from }) => from === "intake")
        ?.invariants,
    ).toContain("review_selection_consistent");

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

  // Trace: FR-043-AC-5 / TC-281
  // Trace: FR-043-AC-6 / TC-282
  it("blocks an all-set review until the EARS review document is recorded", async () => {
    const module = await loadInvariants();
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

  // Trace: FR-043-AC-4 / TC-280
  // Trace: FR-043-AC-5 / TC-281
  // Trace: FR-043-AC-6 / TC-282
  // Trace: FR-043-AC-7 / TC-283
  it("cannot truncate all or bypass a required profile selection", async () => {
    const module = await loadInvariants();
    const analyses = [...directAnalysisSet()];
    const truncated = analyses.filter((analysis) => analysis !== "evidence");
    const allInstance = {
      items: {
        operation_request: [
          {
            interviewId: "request",
            review_set: "all",
            selected_analyses: truncated,
          },
        ],
        review_doc: truncated.map((analysis) => ({ analysis, path: analysis })),
      },
    };
    expect(
      module.invariants.review_selection_consistent({ instance: allInstance }),
    ).toMatchObject({
      ok: false,
      code: "all_set_incomplete",
      details: { missing: ["evidence"] },
    });
    expect(
      module.invariants.selected_analyses_covered({ instance: allInstance }),
    ).toMatchObject({
      ok: false,
      details: { missing: ["evidence"] },
    });

    const required = {
      items: {
        operation_request: [
          {
            interviewId: "request",
            review_set: "subset",
            selected_analyses: ["integrity"],
            assurance_profile: "spec/assurance/AP-001.md",
            profile_selection_mode: "require",
            profile_analyses: ["integrity", "evidence"],
          },
        ],
        review_doc: [{ analysis: "integrity", path: "integrity" }],
      },
    };
    required.items.operation_request[0].assurance_profile = "";
    expect(
      module.invariants.review_selection_consistent({ instance: required }),
    ).toMatchObject({
      ok: false,
      code: "required_profile_path_missing",
    });
    required.items.operation_request[0].assurance_profile =
      "spec/assurance/AP-001.md";
    required.items.operation_request[0].review_set = "all";
    expect(
      module.invariants.review_selection_consistent({ instance: required }),
    ).toMatchObject({
      ok: false,
      code: "required_profile_review_set",
    });
    required.items.operation_request[0].review_set = "subset";
    expect(
      module.invariants.review_selection_consistent({ instance: required }),
    ).toMatchObject({
      ok: false,
      code: "required_profile_selection_mismatch",
    });
    required.items.operation_request[0].selected_analyses.push("evidence");
    expect(
      module.invariants.review_selection_consistent({ instance: required }),
    ).toBe(true);
    expect(
      module.invariants.selected_analyses_covered({ instance: required }),
    ).toMatchObject({
      ok: false,
      details: { missing: ["evidence"] },
    });

    required.items.operation_request[0].profile_analyses = [
      "architecture-evaluation",
    ];
    required.items.operation_request[0].selected_analyses = [
      "architecture-evaluation",
    ];
    expect(
      module.invariants.review_selection_consistent({ instance: required }),
    ).toMatchObject({
      ok: false,
      code: "unsupported_selected_analysis",
      details: { unsupported: ["architecture-evaluation"] },
    });
  });

  // Trace: FR-043-AC-3 / TC-279
  it("keeps a recommended profile advisory when the user chooses base", async () => {
    const module = await loadInvariants();
    const instance = {
      items: {
        operation_request: [
          {
            interviewId: "request",
            review_set: "base",
            selected_analyses: [],
            assurance_profile: "spec/assurance/AP-001.md",
            profile_selection_mode: "recommend",
            profile_analyses: ["architecture-evaluation"],
          },
        ],
        review_doc: [],
      },
    };
    expect(module.invariants.review_selection_consistent({ instance })).toBe(
      true,
    );
    expect(module.invariants.selected_analyses_covered({ instance })).toBe(
      true,
    );
  });

  // Trace: FR-043-AC-8 / TC-284
  it("preserves the ordinary base path when no profile applies", async () => {
    const module = await loadInvariants();
    const instance = {
      items: {
        operation_request: [
          {
            interviewId: "request",
            review_set: "base",
            selected_analyses: [],
          },
        ],
        review_doc: [],
      },
    };
    expect(module.invariants.review_selection_consistent({ instance })).toBe(
      true,
    );
    expect(module.invariants.selected_analyses_covered({ instance })).toBe(
      true,
    );
  });
});
