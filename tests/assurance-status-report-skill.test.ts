import { readFileSync } from "node:fs";
import { join } from "node:path";

const root = join(__dirname, "..");
const skill = readFileSync(
  join(root, "skills/assurance-status-report/SKILL.md"),
  "utf8",
);
const contract = readFileSync(
  join(root, "skills/assurance-status-report/references/report-contract.md"),
  "utf8",
);

describe("assurance status report skill", () => {
  it("delegates measurement and provenance semantics to Quoin and Quire", () => {
    expect(skill).toContain("quoin report --format json");
    expect(skill).toContain("quoin report --series METRIC --format json");
    expect(skill).toContain("quire provenance --json");
    expect(skill).toMatch(/Do not recreate their\s+analysis in the skill/);
  });

  it("pins remote queries and requires exact source identity", () => {
    expect(skill).toContain("X-GitHub-Api-Version: 2022-11-28");
    expect(skill).toContain("full 40-character SHAs");
    expect(skill).toContain("lock and executable digests");
    expect(skill).toMatch(/dirty, unreachable, ambiguous, or\nwrong-origin/);
  });

  it("requires definitions before metrics and honest comparisons", () => {
    expect(skill.indexOf("Define every project-specific term")).toBeLessThan(
      skill.indexOf("For every percentage"),
    );
    expect(skill).toContain("numerator and denominator");
    expect(skill).toContain("definition versions and population identities");
    expect(contract).toContain("Incomparable");
    expect(contract).toContain("Do not calculate a delta when:");
  });

  it("enumerates completed and open work without equating closure to proof", () => {
    expect(skill).toContain("A closed ticket is workflow state, not proof");
    for (const disposition of [
      "blocking",
      "non-blocking",
      "downstream",
      "excluded",
      "unknown",
    ]) {
      expect(contract).toContain(`**${disposition}:**`);
    }
    expect(contract).toContain("Separately enumerate promotion PRs");
  });

  it("is read-only and fails visibly on unavailable evidence", () => {
    expect(skill).toContain("This workflow is read-only");
    expect(skill).toContain("must not run measurement producers");
    expect(skill).toContain("must not");
    expect(skill).toContain("create/comment/close/reopen issues");
    expect(skill).toContain("`unknown` or `not_computed`");
    expect(contract).toContain(
      "Never turn incomplete, missing, or incomparable evidence into zero",
    );
  });
});
