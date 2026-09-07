#!/usr/bin/env node

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  SOURCE_NAMES,
  parseArguments,
  parseContract,
  prepareCandidate,
  writeCandidate,
} from "./verification-relock.mjs";
import { assertRepository, sha256 } from "./verification-stack.mjs";

const git = (root, ...args) =>
  execFileSync("git", ["-C", root, ...args], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
function put(root, path, bytes) {
  mkdirSync(dirname(join(root, path)), { recursive: true });
  writeFileSync(join(root, path), bytes);
}
function commit(root) {
  git(root, "add", "--all");
  git(root, "commit", "-qm", "fixture");
  const revision = git(root, "rev-parse", "HEAD");
  git(root, "update-ref", "refs/remotes/origin/main", revision);
  return revision;
}

export function fixture(scratch) {
  const roots = {};
  for (const name of SOURCE_NAMES) {
    const root = join(scratch, name);
    mkdirSync(root);
    git(root, "init", "-q");
    git(root, "config", "user.email", "relock@example.invalid");
    git(root, "config", "user.name", "relock selftest");
    git(root, "remote", "add", "origin", `https://github.com/agent-ix/${name}`);
    put(root, "source.txt", `${name}\n`);
    roots[name] = root;
  }
  const schemas = [
    "assurance-v1.schema.json",
    "coverage-v1.schema.json",
    "properties-v1.schema.json",
  ];
  const bytes = '{"fixture":true}\n';
  for (const name of schemas) put(roots.quire, `schemas/output/${name}`, bytes);
  const engine = commit(roots.quire);
  put(
    roots["quire-cli"],
    "Cargo.toml",
    `[dependencies]\nquire-rs = { git = "https://github.com/agent-ix/quire", rev = "${engine}" }\n`,
  );
  put(
    roots["quire-cli"],
    "Cargo.lock",
    `[[package]]\nname = "quire-rs"\nsource = "git+https://github.com/agent-ix/quire?rev=${engine}#${engine}"\n`,
  );
  put(
    roots["qa-corpus"],
    "bounds.py",
    'import json, pathlib\nroot = pathlib.Path(__file__).parent\nvalue = json.loads((root / "inventory.json").read_text())\nif (root / "extra.json").exists(): value["cases"] += json.loads((root / "extra.json").read_text())\nprint(json.dumps(value))\n',
  );
  put(
    roots["qa-corpus"],
    "inventory.json",
    '{"cases":[{"mode":"detection"},{"mode":"reporting"}]}\n',
  );
  put(roots["qa-corpus"], ".gitignore", "extra.json\n");
  const qa = commit(roots["qa-corpus"]);
  git(roots.quoin, "clone", "-q", roots["qa-corpus"], "corpus");
  git(
    join(roots.quoin, "corpus"),
    "remote",
    "set-url",
    "origin",
    "https://github.com/agent-ix/qa-corpus",
  );
  git(
    roots.quoin,
    "update-index",
    "--add",
    "--cacheinfo",
    `160000,${qa},corpus`,
  );
  put(
    roots.quoin,
    ".gitmodules",
    '[submodule "corpus"]\n\tpath = corpus\n\turl = https://github.com/agent-ix/qa-corpus\n',
  );
  put(
    roots.quoin,
    "src/quire/contract.ts",
    `export const QUIRE_CONTRACT = { sourceRevision: "${engine}",\nhashes: {${schemas.map((name) => `"${name}": "${sha256(bytes).slice(7)}"`).join(",")}}} as const;\n`,
  );
  for (const name of schemas)
    put(roots.quoin, `src/quire/schemas/${name}`, bytes);
  for (const name of [
    "verification-relock.mjs",
    "verification-relock-selftest.mjs",
    "verification-declarations.mjs",
    "verification-declarations-selftest.mjs",
    "verification-object-integrity-selftest.mjs",
    "workspace-policy-selftest.mjs",
  ]) {
    put(roots.quoin, `scripts/${name}`, "// fixture artifact\n");
  }
  const repositories = {};
  for (const name of SOURCE_NAMES) {
    repositories[name] = {
      remote: `https://github.com/agent-ix/${name}`,
      revision: ["quire", "qa-corpus"].includes(name)
        ? git(roots[name], "rev-parse", "HEAD")
        : commit(roots[name]),
    };
  }
  const base = {
    schemaVersion: "quoin-verification-stack-lock-v1",
    repositories,
    contracts: { quire: repositories.quire },
    cohorts: {
      qaExternalQuoin: {
        revision: repositories.quoin.revision,
        version: "0.1.0-fixture",
      },
      quireBenchmarkQuoin: { revision: repositories.quoin.revision },
      qaCorpus: { executableCases: 0, reportingCases: 0, totalCases: 0 },
    },
    requiredCapabilities: ["metrics_envelope"],
    toolchains: { node: "22.15.0", rust: "1.94.1", python: "3.10.12" },
    artifacts: {
      "source.txt": sha256("old artifact"),
      "corpus/source.txt": sha256("old corpus artifact"),
    },
    timeouts: Object.fromEntries(
      ["case", "corpus", "tier1", "tier2", "install", "quoin", "span"].map(
        (name) => [`${name}Milliseconds`, 1000],
      ),
    ),
  };
  return { roots, base };
}

let passed = 0;
function check(name, action) {
  const scratch = mkdtempSync(join(tmpdir(), "quoin-relock-selftest-"));
  try {
    action({ ...fixture(scratch), scratch });
    passed += 1;
    console.log(`ok ${name}`);
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}

// TC-1589 / FR-043-AC-32
check(
  "exact candidate preserves policy and historical identities",
  ({ base, roots }) => {
    const snapshot = structuredClone(base);
    const candidate = prepareCandidate(base, roots);
    assert.deepEqual(base, snapshot);
    assert.deepEqual(candidate.repositories, base.repositories);
    for (const field of ["qaExternalQuoin", "quireBenchmarkQuoin"])
      assert.deepEqual(candidate.cohorts[field], base.cohorts[field]);
    for (const field of ["requiredCapabilities", "toolchains", "timeouts"])
      assert.deepEqual(candidate[field], base[field]);
    assert.deepEqual(candidate.cohorts.qaCorpus, {
      executableCases: 1,
      reportingCases: 1,
      totalCases: 2,
    });
    assert.equal(
      candidate.artifacts["source.txt"],
      sha256(readFileSync(join(roots.quoin, "source.txt"))),
    );
    assert.ok(candidate.artifacts["scripts/verification-relock.mjs"]);
    assert.deepEqual(prepareCandidate(base, roots), candidate);
  },
);

// TC-1590 / FR-043-AC-32: each incompatible input is rejected independently.
for (const [name, mutate, expected] of [
  ["missing root", ({ roots }) => delete roots.quire, /explicit absolute root/],
  [
    "dirty source",
    ({ roots }) => put(roots.quire, "dirty", "uncommitted"),
    /checkout is dirty/,
  ],
  [
    "unpublished source",
    ({ roots }) =>
      git(roots.quire, "update-ref", "-d", "refs/remotes/origin/main"),
    /not reachable/,
  ],
  [
    "wrong remote",
    ({ roots }) =>
      git(
        roots.quire,
        "remote",
        "set-url",
        "origin",
        "https://github.com/other/quire",
      ),
    /origin.*does not equal/,
  ],
  [
    "Cargo declared pin",
    ({ roots }) => {
      put(roots["quire-cli"], "Cargo.toml", "[dependencies]\n");
      commit(roots["quire-cli"]);
    },
    /Cargo.toml does not pin/,
  ],
  [
    "Cargo comment cannot impersonate the selected revision",
    ({ roots, base }) => {
      put(
        roots["quire-cli"],
        "Cargo.toml",
        `[dependencies]\nquire-rs = { git = "https://github.com/agent-ix/quire", rev = "${"b".repeat(40)}" }\n# expected: rev = "${base.repositories.quire.revision}"\n`,
      );
      commit(roots["quire-cli"]);
    },
    /Cargo.toml does not pin/,
  ],
  [
    "Cargo resolved pin",
    ({ roots }) => {
      put(roots["quire-cli"], "Cargo.lock", "");
      commit(roots["quire-cli"]);
    },
    /Cargo.lock does not select/,
  ],
  [
    "vendored hash drift",
    ({ roots }) => {
      put(roots.quoin, "src/quire/schemas/coverage-v1.schema.json", "{}\n");
      commit(roots.quoin);
    },
    /vendored schema hash drift/,
  ],
  [
    "contract comment cannot impersonate exported source revision",
    ({ roots, base }) => {
      const path = join(roots.quoin, "src/quire/contract.ts");
      const source = readFileSync(path, "utf8").replace(
        base.repositories.quire.revision,
        "b".repeat(40),
      );
      writeFileSync(
        path,
        `// prior sourceRevision: "${base.repositories.quire.revision}"\n${source}`,
      );
      commit(roots.quoin);
    },
    /not a local commit/,
  ],
  [
    "contract comment cannot impersonate an exported schema hash",
    ({ roots }) => {
      const path = join(roots.quoin, "src/quire/contract.ts");
      const hash = sha256(
        readFileSync(
          join(roots.quoin, "src/quire/schemas/assurance-v1.schema.json"),
        ),
      ).slice(7);
      const source = readFileSync(path, "utf8").replace(hash, "b".repeat(64));
      writeFileSync(
        path,
        `// "assurance-v1.schema.json": "${hash}"\n${source}`,
      );
      commit(roots.quoin);
    },
    /vendored schema hash drift/,
  ],
  [
    "git-object schema drift",
    ({ roots }) => {
      put(roots.quire, "schemas/output/coverage-v1.schema.json", "{}\n");
      const next = commit(roots.quire);
      for (const file of ["Cargo.toml", "Cargo.lock"]) {
        const path = join(roots["quire-cli"], file);
        writeFileSync(
          path,
          readFileSync(path, "utf8").replaceAll(/[0-9a-f]{40}/g, next),
        );
      }
      commit(roots["quire-cli"]);
    },
    /differs from engine git object/,
  ],
  [
    "QA gitlink mismatch",
    ({ roots }) => {
      put(roots["qa-corpus"], "next", "next");
      commit(roots["qa-corpus"]);
    },
    /corpus gitlink/,
  ],
  [
    "historical cohort missing",
    ({ base }) => (base.cohorts.qaExternalQuoin.revision = "a".repeat(40)),
    /not a local commit/,
  ],
  [
    "assume-unchanged artifact bytes cannot impersonate committed source",
    ({ roots }) => {
      git(roots.quoin, "update-index", "--assume-unchanged", "source.txt");
      put(roots.quoin, "source.txt", "not committed\n");
    },
    /artifact.*git object/,
  ],
  [
    "assume-unchanged corpus bytes cannot impersonate selected gitlink source",
    ({ roots }) => {
      git(
        roots["qa-corpus"],
        "update-index",
        "--assume-unchanged",
        "source.txt",
      );
      put(roots["qa-corpus"], "source.txt", "not committed\n");
    },
    /artifact.*git object/,
  ],
  [
    "new relock tool artifacts must also equal committed source",
    ({ roots }) => {
      git(
        roots.quoin,
        "update-index",
        "--assume-unchanged",
        "scripts/verification-relock.mjs",
      );
      put(roots.quoin, "scripts/verification-relock.mjs", "not committed\n");
    },
    /artifact.*git object/,
  ],
  [
    "hidden non-artifact inventory input cannot change locked population",
    ({ roots }) => {
      git(
        roots["qa-corpus"],
        "update-index",
        "--assume-unchanged",
        "inventory.json",
      );
      put(
        roots["qa-corpus"],
        "inventory.json",
        '{"cases":[{"mode":"reporting"},{"mode":"reporting"}]}\n',
      );
    },
    /tracked source.*git object/,
  ],
  [
    "skip-worktree source bytes cannot impersonate a selected producer",
    ({ roots }) => {
      git(roots.quire, "update-index", "--skip-worktree", "source.txt");
      put(roots.quire, "source.txt", "not committed\n");
    },
    /tracked source.*git object/,
  ],
  [
    "undersized budget",
    ({ roots }) => {
      put(roots["qa-corpus"], "bounds.py", "print('{\"cases\":[{},{}]}')\n");
      const next = commit(roots["qa-corpus"]);
      git(join(roots.quoin, "corpus"), "fetch", roots["qa-corpus"]);
      git(join(roots.quoin, "corpus"), "checkout", "--detach", next);
      git(
        join(roots.quoin, "corpus"),
        "update-ref",
        "refs/remotes/origin/main",
        next,
      );
      commit(roots.quoin);
    },
    /cover the locked per-case budget/,
  ],
]) {
  check(name, (fixture) => {
    mutate(fixture);
    assert.throws(
      () => prepareCandidate(fixture.base, fixture.roots),
      expected,
    );
  });
}

// TC-1591 / FR-043-AC-33
check(
  "new candidate only; existing output and inputs survive",
  ({ base, roots, scratch }) => {
    const candidate = prepareCandidate(base, roots);
    const output = join(scratch, "candidate.json");
    writeCandidate(output, candidate);
    const bytes = readFileSync(output);
    assert.throws(() => writeCandidate(output, { changed: true }), /EEXIST/);
    assert.deepEqual(readFileSync(output), bytes);
    for (const name of SOURCE_NAMES)
      assertRepository(name, roots[name], candidate.repositories[name]);
    assert.deepEqual(
      candidate.cohorts.qaExternalQuoin,
      base.cohorts.qaExternalQuoin,
    );
  },
);

check(
  "real command creates a candidate and refuses incompatible inputs before output",
  ({ base, roots, scratch }) => {
    const input = join(scratch, "input-lock.json");
    const output = join(scratch, "command-candidate.json");
    writeFileSync(input, JSON.stringify(base));
    const command = join(
      dirname(fileURLToPath(import.meta.url)),
      "verification-relock.mjs",
    );
    const args = [
      command,
      "--lock",
      input,
      "--out",
      output,
      ...SOURCE_NAMES.flatMap((name) => ["--root", `${name}=${roots[name]}`]),
    ];
    execFileSync(process.execPath, args, { stdio: "pipe" });
    assert.deepEqual(
      JSON.parse(readFileSync(output, "utf8")),
      prepareCandidate(base, roots),
    );
    assert.throws(() =>
      execFileSync(process.execPath, args, { stdio: "pipe" }),
    );
    put(roots.quire, "dirty", "dirty");
    const refusedOutput = join(scratch, "refused-candidate.json");
    args[4] = refusedOutput;
    assert.throws(() =>
      execFileSync(process.execPath, args, { stdio: "pipe" }),
    );
    assert.throws(() => readFileSync(refusedOutput), /ENOENT/);
    assert.deepEqual(JSON.parse(readFileSync(input, "utf8")), base);
  },
);

// TC-1592 / FR-043-AC-33
check("rollback still refuses and names the remedy", ({ base, roots }) => {
  const before = base.repositories.quire.revision;
  put(roots.quire, "next", "next");
  const next = commit(roots.quire);
  git(roots.quire, "checkout", "--detach", before);
  assert.throws(
    () =>
      assertRepository(
        "quire",
        roots.quire,
        { ...base.repositories.quire, revision: next },
        { allowEvidenceOverlay: true },
      ),
    (error) =>
      /does not descend/.test(error.message) &&
      /make verification-relock/.test(error.message),
  );
});

assert.throws(() => parseArguments([]), /--out/);
assert.throws(
  () => parseArguments(["--out", "candidate.json"]),
  /explicit --root/,
);
assert.throws(
  () => parseArguments(["--root", "quire=relative"]),
  /absolute path/,
);
assert.throws(() => parseArguments(["--unknown", "value"]), /unknown argument/);
assert.throws(
  () =>
    parseArguments(["--root", "quire=/tmp/one", "--root", "quire=/tmp/two"]),
  /duplicate/,
);

check(
  "unchanged assume-unchanged files remain valid; contract AST rejects ambiguity",
  ({ roots, base }) => {
    git(roots.quoin, "update-index", "--assume-unchanged", "source.txt");
    prepareCandidate(base, roots);
    const source = readFileSync(
      join(roots.quoin, "src/quire/contract.ts"),
      "utf8",
    );
    for (const invalid of [
      source.replace("export const", "export let"),
      source.replace("sourceRevision:", "['sourceRevision']:"),
      source.replace("sourceRevision:", "...{}, sourceRevision:"),
      source.replace(
        "sourceRevision:",
        `sourceRevision: "${"b".repeat(40)}", sourceRevision:`,
      ),
      source.replace(
        "hashes: {",
        'hashes: { ["coverage-v1.schema.json"]: "bad", ',
      ),
      source + source,
    ])
      assert.throws(() => parseContract(invalid));
    const actual = parseContract(
      `// sourceRevision: "${"b".repeat(40)}"\n${source}`,
    );
    assert.equal(actual.revision, base.repositories.quire.revision);
  },
);
check(
  "ignored working-tree inputs never enter the committed inventory snapshot",
  ({ roots, base }) => {
    put(roots["qa-corpus"], "extra.json", '[{"mode":"reporting"}]\n');
    assert.equal(git(roots["qa-corpus"], "status", "--porcelain"), "");
    assert.deepEqual(prepareCandidate(base, roots).cohorts.qaCorpus, {
      executableCases: 1,
      reportingCases: 1,
      totalCases: 2,
    });
  },
);

console.log(
  `${passed} verification relock integration checks and 5 argument checks passed`,
);
