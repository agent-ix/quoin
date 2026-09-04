#!/usr/bin/env node
// Stage the Filament-module payload for npm packaging.
//
// Non-destructive: copies the module payload from the inner Python package
// directory up to the repository root, so the published npm tarball IS the
// module root — manifest.yaml at the top, with every schema reference resolving
// relative to it. The inner directory stays the single source of truth and the
// staged copies are gitignored.
//
// `--clean` (run from `postpack`) removes the staged copies again. Leaving them
// behind puts a manifest.yaml at the repository root, which every Filament tool
// then discovers as a SECOND module: quire's scoped module search finds the
// root "module", stops merging the installed module set, and an unrelated
// `quire validate` in this repository fails with "no archetype registered".
//
// Node built-ins only, zero dependencies.
import {
  cpSync,
  existsSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const inner = "{{ cookiecutter.package_name }}";

if (!existsSync(join(root, inner, "manifest.yaml"))) {
  console.error(
    `stage-npm: ${inner}/manifest.yaml is missing; there is no module payload to stage.`,
  );
  process.exit(1);
}

const PAYLOAD = [
  "manifest.yaml",
  "schemas",
  "skeletons",
{%- if cookiecutter.module_kind in ("artifact", "mixed") %}
  "mappings.yaml",
  "examples",
{%- endif %}
];

if (process.argv.includes("--clean")) {
  for (const item of PAYLOAD) {
    const target = join(root, item);
    if (existsSync(target)) {
      rmSync(target, { recursive: true, force: true });
      console.log(`stage-npm: removed staged ${item}`);
    }
  }
  process.exit(0);
}

for (const item of PAYLOAD) {
  const from = join(root, inner, item);
  if (!existsSync(from)) {
    console.error(
      `stage-npm: ${inner}/${item} is missing. Run \`make bootstrap\` before packing:` +
        " the emitted schemas are produced from typespec/main.tsp, not committed by hand.",
    );
    process.exit(1);
  }
  const to = join(root, item);
  rmSync(to, { recursive: true, force: true });
  cpSync(from, to, { recursive: true });
  console.log(`stage-npm: ${inner}/${item} -> ${item}`);
}

// Version sync: when packing from a CI tag (vX.Y.Z), stamp package.json so the
// tarball is named and published at the tag version. A no-op locally.
const tag = (process.env.GITHUB_REF_NAME ?? "").match(
  /^v?(\d+\.\d+\.\d+(?:[-+].+)?)$/,
);
if (tag) {
  const pkgPath = join(root, "package.json");
  const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));
  if (pkg.version !== tag[1]) {
    pkg.version = tag[1];
    writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + "\n");
    console.log(`stage-npm: version -> ${tag[1]}`);
  }
}
