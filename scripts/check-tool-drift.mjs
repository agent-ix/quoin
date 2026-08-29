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
