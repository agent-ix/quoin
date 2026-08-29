#!/usr/bin/env node

import { auditToolDrift, repositoryFiles } from "./check-tool-drift.mjs";

const base = repositoryFiles();
const mutations = [
  [
    "moving npm range",
    "package.json",
    (s) => s.replace('"ajv": "8.20.0"', '"ajv": "^8.20.0"'),
    /not exact/,
  ],
  [
    "lock disagreement",
    "pnpm-lock.yaml",
    (s) => s.replace("specifier: 8.20.0", "specifier: ^8.20.0"),
    /lock specifier drift/,
  ],
  [
    "ambient Make pnpm",
    "Makefile",
    (s) => `${s}\nambient-pnpm:\n\tpnpm run build\n`,
    /may not execute ambient pnpm/,
  ],
  [
    "moving action",
    ".github/workflows/install-smoke.yml",
    (s) => s.replace(/actions\/checkout@[0-9a-f]{40}/, "actions/checkout@v7"),
    /moving action/,
  ],
  [
    "moving runner",
    ".github/workflows/install-smoke.yml",
    (s) => s.replace("ubuntu-24.04", "ubuntu-latest"),
    /latest runner/,
  ],
  [
    "broad Node",
    ".github/workflows/release-drift.yml",
    (s) => s.replace("node-version: 22.15.0", "node-version: 22"),
    /non-exact Node/,
  ],
  [
    "wrong governed CLI revision",
    ".github/workflows/build-test.yml",
    (s) =>
      s.replace(
        /ref: [0-9a-f]{40}/,
        "ref: 0000000000000000000000000000000000000000",
      ),
    /must equal verification-stack quire-cli revision/,
  ],
  [
    "workflow pnpm disagreement",
    ".github/workflows/build-test.yml",
    (s) => s.replace("version: 11.20.0", "version: 11.19.0"),
    /pnpm version must equal/,
  ],
  [
    "workflow Node disagreement",
    ".github/workflows/build-test.yml",
    (s) => s.replace("node-version: 22.15.0", "node-version: 22.14.0"),
    /Node version must equal/,
  ],
  [
    "workflow Rust disagreement",
    ".github/workflows/build-test.yml",
    (s) => s.replace("toolchain: 1.94.1", "toolchain: 1.93.1"),
    /Rust version must equal/,
  ],
  [
    "vendored contract provenance disagreement",
    "src/quire/contract.ts",
    (s) =>
      s.replace(
        /sourceRevision: "[0-9a-f]{40}"/,
        'sourceRevision: "0000000000000000000000000000000000000000"',
      ),
    /vendored Quire contract source revision must equal/,
  ],
  [
    "reviewed evidence provenance disagreement",
    "bench/span-breadth-v1-labels.json",
    (s) =>
      s.replace(
        /"sourceRevision": "[0-9a-f]{40}"/,
        '"sourceRevision": "0000000000000000000000000000000000000000"',
      ),
    /CLI provenance must equal/,
  ],
  [
    "missing benchmark corpus cohort",
    "quality/verification-stack-lock.json",
    (s) => s.replace('"quireBenchmarkQuoin"', '"unguardedQuoin"'),
    /benchmark Quoin corpus must be locked/,
  ],
  [
    "collapsed benchmark corpus identity",
    "scripts/verification-stack.mjs",
    (s) => s.replace('sources["quoin-benchmark-corpus"]', "sources.quoin"),
    /separate the Quoin benchmark corpus identity/,
  ],
  [
    "unguarded Quire evidence overlay",
    "scripts/verification-stack.mjs",
    (s) => s.replace('quire: ["spec/evidence/measurements"]', 'quire: [""]'),
    /quire verification source must allow only governed measurement overlays/,
  ],
  [
    "unguarded QA evidence overlay",
    "scripts/verification-stack.mjs",
    (s) =>
      s.replace(
        '"qa-corpus": ["spec/evidence/measurements"]',
        '"qa-corpus": [""]',
      ),
    /qa-corpus verification source must allow only governed measurement overlays/,
  ],
  [
    "unguarded span-breadth evidence overlay",
    "scripts/verify-span-breadth.mjs",
    (s) =>
      s.replace(
        'allowedOverlayPaths: ["spec/evidence/measurements"]',
        'allowedOverlayPaths: [""]',
      ),
    /span-breadth Quire source must allow only governed measurement overlays/,
  ],
  [
    "unhashed span-breadth verifier",
    "quality/verification-stack-lock.json",
    (s) =>
      s.replace(
        '"scripts/verify-span-breadth.mjs"',
        '"unguarded-span-verifier"',
      ),
    /scripts\/verify-span-breadth\.mjs must be artifact-digest guarded/,
  ],
  [
    "unhashed verification-stack selftest",
    "quality/verification-stack-lock.json",
    (s) =>
      s.replace(
        '"scripts/verification-stack-selftest.mjs"',
        '"unguarded-stack-selftest"',
      ),
    /scripts\/verification-stack-selftest\.mjs must be artifact-digest guarded/,
  ],
  [
    "skipped verification-stack selftest",
    "scripts/verification-stack.mjs",
    (s) =>
      s.replace(
        '["pnpm", "run", "test:verification-stack"]',
        '["pnpm", "run", "skipped:verification-stack"]',
      ),
    /canonical campaign must run verification-stack selftests/,
  ],
  [
    "debug canonical build",
    "scripts/verification-stack.mjs",
    (s) => s.replace('      "--release",\n', ""),
    /release profile and locked resolution/,
  ],
  [
    "unlocked canonical build",
    "scripts/verification-stack.mjs",
    (s) => s.replace('      "--locked",\n', ""),
    /release profile and locked resolution/,
  ],
  [
    "missing attested build profile",
    "scripts/verification-stack.mjs",
    (s) => s.replace('      buildProfile: "release",\n', ""),
    /attestation and Tier-1 must agree on release build profile/,
  ],
  [
    "Tier-1 ignores build profile",
    "scripts/bench-tier1.mjs",
    (s) =>
      s.replace(
        'value.buildProfile !== "release"',
        'value.ignoredProfile !== "release"',
      ),
    /attestation and Tier-1 must agree on release build profile/,
  ],
  [
    "shared-workspace Tier-1 runtime",
    "scripts/verification-stack.mjs",
    (s) => s.replace('"--quoin",\n      isolatedQuoin,', ""),
    /deploy and select a frozen isolated Quoin runtime/,
  ],
  [
    "moving image",
    "Dockerfile",
    (s) => s.replace(/@sha256:[0-9a-f]{64}/, ""),
    /unpinned base image/,
  ],
  [
    "moving smoke tool",
    "smoke/run.sh",
    (s) => s.replace("0.149.1", "latest"),
    /moving latest/,
  ],
  [
    "npx execution",
    "Makefile",
    (s) => `${s}\n\tnpx vitest\n`,
    /may not use npx/,
  ],
  [
    "unversioned dlx",
    "package.json",
    (s) => s.replace("@agent-ix/js-deps@0.2.4", "@agent-ix/js-deps"),
    /dlx package is not exact/,
  ],
  [
    "PATH-only producer",
    "src/quire/exec.ts",
    (s) => s.replaceAll("QUOIN_QUIRE", "UNGUARDED_SELECTOR"),
    /no explicit binary selector/,
  ],
  [
    "unhashed producer",
    "src/quire/exec.ts",
    (s) => s.replaceAll("QUOIN_EXPECTED_QUIRE_SHA256", "UNGUARDED_DIGEST"),
    /no executable digest guard/,
  ],
];

let failures = 0;
for (const [name, path, mutate, expected] of mutations) {
  const files = { ...base, [path]: mutate(base[path]) };
  const errors = auditToolDrift(files).join("\n");
  if (!expected.test(errors)) {
    console.error(`tool-drift mutation was not rejected: ${name}\n${errors}`);
    failures += 1;
  }
}
if (failures) process.exit(1);
console.log(
  `tool-drift-selftest: ${mutations.length}/${mutations.length} drift mutations rejected by class`,
);
