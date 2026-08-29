#!/usr/bin/env node

import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parse as parseYaml } from "yaml";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const EXACT_VERSION = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;
const FULL_SHA = /^[0-9a-f]{40}$/;
const OCI = /@sha256:[0-9a-f]{64}$/;

export function auditToolDrift(files) {
  const errors = [];
  const pkg = JSON.parse(files["package.json"]);
  const stackLock = JSON.parse(files["quality/verification-stack-lock.json"]);
  if (!FULL_SHA.test(stackLock.cohorts?.quireBenchmarkQuoin?.revision ?? "")) {
    errors.push("Quire benchmark Quoin corpus must be locked to a full SHA");
  }
  if (!/^pnpm@\d+\.\d+\.\d+$/.test(pkg.packageManager ?? "")) {
    errors.push("packageManager must pin an exact pnpm version");
  }
  for (const section of [
    "dependencies",
    "devDependencies",
    "optionalDependencies",
  ]) {
    for (const [name, version] of Object.entries(pkg[section] ?? {})) {
      if (!EXACT_VERSION.test(version)) {
        errors.push(`package.json ${section}.${name} is not exact: ${version}`);
      }
    }
  }
  if (!EXACT_VERSION.test(files[".node-version"].trim())) {
    errors.push(".node-version must be exact x.y.z");
  }
  if (!/save-exact=true/.test(files[".npmrc"])) {
    errors.push(".npmrc must enforce save-exact=true");
  }
  if (!/^PNPM := corepack pnpm$/m.test(files["Makefile"])) {
    errors.push("Makefile must resolve pnpm through Corepack");
  }
  if (/^\s*@?pnpm\s/m.test(files["Makefile"])) {
    errors.push("Makefile recipes may not execute ambient pnpm");
  }
  if (/command\s+-v\s+quire|^QUIRE\s*\?=/m.test(files["Makefile"])) {
    errors.push("Makefile may not discover or default Quire through PATH");
  }
  if (
    !/^test:\n\tnode scripts\/verification-stack\.mjs$/m.test(files["Makefile"])
  ) {
    errors.push("bare make test must run the canonical verification stack");
  }
  if (
    !/^test-with-quire: require-quire build validate check-version$/m.test(
      files["Makefile"],
    )
  ) {
    errors.push(
      "the inner test gate must require an explicit absolute Quire path",
    );
  }
  if (
    !/test -x "\$\(QUIRE\)"/.test(files["Makefile"]) ||
    !/\*\/quire\) ;; \*\) echo "QUIRE must name an executable called quire/.test(
      files["Makefile"],
    )
  ) {
    errors.push("the explicit Quire path must be executable and named quire");
  }
  if (
    !/PATH="\$\(dir \$\(QUIRE\)\):\$\$PATH" QUIRE="\$\(QUIRE\)" \$\(PNPM\) run test/.test(
      files["Makefile"],
    )
  ) {
    errors.push(
      "the inner test gate must default Quoin and contract tests to one Quire binary",
    );
  }
  for (const target of [
    "battletest",
    "battletest-update",
    "bench-tier1-experimental",
    "evidence-audit",
    "validate",
  ]) {
    if (
      !new RegExp(`^${target}:.*\\brequire-quire\\b`, "m").test(
        files["Makefile"],
      )
    ) {
      errors.push(`${target} must fail closed without an explicit Quire path`);
    }
  }
  if (
    !/QUOIN_QUIRE="\$\(QUIRE\)" node bin\/quoin\.js evidence audit/.test(
      files["Makefile"],
    )
  ) {
    errors.push("evidence-audit must pass its explicit Quire path to Quoin");
  }

  const pnpmLock = parseYaml(files["pnpm-lock.yaml"]);
  const importer = pnpmLock?.importers?.["."] ?? {};
  for (const section of [
    "dependencies",
    "devDependencies",
    "optionalDependencies",
  ]) {
    for (const [name, version] of Object.entries(pkg[section] ?? {})) {
      if (importer[section]?.[name]?.specifier !== version) {
        errors.push(`pnpm lock specifier drift for ${section}.${name}`);
      }
    }
  }

  for (const [path, text] of Object.entries(files)) {
    if (!path.startsWith(".github/workflows/")) continue;
    for (const match of text.matchAll(/^\s*(?:-\s*)?uses:\s*\S+@([^\s#]+)/gm)) {
      if (!FULL_SHA.test(match[1]))
        errors.push(`${path} uses moving action ref @${match[1]}`);
    }
    if (/runs-on:\s*\S*latest/.test(text))
      errors.push(`${path} uses a latest runner`);
    for (const match of text.matchAll(/node-version:\s*['"]?([^\s'"]+)/g)) {
      if (!EXACT_VERSION.test(match[1]))
        errors.push(`${path} has non-exact Node ${match[1]}`);
    }
  }

  const buildWorkflow = parseYaml(files[".github/workflows/build-test.yml"]);
  const buildSteps = Object.values(buildWorkflow?.jobs ?? {}).flatMap(
    (job) => job?.steps ?? [],
  );
  const governedCliCheckout = buildSteps.find(
    (step) => step?.with?.repository === "agent-ix/quire-cli",
  );
  const governedTestStep = buildSteps.find(
    (step) => step?.name === "Test with exact governed Quire",
  );
  if (!String(governedTestStep?.run ?? "").includes("make test-with-quire")) {
    errors.push(
      "build-test must use the explicit non-recursive Quire test gate",
    );
  }
  if (
    governedCliCheckout?.with?.ref !==
    stackLock.repositories?.["quire-cli"]?.revision
  ) {
    errors.push(
      "build-test governed Quire checkout must equal verification-stack quire-cli revision",
    );
  }
  const declaredPnpm = pkg.packageManager?.replace(/^pnpm@/, "");
  for (const step of buildSteps.filter((candidate) =>
    String(candidate?.uses ?? "").startsWith("pnpm/action-setup@"),
  )) {
    if (String(step?.with?.version) !== declaredPnpm) {
      errors.push(
        "build-test pnpm version must equal package.json packageManager",
      );
    }
  }

  if (
    !files["src/quire/contract.ts"].includes(
      `sourceRevision: "${stackLock.repositories.quire.revision}"`,
    )
  ) {
    errors.push(
      "vendored Quire contract source revision must equal verification-stack engine revision",
    );
  }
  if (
    !files["scripts/verification-stack.mjs"].includes(
      '"deploy",\n        "--prod",\n        "--legacy",\n        "--frozen-lockfile"',
    ) ||
    !files["scripts/verification-stack.mjs"].includes(
      '"--quoin",\n      isolatedQuoin',
    )
  ) {
    errors.push(
      "canonical Tier-1 must deploy and select a frozen isolated Quoin runtime",
    );
  }
  if (
    !files["scripts/verification-stack.mjs"].includes(
      "toolchains: structuredClone(lock.toolchains)",
    )
  ) {
    errors.push("canonical attestation must carry locked toolchain identities");
  }
  if (
    !files["scripts/verification-stack.mjs"].includes(
      '"build",\n      "--release",\n      "--locked",',
    )
  ) {
    errors.push(
      "canonical Quire build must use release profile and locked resolution",
    );
  }
  if (
    !files["scripts/verification-stack.mjs"].includes(
      'buildProfile: "release"',
    ) ||
    !files["scripts/bench-tier1.mjs"].includes(
      'value.buildProfile !== "release"',
    )
  ) {
    errors.push(
      "canonical attestation and Tier-1 must agree on release build profile",
    );
  }
  if (
    !files["scripts/verification-stack.mjs"].includes(
      'sources["quoin-benchmark-corpus"]',
    )
  ) {
    errors.push(
      "canonical attestation must separate the Quoin benchmark corpus identity",
    );
  }
  for (const producer of ["quire", "qa-corpus"]) {
    const declaration =
      producer === "quire"
        ? 'quire: ["spec/evidence/measurements"]'
        : '"qa-corpus": ["spec/evidence/measurements"]';
    if (!files["scripts/verification-stack.mjs"].includes(declaration)) {
      errors.push(
        `${producer} verification source must allow only governed measurement overlays`,
      );
    }
  }
  if (
    !files["scripts/verify-span-breadth.mjs"].includes(
      'allowedOverlayPaths: ["spec/evidence/measurements"]',
    )
  ) {
    errors.push(
      "span-breadth Quire source must allow only governed measurement overlays",
    );
  }
  for (const path of [
    "scripts/verify-span-breadth.mjs",
    "scripts/verification-stack-selftest.mjs",
    "scripts/battletest.mjs",
    "scripts/lib/tier2-baseline.mjs",
  ]) {
    if (!stackLock.artifacts?.[path]) {
      errors.push(`${path} must be artifact-digest guarded`);
    }
  }
  if (
    !files["scripts/verification-stack.mjs"].includes(
      '["pnpm", "run", "test:verification-stack"]',
    )
  ) {
    errors.push("canonical campaign must run verification-stack selftests");
  }
  if (
    !files["scripts/verification-stack.mjs"].includes(
      '["test-with-quire", `QUIRE=${binary}`]',
    )
  ) {
    errors.push("canonical campaign must enter the explicit Quire test gate");
  }
  if (
    !files["scripts/bench-tier1.mjs"].includes(
      "canonical runs require --quoin <isolated executable>",
    )
  ) {
    errors.push("canonical Tier-1 must require an explicit Quoin executable");
  }
  for (const path of [
    "bench/span-breadth-v1-labels.json",
    "bench/guidance-evaluator-contract-v1.json",
    "bench/guidance-independent-review-v1.json",
  ]) {
    const reviewed = JSON.parse(files[path]);
    const producer = reviewed.producer ?? reviewed.populationSource?.producer;
    if (
      producer?.cli?.sourceRevision !==
      stackLock.repositories["quire-cli"].revision
    ) {
      errors.push(`${path} CLI provenance must equal verification-stack lock`);
    }
    if (
      producer?.engine?.sourceRevision !== stackLock.repositories.quire.revision
    ) {
      errors.push(
        `${path} engine provenance must equal verification-stack lock`,
      );
    }
    if (
      reviewed.repositories &&
      reviewed.repositories?.["quire-rs"]?.revision !==
        stackLock.repositories.quire.revision
    ) {
      errors.push(
        `${path} Quire repository must equal verification-stack lock`,
      );
    }
    if (
      reviewed.repositories &&
      reviewed.repositories?.quoin?.revision !==
        stackLock.repositories.quoin.revision
    ) {
      errors.push(
        `${path} Quoin repository must equal verification-stack lock`,
      );
    }
  }
  for (const step of buildSteps.filter((candidate) =>
    String(candidate?.uses ?? "").startsWith("actions/setup-node@"),
  )) {
    if (String(step?.with?.["node-version"]) !== stackLock.toolchains?.node) {
      errors.push(
        "build-test Node version must equal verification-stack toolchain",
      );
    }
  }
  for (const step of buildSteps.filter((candidate) =>
    String(candidate?.uses ?? "").startsWith("dtolnay/rust-toolchain@"),
  )) {
    if (String(step?.with?.toolchain) !== stackLock.toolchains?.rust) {
      errors.push(
        "build-test Rust version must equal verification-stack toolchain",
      );
    }
  }

  for (const path of ["Dockerfile", "smoke/Dockerfile"]) {
    for (const line of files[path]
      .split("\n")
      .filter((row) => /^FROM\s+/.test(row))) {
      if (!OCI.test(line.trim()))
        errors.push(`${path} has an unpinned base image: ${line}`);
    }
  }
  for (const path of [
    "smoke/Dockerfile",
    "smoke/run.sh",
    "smoke/entrypoint.sh",
  ]) {
    if (/=(?:"?\$\{[^}]+:-)?latest\b/.test(files[path])) {
      errors.push(`${path} executes a moving latest version`);
    }
  }

  const executableSurfaces = [files["package.json"], files["Makefile"]].join(
    "\n",
  );
  if (/(?:^|\s)npx\s/.test(executableSurfaces))
    errors.push("canonical scripts may not use npx");
  for (const match of executableSurfaces.matchAll(/pnpm\s+dlx\s+([^\s"]+)/g)) {
    if (!/@\d+\.\d+\.\d+(?:-|$)/.test(match[1])) {
      errors.push(`pnpm dlx package is not exact: ${match[1]}`);
    }
  }
  if (!/QUOIN_QUIRE/.test(files["src/quire/exec.ts"])) {
    errors.push(
      "Quoin's Quire subprocess boundary has no explicit binary selector",
    );
  }
  if (!/QUOIN_EXPECTED_QUIRE_SHA256/.test(files["src/quire/exec.ts"])) {
    errors.push(
      "Quoin's Quire subprocess boundary has no executable digest guard",
    );
  }
  return errors;
}

export function repositoryFiles(root = ROOT) {
  const paths = [
    "package.json",
    "quality/verification-stack-lock.json",
    "pnpm-lock.yaml",
    ".node-version",
    ".npmrc",
    "Makefile",
    "Dockerfile",
    "smoke/Dockerfile",
    "smoke/run.sh",
    "smoke/entrypoint.sh",
    "src/quire/exec.ts",
    "src/quire/contract.ts",
    "scripts/verification-stack.mjs",
    "scripts/bench-tier1.mjs",
    "scripts/verify-span-breadth.mjs",
    "scripts/verification-stack-selftest.mjs",
    "scripts/battletest.mjs",
    "scripts/lib/tier2-baseline.mjs",
    "bench/span-breadth-v1-labels.json",
    "bench/guidance-evaluator-contract-v1.json",
    "bench/guidance-independent-review-v1.json",
    ".github/workflows/build-test.yml",
    ".github/workflows/codeql.yml",
    ".github/workflows/install-smoke.yml",
    ".github/workflows/release-drift.yml",
    ".github/workflows/release.yml",
  ];
  return Object.fromEntries(
    paths.map((path) => [path, readFileSync(join(root, path), "utf8")]),
  );
}

if (
  resolve(process.argv[1] ?? "") === resolve(fileURLToPath(import.meta.url))
) {
  const errors = auditToolDrift(repositoryFiles());
  if (errors.length > 0) {
    for (const error of errors) console.error(`tool-drift: ${error}`);
    process.exit(1);
  }
  console.log(
    "tool-drift: all executable dependency surfaces are immutable or explicitly noncanonical",
  );
}
