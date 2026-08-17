/**
 * FR-030 — the evidence store (TC-119..TC-129).
 */

import { mkdtempSync, readFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { beforeEach, describe, expect, it } from "vitest";

import {
  affirm,
  bind,
  canonicalJson,
  gc,
  listRuns,
  readBindings,
  readRun,
  runPath,
  storeRoot,
  writeBindings,
  writeRun,
  type Binding,
} from "../src/evidence/index.js";
import { recordRun } from "../src/evidence/record.js";
import { STORE_SCHEMA_VERSION } from "../src/evidence/types.js";
import type { Obligation } from "../src/quire/index.js";

let repo: string;

beforeEach(() => {
  repo = mkdtempSync(join(tmpdir(), "quoin-evidence-"));
});

function obligation(id: string, hash: string): Obligation {
  return {
    source: "acceptance-criterion",
    id,
    document: "spec/functional/FR-001.md",
    statement: `statement for ${id}`,
    statement_hash: hash,
  };
}

const HASH_A = "a".repeat(64);
const COMMIT = "abcdef0123456789";
const HASH_B = "b".repeat(64);

describe("TC-119 the store lives under spec/", () => {
  it("places the store inside the walked document root", () => {
    // quire-rs CR-045 bounds the document walk to <scope>/spec, so the authored
    // half is only a validated corpus document if the store lives there. A
    // registry at the repository root mints nothing and is reported nowhere.
    expect(storeRoot(repo)).toBe(join(repo, "spec", "evidence"));
  });

  it("names a run file by suite and 12-char commit prefix", () => {
    expect(runPath(repo, "SUITE-001", "abcdef0123456789")).toBe(
      join(repo, "spec", "evidence", "runs", "SUITE-001", "abcdef012345.json"),
    );
  });
});

describe("TC-120 writes are canonical, so a diff of the store is the delta", () => {
  it("sorts keys at every level and ends with a newline", () => {
    const json = canonicalJson({ b: 1, a: { d: 2, c: 3 } });
    expect(json).toBe(
      '{\n  "a": {\n    "c": 3,\n    "d": 2\n  },\n  "b": 1\n}\n',
    );
  });

  it("serializes the same record identically twice", () => {
    const record = {
      schemaVersion: STORE_SCHEMA_VERSION,
      suite: "SUITE-001",
      commit: "abcdef0123456789",
      tool: "cargo test",
      timestamp: "2026-08-17T00:00:00Z",
      entries: [{ symbol: "tests::tc001", outcome: "pass" as const }],
    };
    writeRun(repo, record);
    const first = readFileSync(
      runPath(repo, "SUITE-001", record.commit),
      "utf8",
    );
    writeRun(repo, record);
    const second = readFileSync(
      runPath(repo, "SUITE-001", record.commit),
      "utf8",
    );
    expect(second).toBe(first);
  });
});

describe("TC-121 the suite is the atomic unit of evidence", () => {
  it("keeps one file per (suite, commit) and does not merge suites", () => {
    const base = {
      schemaVersion: STORE_SCHEMA_VERSION,
      commit: "abcdef0123456789",
      tool: "t",
      timestamp: "2026-08-17T00:00:00Z",
      entries: [],
    };
    writeRun(repo, { ...base, suite: "SUITE-001" });
    writeRun(repo, { ...base, suite: "SUITE-002" });
    // A partial run must never masquerade as a full one — the `make ci` /
    // `make ci-python` split already caused exactly that rot (quire-rs TC-715).
    expect(listRuns(repo, "SUITE-001")).toEqual(["abcdef012345.json"]);
    expect(listRuns(repo, "SUITE-002")).toEqual(["abcdef012345.json"]);
  });

  it("is last-write-wins at the same commit", () => {
    const base = {
      schemaVersion: STORE_SCHEMA_VERSION,
      suite: "SUITE-001",
      commit: "abcdef0123456789",
      timestamp: "2026-08-17T00:00:00Z",
      entries: [],
    };
    writeRun(repo, { ...base, tool: "first" });
    writeRun(repo, { ...base, tool: "second" });
    expect(listRuns(repo, "SUITE-001")).toHaveLength(1);
    expect(readRun(repo, "SUITE-001", base.commit)?.tool).toBe("second");
  });
});

describe("TC-122 first discharge auto-binds and stamps the hash", () => {
  it("binds a passing symbol's obligation with the hash as it stands now", () => {
    const outcome = recordRun({
      repo,
      suite: "SUITE-001",
      commit: "abcdef0123456789",
      tool: "cargo test",
      timestamp: "2026-08-17T00:00:00Z",
      entries: [
        { symbol: "tests::tc001", outcome: "pass", traceIds: ["FR-001-AC-1"] },
      ],
      obligations: [obligation("FR-001-AC-1", HASH_A)],
    });
    expect(outcome.bound).toEqual(["FR-001-AC-1"]);
    const stored = readBindings(repo).bindings;
    expect(stored).toHaveLength(1);
    expect(stored[0].statementHashAtBinding).toBe(HASH_A);
    expect(stored[0].symbols).toEqual(["tests::tc001"]);
  });

  it("does not bind on a failing or skipped symbol", () => {
    // The run record already says the suite ran. A red result is not evidence
    // the obligation holds, and binding on it would make the store agree with
    // a failing build.
    const outcome = recordRun({
      repo,
      suite: "SUITE-001",
      commit: "abcdef0123456789",
      tool: "cargo test",
      timestamp: "2026-08-17T00:00:00Z",
      entries: [
        { symbol: "tests::tc001", outcome: "fail", traceIds: ["FR-001-AC-1"] },
        { symbol: "tests::tc002", outcome: "skip", traceIds: ["FR-001-AC-2"] },
      ],
      obligations: [
        obligation("FR-001-AC-1", HASH_A),
        obligation("FR-001-AC-2", HASH_A),
      ],
    });
    expect(outcome.bound).toEqual([]);
    expect(readBindings(repo).bindings).toHaveLength(0);
    // But the run itself is recorded — a suite that stopped passing is exactly
    // what a freshness check needs to see.
    expect(existsSync(outcome.runPath)).toBe(true);
  });
});

describe("TC-123 a reworded statement makes a binding suspect", () => {
  it("reports suspicion and does NOT overwrite the hash", () => {
    const request = {
      repo,
      suite: "SUITE-001",
      commit: "abcdef0123456789",
      tool: "cargo test",
      timestamp: "2026-08-17T00:00:00Z",
      entries: [
        {
          symbol: "tests::tc001",
          outcome: "pass" as const,
          traceIds: ["FR-001-AC-1"],
        },
      ],
    };
    recordRun({ ...request, obligations: [obligation("FR-001-AC-1", HASH_A)] });

    // The criterion is reworded; its hash changes. Re-running the same test
    // must NOT clear the suspicion — if it did, the state would clear itself on
    // the next CI run and the detector would never fire.
    const second = recordRun({
      ...request,
      commit: "1111111111111111",
      obligations: [obligation("FR-001-AC-1", HASH_B)],
    });
    expect(second.suspect).toEqual(["FR-001-AC-1"]);
    expect(readBindings(repo).bindings[0].statementHashAtBinding).toBe(HASH_A);
  });
});

describe("TC-124 affirmation is the explicit act", () => {
  it("moves the hash forward and records who, at which commit", () => {
    const existing: Binding[] = [
      {
        obligation: "FR-001-AC-1",
        statementHashAtBinding: HASH_A,
        suite: "SUITE-001",
        commit: "abcdef0123456789",
        symbols: ["tests::tc001"],
      },
    ];
    const { bindings, found } = affirm(
      existing,
      "FR-001-AC-1",
      HASH_B,
      "@reviewer",
      "2222222222222222",
      "wording narrowed, evidence unchanged",
    );
    expect(found).toBe(true);
    expect(bindings[0].statementHashAtBinding).toBe(HASH_B);
    expect(bindings[0].affirmations).toEqual([
      {
        who: "@reviewer",
        commit: "2222222222222222",
        note: "wording narrowed, evidence unchanged",
      },
    ]);
  });

  it("reports an unknown obligation rather than inventing a binding", () => {
    expect(affirm([], "FR-999-AC-9", HASH_A, "@x", "c").found).toBe(false);
  });
});

describe("TC-125 a trace id no obligation states is reported", () => {
  it("names it rather than dropping it", () => {
    // quire-rs#72 from the other direction: a test claiming to verify something
    // the spec does not state.
    const outcome = recordRun({
      repo,
      suite: "SUITE-001",
      commit: "abcdef0123456789",
      tool: "cargo test",
      timestamp: "2026-08-17T00:00:00Z",
      entries: [
        { symbol: "tests::ghost", outcome: "pass", traceIds: ["FR-999-AC-9"] },
      ],
      obligations: [obligation("FR-001-AC-1", HASH_A)],
    });
    expect(outcome.unmatched).toEqual(["FR-999-AC-9"]);
    expect(outcome.bound).toEqual([]);
  });
});

describe("TC-126 gc keeps the latest run and anything a binding references", () => {
  it("deletes only unreferenced older runs", () => {
    const base = {
      schemaVersion: STORE_SCHEMA_VERSION,
      suite: "SUITE-001",
      tool: "t",
      timestamp: "2026-08-17T00:00:00Z",
      entries: [],
    };
    writeRun(repo, { ...base, commit: "aaaaaaaaaaaa0000" });
    writeRun(repo, { ...base, commit: "bbbbbbbbbbbb0000" });
    writeRun(repo, { ...base, commit: "cccccccccccc0000" });
    writeBindings(repo, {
      schemaVersion: STORE_SCHEMA_VERSION,
      bindings: [
        {
          obligation: "FR-001-AC-1",
          statementHashAtBinding: HASH_A,
          suite: "SUITE-001",
          commit: "aaaaaaaaaaaa0000",
          symbols: ["tests::tc001"],
        },
      ],
    });

    const deleted = gc(repo);
    // `cccc…` is latest, `aaaa…` is referenced by a binding; only `bbbb…` goes.
    expect(deleted).toHaveLength(1);
    expect(deleted[0]).toContain("bbbbbbbbbbbb.json");
    expect(listRuns(repo, "SUITE-001")).toEqual([
      "aaaaaaaaaaaa.json",
      "cccccccccccc.json",
    ]);
  });

  it("changes nothing under --dry-run", () => {
    const base = {
      schemaVersion: STORE_SCHEMA_VERSION,
      suite: "SUITE-001",
      tool: "t",
      timestamp: "2026-08-17T00:00:00Z",
      entries: [],
    };
    writeRun(repo, { ...base, commit: "aaaaaaaaaaaa0000" });
    writeRun(repo, { ...base, commit: "bbbbbbbbbbbb0000" });
    expect(gc(repo, true)).toHaveLength(1);
    expect(listRuns(repo, "SUITE-001")).toHaveLength(2);
  });
});

describe("TC-127 an absent store reads as empty, not as an error", () => {
  it("returns an empty binding graph before anything has been recorded", () => {
    expect(readBindings(repo).bindings).toEqual([]);
    expect(listRuns(repo, "SUITE-001")).toEqual([]);
    expect(gc(repo)).toEqual([]);
  });
});

describe("TC-128 no obligation is ever stored", () => {
  it("keeps only the hash, never the statement", () => {
    // The governing principle: store only what cannot be recomputed from
    // spec + code at HEAD. An obligation is always re-derivable, so a stored
    // copy could only ever disagree with the requirement it describes.
    recordRun({
      repo,
      suite: "SUITE-001",
      commit: "abcdef0123456789",
      tool: "cargo test",
      timestamp: "2026-08-17T00:00:00Z",
      entries: [
        { symbol: "tests::tc001", outcome: "pass", traceIds: ["FR-001-AC-1"] },
      ],
      obligations: [obligation("FR-001-AC-1", HASH_A)],
    });
    const raw = readFileSync(join(storeRoot(repo), "bindings.json"), "utf8");
    expect(raw).toContain(HASH_A);
    expect(raw).not.toContain("statement for FR-001-AC-1");
    expect(raw).not.toContain("spec/functional/FR-001.md");
  });
});

describe("bind() is the one place the auto-bind rule lives", () => {
  it("creates on first sight and reports suspicion on a changed hash", () => {
    const first = bind([], {
      obligation: "X",
      statementHashAtBinding: HASH_A,
      suite: "S",
      commit: "c",
      symbols: [],
    });
    expect(first.created).toBe(true);
    expect(first.suspect).toBe(false);

    const second = bind(first.bindings, {
      obligation: "X",
      statementHashAtBinding: HASH_B,
      suite: "S",
      commit: "d",
      symbols: [],
    });
    expect(second.created).toBe(false);
    expect(second.suspect).toBe(true);
    expect(second.bindings[0].statementHashAtBinding).toBe(HASH_A);
  });
});

describe("TC-129 a second suite appends, it does not overwrite (FR-030-AC-11)", () => {
  // `BindingsFile`'s own doc says the graph IS cross-suite — "one obligation
  // can be discharged by a unit test and a benchmark" — and `bind()` keyed on
  // the obligation alone, so the second discharge replaced the first. The
  // relationship the file exists to hold was destroyed on write, silently
  // (agent-ix/quoin#102).
  it("keeps both bindings and orders them by (obligation, suite)", () => {
    const first = bind([], {
      obligation: "FR-001-AC-1",
      statementHashAtBinding: HASH_A,
      suite: "SUITE-002",
      commit: COMMIT,
      symbols: ["bench::tc900"],
    });
    expect(first.created).toBe(true);

    const second = bind(first.bindings, {
      obligation: "FR-001-AC-1",
      statementHashAtBinding: HASH_A,
      suite: "SUITE-001",
      commit: COMMIT,
      symbols: ["tests::tc001"],
    });
    expect(second.created).toBe(true);
    expect(second.bindings).toHaveLength(2);
    expect(second.bindings.map((b) => b.suite).sort()).toEqual([
      "SUITE-001",
      "SUITE-002",
    ]);

    writeBindings(repo, { schemaVersion: 1, bindings: second.bindings });
    // Written in (obligation, suite) order, so the diff is stable.
    expect(readBindings(repo).bindings.map((b) => b.suite)).toEqual([
      "SUITE-001",
      "SUITE-002",
    ]);
  });

  it("re-discharging the SAME suite still merges rather than appending", () => {
    const first = bind([], {
      obligation: "FR-001-AC-1",
      statementHashAtBinding: HASH_A,
      suite: "SUITE-001",
      commit: COMMIT,
      symbols: ["tests::tc001"],
    });
    const again = bind(first.bindings, {
      obligation: "FR-001-AC-1",
      statementHashAtBinding: HASH_A,
      suite: "SUITE-001",
      commit: "9999999999999999",
      symbols: ["tests::tc001", "tests::tc002"],
    });
    expect(again.created).toBe(false);
    expect(again.bindings).toHaveLength(1);
    expect(again.bindings[0].symbols).toEqual(["tests::tc001", "tests::tc002"]);
  });

  it("affirming clears every suite's suspicion, not just the first", () => {
    const bindings = [
      {
        obligation: "FR-001-AC-1",
        statementHashAtBinding: HASH_A,
        suite: "SUITE-001",
        commit: COMMIT,
        symbols: ["tests::tc001"],
      },
      {
        obligation: "FR-001-AC-1",
        statementHashAtBinding: HASH_A,
        suite: "SUITE-002",
        commit: COMMIT,
        symbols: ["bench::tc900"],
      },
    ];
    const out = affirm(bindings, "FR-001-AC-1", HASH_B, "peter", COMMIT);
    expect(out.found).toBe(true);
    expect(out.bindings.every((b) => b.statementHashAtBinding === HASH_B)).toBe(
      true,
    );
    // Narrowed to one suite when the reviewer means only one.
    const narrowed = affirm(
      bindings,
      "FR-001-AC-1",
      HASH_B,
      "peter",
      COMMIT,
      undefined,
      "SUITE-002",
    );
    expect(narrowed.bindings[0].statementHashAtBinding).toBe(HASH_A);
    expect(narrowed.bindings[1].statementHashAtBinding).toBe(HASH_B);
  });
});
