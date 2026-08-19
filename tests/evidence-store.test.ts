/**
 * FR-030 — the evidence store (TC-119..TC-132).
 */

import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { beforeEach, describe, expect, it } from "vitest";

import {
  affirm,
  bind,
  bindingsPath,
  canonicalJson,
  gc,
  latestRun,
  listRuns,
  readBindings,
  readRun,
  readRuns,
  runPath,
  short,
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
  // TC-119
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
  // TC-120
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
  // TC-121
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
  // TC-122
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
  // TC-123
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
  // TC-124
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
  // TC-125
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
  // TC-126
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
  // TC-127
  it("returns an empty binding graph before anything has been recorded", () => {
    expect(readBindings(repo).bindings).toEqual([]);
    expect(listRuns(repo, "SUITE-001")).toEqual([]);
    expect(gc(repo)).toEqual([]);
  });
});

describe("TC-128 no obligation is ever stored", () => {
  // TC-128
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
  // TC-129
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

describe("TC-130 the latest run is the newest, not the highest filename (FR-030-AC-12)", () => {
  // A run filename is `<commit12>.json` and a commit prefix is uniformly random
  // hex, so `listRuns().at(-1)` picked the newest run with probability 1/n.
  // These fixtures are built so the two answers DISAGREE: the newest run's
  // commit sorts FIRST. Under the old code every assertion below inverted
  // (agent-ix/quoin#104).
  const OLD_HIGH = "ffffffffffff0000"; // older, but sorts last
  const NEW_LOW = "000000000000ffff"; // newer, but sorts first

  function twoRuns(): void {
    writeRun(repo, {
      schemaVersion: 1,
      suite: "SUITE-001",
      commit: OLD_HIGH,
      tool: "cargo test",
      timestamp: "2026-08-01T00:00:00Z",
      entries: [{ symbol: "tests::old", outcome: "pass" }],
    });
    writeRun(repo, {
      schemaVersion: 1,
      suite: "SUITE-001",
      commit: NEW_LOW,
      tool: "cargo test",
      timestamp: "2026-08-17T00:00:00Z",
      entries: [{ symbol: "tests::new", outcome: "pass" }],
    });
  }

  // TC-130
  it("reads the newest by timestamp even when its filename sorts first", () => {
    twoRuns();
    // The premise: filename order and time order genuinely disagree here.
    expect(listRuns(repo, "SUITE-001").at(-1)).toBe(`${short(OLD_HIGH)}.json`);
    expect(latestRun(repo, "SUITE-001")?.commit).toBe(NEW_LOW);
    expect(readRuns(repo, "SUITE-001").map((r) => r.commit)).toEqual([
      OLD_HIGH,
      NEW_LOW,
    ]);
  });

  it("gc keeps the newest run, not the highest filename", () => {
    twoRuns();
    const deleted = gc(repo);
    expect(deleted).toEqual([
      join(storeRoot(repo), "runs", "SUITE-001", `${short(OLD_HIGH)}.json`),
    ]);
    expect(listRuns(repo, "SUITE-001")).toEqual([`${short(NEW_LOW)}.json`]);
  });

  it("orders by commit when two runs share a timestamp", () => {
    for (const commit of ["bbbb00000000", "aaaa00000000"]) {
      writeRun(repo, {
        schemaVersion: 1,
        suite: "SUITE-002",
        commit,
        tool: "cargo test",
        timestamp: "2026-08-17T00:00:00Z",
        entries: [],
      });
    }
    // A tie must still resolve the same way on every machine.
    expect(readRuns(repo, "SUITE-002").map((r) => r.commit)).toEqual([
      "aaaa00000000",
      "bbbb00000000",
    ]);
  });
});

describe("TC-131 a corrupt store file is named, not a bare SyntaxError (FR-030-AC-13)", () => {
  // `bindings.json` and `baseline.json` are checked into git, so a merge
  // conflict leaves `<<<<<<< HEAD` in one of them. Every store read used to
  // throw `SyntaxError: Unexpected token '<'` naming no file
  // (agent-ix/quoin#106).
  const CONFLICTED = '<<<<<<< HEAD\n{"bindings": []}\n=======\n';

  // TC-131
  it("names the file and the cause when the binding graph is unreadable", () => {
    mkdirSync(storeRoot(repo), { recursive: true });
    writeFileSync(bindingsPath(repo), CONFLICTED, "utf8");
    expect(() => readBindings(repo)).toThrow(
      /bindings\.json exists but is not/,
    );
    expect(() => readBindings(repo)).toThrow(/merge conflict/);
  });

  it("skips one corrupt run file instead of hiding every finding", () => {
    writeRun(repo, {
      schemaVersion: 1,
      suite: "SUITE-001",
      commit: "aaaaaaaaaaaa0000",
      tool: "cargo test",
      timestamp: "2026-08-17T00:00:00Z",
      entries: [{ symbol: "tests::ok", outcome: "pass" }],
    });
    writeFileSync(
      runPath(repo, "SUITE-001", "bbbbbbbbbbbb0000"),
      "{ truncated",
      "utf8",
    );

    const skipped: string[] = [];
    const runs = readRuns(repo, "SUITE-001", skipped);
    // The good record still reaches the caller; the bad one is reported.
    expect(runs.map((r) => r.commit)).toEqual(["aaaaaaaaaaaa0000"]);
    expect(skipped).toHaveLength(1);
    expect(skipped[0]).toContain("bbbbbbbbbbbb.json");
  });
});

describe("TC-132 the store's byte order does not depend on the locale (FR-030-AC-14)", () => {
  // `writeBindings` sorted with `localeCompare`, whose collation depends on the
  // runtime's ICU data — so two machines could serialize one binding set two
  // ways and produce a diff nobody made, in a file whose diff is meant to BE
  // the per-PR delta (agent-ix/quoin#106).
  // TC-132
  it("writes an exact, pinned byte sequence", () => {
    writeBindings(repo, {
      schemaVersion: 1,
      bindings: [
        {
          obligation: "FR-001-AC-2",
          statementHashAtBinding: HASH_B,
          suite: "SUITE-001",
          commit: COMMIT,
          symbols: ["b"],
        },
        {
          obligation: "FR-001-AC-1",
          statementHashAtBinding: HASH_A,
          suite: "SUITE-002",
          commit: COMMIT,
          symbols: ["a"],
        },
        {
          obligation: "FR-001-AC-1",
          statementHashAtBinding: HASH_A,
          suite: "SUITE-001",
          commit: COMMIT,
          symbols: ["a"],
        },
      ],
    });
    const written = readFileSync(bindingsPath(repo), "utf8");
    const order = [...written.matchAll(/"suite": "([^"]+)"/g)].map((m) => m[1]);
    // (obligation, suite): AC-1/SUITE-001, AC-1/SUITE-002, then AC-2/SUITE-001.
    expect(order).toEqual(["SUITE-001", "SUITE-002", "SUITE-001"]);
    expect(written.endsWith("\n")).toBe(true);
  });
});

describe("TC-245 a run binds through an obligation's declared test cases", () => {
  // A tool reports the id it knows. A unit test carries the criterion's own id
  // because the tag is written in the test; an agent-eval report — and any tool
  // keyed on the Test Matrix — carries the TEST CASE id. Both are stated by the
  // same criteria row, and quire-rs FR-053-AC-11 carries that join on the
  // obligation, so the store resolves it rather than re-parsing the table.
  //
  // Before this, `quoin evidence record --adapter agent-eval` reported
  // `bound: 0` and `unmatched trace ids … TC-EV-057` while `FR-038-AC-8 →
  // TC-EV-057` sat in the FR's own table (agent-ix/quoin#144).
  // TC-245
  it("binds a test-case id to the criterion whose cell names it", () => {
    const outcome = recordRun({
      repo,
      suite: "EVAL-001",
      commit: COMMIT,
      tool: "cli-agent-evals",
      timestamp: "2026-08-17T00:00:00Z",
      entries: [
        { symbol: "TC-EV-057", outcome: "pass", traceIds: ["TC-EV-057"] },
      ],
      obligations: [
        { ...obligation("FR-038-AC-8", HASH_A), target_ids: ["TC-EV-057"] },
      ],
    });
    expect(outcome.bound).toEqual(["FR-038-AC-8"]);
    expect(outcome.unmatched).toEqual([]);
    expect(readBindings(repo).bindings[0].symbols).toEqual(["TC-EV-057"]);
  });

  it("binds every criterion the same test case discharges", () => {
    // A row says each of those criteria is verified by that test case, so one
    // run discharging it discharges all of them. Reporting only the first would
    // leave the rest undischarged with evidence sitting right there.
    const outcome = recordRun({
      repo,
      suite: "EVAL-001",
      commit: COMMIT,
      tool: "cli-agent-evals",
      timestamp: "2026-08-17T00:00:00Z",
      entries: [
        { symbol: "TC-EV-054", outcome: "pass", traceIds: ["TC-EV-054"] },
      ],
      obligations: [
        { ...obligation("FR-038-AC-1", HASH_A), target_ids: ["TC-EV-054"] },
        { ...obligation("FR-038-AC-2", HASH_A), target_ids: ["TC-EV-054"] },
      ],
    });
    expect(outcome.bound.sort()).toEqual(["FR-038-AC-1", "FR-038-AC-2"]);
  });

  it("prefers a direct obligation id over the indirect route", () => {
    // If a criterion's cell happened to name a SIBLING criterion's id, binding
    // through the indirect route would report a discharge nobody stated
    // directly. The direct match wins and the indirect one is not registered.
    const outcome = recordRun({
      repo,
      suite: "SUITE-001",
      commit: COMMIT,
      tool: "cargo test",
      timestamp: "2026-08-17T00:00:00Z",
      entries: [
        { symbol: "tests::tc001", outcome: "pass", traceIds: ["FR-001-AC-2"] },
      ],
      obligations: [
        { ...obligation("FR-001-AC-1", HASH_A), target_ids: ["FR-001-AC-2"] },
        obligation("FR-001-AC-2", HASH_A),
      ],
    });
    expect(outcome.bound).toEqual(["FR-001-AC-2"]);
  });

  it("still reports a trace id no obligation states, by either route", () => {
    const outcome = recordRun({
      repo,
      suite: "EVAL-001",
      commit: COMMIT,
      tool: "cli-agent-evals",
      timestamp: "2026-08-17T00:00:00Z",
      entries: [
        { symbol: "TC-EV-999", outcome: "pass", traceIds: ["TC-EV-999"] },
      ],
      obligations: [
        { ...obligation("FR-038-AC-8", HASH_A), target_ids: ["TC-EV-057"] },
      ],
    });
    expect(outcome.bound).toEqual([]);
    expect(outcome.unmatched).toEqual(["TC-EV-999"]);
  });
});
