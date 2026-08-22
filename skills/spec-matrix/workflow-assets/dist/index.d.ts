export interface SpecSkill {
    readonly id: string;
    readonly path: string;
    readonly workflows: readonly string[];
    readonly category: "create-edit" | "write" | "derive" | "review" | "analysis" | "planning" | "graph";
}
export declare const specSkills: readonly [{
    readonly id: "spec-create";
    readonly path: "skills/create";
    readonly workflows: readonly ["spec-create", "spec-edit", "spec-batch"];
    readonly category: "create-edit";
}, {
    readonly id: "spec-write-str";
    readonly path: "skills/write-str";
    readonly workflows: readonly ["write-str"];
    readonly category: "write";
}, {
    readonly id: "spec-write-us";
    readonly path: "skills/write-us";
    readonly workflows: readonly ["write-us"];
    readonly category: "write";
}, {
    readonly id: "spec-write-fr";
    readonly path: "skills/write-fr";
    readonly workflows: readonly ["write-fr"];
    readonly category: "write";
}, {
    readonly id: "spec-write-nfr";
    readonly path: "skills/write-nfr";
    readonly workflows: readonly ["write-nfr"];
    readonly category: "write";
}, {
    readonly id: "spec-write-it";
    readonly path: "skills/write-it";
    readonly workflows: readonly ["write-it"];
    readonly category: "write";
}, {
    readonly id: "spec-us-to-fr";
    readonly path: "skills/us-to-fr";
    readonly workflows: readonly ["us-to-fr"];
    readonly category: "derive";
}, {
    readonly id: "spec-convert-objects";
    readonly path: "skills/convert-objects";
    readonly workflows: readonly ["convert-objects"];
    readonly category: "derive";
}, {
    readonly id: "spec-matrix";
    readonly path: "skills/matrix";
    readonly workflows: readonly ["matrix"];
    readonly category: "review";
}, {
    readonly id: "spec-review";
    readonly path: "skills/review";
    readonly workflows: readonly ["review"];
    readonly category: "review";
}, {
    readonly id: "spec-analysis-integrity";
    readonly path: "skills/analysis-integrity";
    readonly workflows: readonly ["analysis-integrity"];
    readonly category: "analysis";
}, {
    readonly id: "spec-analysis-failure-domain";
    readonly path: "skills/analysis-failure-domain";
    readonly workflows: readonly ["analysis-failure-domain"];
    readonly category: "analysis";
}, {
    readonly id: "spec-analysis-dependency";
    readonly path: "skills/analysis-dependency";
    readonly workflows: readonly ["analysis-dependency"];
    readonly category: "analysis";
}, {
    readonly id: "spec-analysis-evidence";
    readonly path: "skills/analysis-evidence";
    readonly workflows: readonly ["analysis-evidence"];
    readonly category: "analysis";
}, {
    readonly id: "spec-analysis-risk-complexity";
    readonly path: "skills/analysis-risk-complexity";
    readonly workflows: readonly ["analysis-risk-complexity"];
    readonly category: "analysis";
}, {
    readonly id: "spec-analysis-scope-boundary";
    readonly path: "skills/analysis-scope-boundary";
    readonly workflows: readonly ["analysis-scope-boundary"];
    readonly category: "analysis";
}, {
    readonly id: "spec-object-review";
    readonly path: "skills/object-review";
    readonly workflows: readonly ["object-review"];
    readonly category: "review";
}, {
    readonly id: "spec-app-review";
    readonly path: "skills/app-review";
    readonly workflows: readonly ["app-review"];
    readonly category: "review";
}, {
    readonly id: "spec-security-analysis";
    readonly path: "skills/security-analysis";
    readonly workflows: readonly ["security-analysis"];
    readonly category: "analysis";
}, {
    readonly id: "spec-blueprint";
    readonly path: "skills/blueprint";
    readonly workflows: readonly ["blueprint"];
    readonly category: "planning";
}, {
    readonly id: "spec-task-generation-readiness";
    readonly path: "skills/task-generation-readiness";
    readonly workflows: readonly ["task-generation-readiness"];
    readonly category: "planning";
}, {
    readonly id: "spec-to-plan";
    readonly path: "skills/to-plan";
    readonly workflows: readonly ["to-plan"];
    readonly category: "planning";
}, {
    readonly id: "spec-implementation-gap-analysis";
    readonly path: "skills/implementation-gap-analysis";
    readonly workflows: readonly ["implementation-gap-analysis"];
    readonly category: "analysis";
}, {
    readonly id: "spec-core-search";
    readonly path: "skills/core-search";
    readonly workflows: readonly ["core-search"];
    readonly category: "graph";
}, {
    readonly id: "spec-core-explore-objects";
    readonly path: "skills/core-explore-objects";
    readonly workflows: readonly ["core-explore-objects"];
    readonly category: "graph";
}, {
    readonly id: "spec-core-trace-analysis";
    readonly path: "skills/core-trace-analysis";
    readonly workflows: readonly ["core-trace-analysis"];
    readonly category: "graph";
}, {
    readonly id: "spec-core-review";
    readonly path: "skills/core-review";
    readonly workflows: readonly ["core-review"];
    readonly category: "graph";
}];
export declare function skillById(id: string): SpecSkill | undefined;
export declare function workflowSkillPath(workflowName: string): string | undefined;
export { artifactFrontmatterValid, artifactRelationshipsResolve, clearProjectRegistryCache, specInvariants, } from './invariants.js';
//# sourceMappingURL=index.d.ts.map