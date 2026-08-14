import { defineSuite } from "../cli-agent-evals/dist/index.js";
import { makeScenarioWorkspace, transcriptPathFor } from "./evals/lib/env.mjs";
import { buildTaskBrief, kickoffLine } from "./evals/lib/prompt.mjs";
import { assertExpectations } from "./evals/lib/assert.mjs";
import { extractMetrics } from "./evals/lib/metrics.mjs";
import { buildSeedOnce } from "./evals/lib/seed.mjs";
import { findQuire, shimDir } from "./evals/lib/resolve.mjs";
import { SCENARIOS } from "./evals/scenarios/index.mjs";

let seed;

export default defineSuite({
  name: "quoin",
  rootDir: import.meta.dirname,
  scenarios: SCENARIOS.map((scenario) => ({
    ...scenario,
    canary: scenario.canary ?? ["TC-EV-001", "TC-EV-008"].includes(scenario.id),
  })),
  workspace(scenario, suite) {
    seed ??= buildSeedOnce({
      force: false,
      log: (message) => console.log(message),
    });
    const ctx = makeScenarioWorkspace(scenario.id);
    ctx.workDir = ctx.work;
    ctx.reportDir = `${suite.rootDir}/evals/reports`;
    return ctx;
  },
  buildTaskBrief,
  kickoffLine,
  shimPath: () => shimDir(),
  extraEnv(ctx) {
    return {
      IX_HOME: ctx.ixHome,
      IX_SCHEMA_PATH: ctx.modulesDir,
    };
  },
  agents: {
    claude: {
      parseMetrics: extractMetrics,
      transcriptPath(ctx) {
        return transcriptPathFor(ctx.cwd, ctx.sessionId);
      },
    },
  },
  assert(ctx, scenario, run) {
    const extraEnv = scenario.env ? scenario.env(ctx) : {};
    return assertExpectations(ctx, scenario.expect ?? {}, run, {
      quireBin: findQuire(),
      modulesDir: ctx.modulesDir,
      extraEnv,
    });
  },
});
