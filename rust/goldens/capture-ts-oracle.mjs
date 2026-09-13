// Golden capture for quoin#381 (Stage 7 Rust port). Run once from the TS tree.
import { mkdirSync, mkdtempSync, writeFileSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { parse as parseYaml } from "yaml";
import { toGitUrl, normalizeSource, validateMarketplaceManifest, readRegistry, writeRegistry, upsertPlugin } from "@agent-ix/ts-plugin-kit";
import { originOrg, resolveOrg } from "../src/org.ts";
import { parseSourceArg, registryPath, readModuleName, installOptions } from "../src/plugins.ts";
import { QuoinConfigSchema } from "../src/config-schema.ts";
import { ixHome, filamentModulesDir } from "../src/catalog.ts";

const out = {};
const tryv = (f) => { try { return { ok: true, value: f() }; } catch (e) { return { ok: false, error: String(e?.message ?? e) }; } };

// --- originOrg -------------------------------------------------------------
const gitConfig = (url) => `[core]\n\trepositoryformatversion = 0\n[remote "origin"]\n\turl = ${url}\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n`;
const URLS = [
  "git@github.com:acme/widgets.git", "https://github.com/acme/widgets.git",
  "https://github.com/acme/widgets/", "git@github.com:acme/widgets",
  "https://git.example.com:8443/acme/widgets.git", "ssh://git@host:2222/acme/widgets.git",
  "https://gitlab.com/top/sub/widgets.git", "https://github.com/repo.git",
  "/srv/git/myrepo.git", "../sibling-repo", "file:///srv/git/myrepo.git",
  "~/repos/thing.git", "https://github.com", "justahostname", "https://github.com/acme//",
  "https://github.com/", "git@github.com:", "ssh://git@host/acme/widgets.git",
  "FILE:///srv/git/x.git", "user@host:org/repo.git", "https://host/a/b/c/d.git",
];
out.origin_org_urls = URLS.map((url) => ({ url, org: originOrg(gitConfig(url)) ?? null }));
out.origin_org_configs = [
  { label: "upstream before origin", config: `[remote "upstream"]\n\turl = git@github.com:wrong/repo.git\n[remote "origin"]\n\turl = git@github.com:right/repo.git\n` },
  { label: "section name uppercase", config: '[REMOTE "origin"]\n\turl = git@github.com:acme/w.git\n' },
  { label: "subsection name cased", config: '[remote "Origin"]\n\turl = git@github.com:acme/w.git\n' },
  { label: "url not an assignment", config: '[remote "origin"]\n\turlsomething\n' },
  { label: "no origin section", config: `[core]\n\trepositoryformatversion = 0\n[remote "upstream"]\n\turl = git@github.com:other/repo.git\n` },
  { label: "spaces in section header", config: '[ remote  "origin" ]\n\turl = git@github.com:acme/w.git\n' },
  { label: "urlfoo key", config: '[remote "origin"]\n\turlfoo = git@github.com:acme/w.git\n' },
  { label: "empty", config: "" },
].map((c) => ({ ...c, org: originOrg(c.config) ?? null }));

// --- parseSourceArg --------------------------------------------------------
out.parse_source_arg = [
  "path:/a/b", "github:agent-ix/x@v1", "github:agent-ix/x",
  "github:agent-ix/repo//pkg@v1.0.0", "github:agent-ix/repo//pkg",
  "package:foo@1.2.3", "package:foo", "package:@scope/foo@2.0.0", "package:@scope/foo",
  "./bare", "/abs/path", "github:agent-ix/repo//a/b@main",
].map((arg) => ({ arg, source: parseSourceArg(arg) }));

// --- toGitUrl --------------------------------------------------------------
out.to_git_url = [
  "agent-ix/quoin", "agent-ix/quoin.git", "https://github.com/a/b.git",
  "git@github.com:a/b.git", "  agent-ix/quoin  ", "ssh://git@h/a/b.git",
].map((raw) => ({ raw, url: toGitUrl(raw) }));

// --- normalizeSource -------------------------------------------------------
out.normalize_source = [
  { type: "github", repo: "a/b" }, { type: "github", repo: "" },
  { type: "github", repo: "-evil" }, { type: "git-subdir", url: "a/b", path: "p" },
  { type: "git-subdir", url: "a/b" }, { type: "path", path: "-x" },
  { type: "path", path: "" }, { type: "npm", package: "x" },
  { type: "bogus" }, {}, { type: "git", url: "u", ref: "-r" },
  { type: "url", url: "http://x" },
].map((source) => ({ source, result: tryv(() => normalizeSource(source)) }));

// --- marketplace manifest --------------------------------------------------
const defaults = parseYaml(readFileSync(new URL("../default-modules.yaml", import.meta.url), "utf8"));
out.default_modules_manifest = tryv(() => validateMarketplaceManifest(defaults));
out.validate_manifest = [
  null, {}, { schemaVersion: 2, entries: [] }, { schemaVersion: 1, entries: {} },
  { schemaVersion: 1, entries: [{ source: { type: "path", path: "p" } }] },
  { schemaVersion: 1, entries: [{ name: "", source: { type: "path", path: "p" } }] },
  { schemaVersion: 1, name: 7, entries: [] },
  { schemaVersion: 1, entries: [{ name: "a", source: { type: "path", path: "p" }, defaultEnabled: false }] },
].map((obj) => ({ input: obj, result: tryv(() => validateMarketplaceManifest(obj)) }));

// --- paths -----------------------------------------------------------------
out.paths = { registry_path: registryPath("/h"), modules_dir: filamentModulesDir("/h"), install_options: (() => { const o = installOptions("/h"); return { cacheRoot: o.cacheRoot, targetRoot: o.targetRoot, registryPath: o.registryPath, materialize: o.materialize }; })() };

// --- registry file format --------------------------------------------------
{
  const dir = mkdtempSync(join(tmpdir(), "quoin-381-reg-"));
  const path = join(dir, "registry.json");
  const rec = { name: "m", source: { type: "path", path: "/p" }, ref: undefined, sha: undefined, resolvedPath: "/p", targetPath: "/t/m", installedAt: "2026-01-02T03:04:05.678Z" };
  writeRegistry(path, upsertPlugin(readRegistry(path), rec));
  out.registry_file_bytes = readFileSync(path, "utf8");
  // A second record with every optional field PRESENT. The record above has
  // them all absent, so on its own it pins nothing about the ordering or
  // presence of `ref`, `sha` and `semantic` — the keys most likely to drift.
  const full = mkdtempSync(join(tmpdir(), "quoin-381-regfull-"));
  const fullPath = join(full, "registry.json");
  const recFull = { name: "m", source: { type: "git-subdir", url: "https://github.com/acme/widgets.git", path: "modules/m", ref: "v1.2.3", sha: "978111c6884ccc0f5e6ca100fb17be5361a98713" }, ref: "v1.2.3", sha: "978111c6884ccc0f5e6ca100fb17be5361a98713", resolvedPath: "/c/978111c/modules/m", targetPath: "/t/m", installedAt: "2026-01-02T03:04:05.678Z", semantic: { package: "acme.widgets", semanticCore: "quire-core@1", exports: { "Gadget": "sha256:bb", "Widget": "sha256:aa" } } };
  writeRegistry(fullPath, upsertPlugin(readRegistry(fullPath), recFull));
  out.registry_file_bytes_full = readFileSync(fullPath, "utf8");
  writeFileSync(path, "{ not json");
  out.registry_read_malformed = tryv(() => readRegistry(path));
  writeFileSync(path, '{"schemaVersion":1,"plugins":"nope"}');
  out.registry_read_plugins_not_array = tryv(() => readRegistry(path));
  out.registry_read_absent = readRegistry(join(dir, "nope.json"));
}

// --- readModuleName --------------------------------------------------------
{
  const mk = (name, nested) => { const root = mkdtempSync(join(tmpdir(), "quoin-381-mod-")); const base = root.split("/").pop(); const dir = nested ? join(root, base) : root; mkdirSync(dir, { recursive: true }); if (name !== undefined) writeFileSync(join(dir, "manifest.yaml"), `name: ${JSON.stringify(name)}\n`); return root; };
  out.read_module_name = [
    { label: "top-level", result: tryv(() => readModuleName(mk("spec-objects-business", false))) },
    { label: "nested <root>/<basename>", result: tryv(() => readModuleName(mk("nested-mod", true))) },
    { label: "absent", result: tryv(() => readModuleName(mkdtempSync(join(tmpdir(), "quoin-381-mod-")))) },
    { label: "non-string name", result: tryv(() => readModuleName(mk(123, false))) },
    { label: "empty name", result: tryv(() => readModuleName(mk("", false))) },
  ].map((c) => ({ ...c, result: { ok: c.result.ok, value: c.result.value, error: c.result.error ? c.result.error.replace(/\/tmp\/[^ ]*/, "<tmp>") : undefined } }));
}

// --- config schema ---------------------------------------------------------
out.config_schema = [
  {}, { org: "acme" }, { org: "" }, { bogus: "x" }, { org: "acme", bogus: 1 },
  { org: 5 }, { org: null },
].map((input) => { const r = QuoinConfigSchema.safeParse(input); return { input, valid: r.success, value: r.success ? r.data : undefined }; });

// --- ConfigService layering (via resolveOrg) -------------------------------
{
  const priorXdg = process.env.XDG_CONFIG_HOME, priorOrg = process.env.QUOIN_ORG;
  const cases = [];
  const run = (label, { user, project, projectEnabled, env, remote }) => {
    const configHome = mkdtempSync(join(tmpdir(), "quoin-381-cfg-"));
    process.env.XDG_CONFIG_HOME = configHome;
    if (env === undefined) delete process.env.QUOIN_ORG; else process.env.QUOIN_ORG = env;
    if (user !== undefined) { const d = join(configHome, "ix", "config.d"); mkdirSync(d, { recursive: true }); writeFileSync(join(d, "quoin.yaml"), user); }
    let projectRoot;
    if (project !== undefined) { const p = mkdtempSync(join(tmpdir(), "quoin-381-proj-")); projectRoot = join(p, ".ix"); mkdirSync(join(projectRoot, "config.d"), { recursive: true }); writeFileSync(join(projectRoot, "config.d", "quoin.yaml"), project); }
    const root = mkdtempSync(join(tmpdir(), "quoin-381-repo-"));
    if (remote !== undefined) { mkdirSync(join(root, ".git"), { recursive: true }); writeFileSync(join(root, ".git", "config"), gitConfig(remote)); }
    const opts = {}; if (projectRoot) opts.projectConfigRoot = projectRoot; if (projectEnabled !== undefined) opts.projectConfigEnabled = projectEnabled;
    cases.push({ label, given: { user: user ?? null, project: project ?? null, projectEnabled: projectEnabled ?? null, env: env ?? null, remote: remote ?? null }, resolved: resolveOrg(root, opts) });
  };
  run("stored beats remote", { user: "org: from-config\n", remote: "git@github.com:from-git/repo.git" });
  run("remote when nothing stored", { remote: "git@github.com:from-git/repo.git" });
  run("env layers over stored", { user: "org: from-config\n", env: "from-env", remote: "git@github.com:from-git/repo.git" });
  run("nothing at all", {});
  run("project beats user", { user: "org: user-level\n", project: "org: project-level\n", projectEnabled: true, remote: "git@github.com:from-git/repo.git" });
  run("project disabled", { user: "org: user-level\n", project: "org: project-level\n", projectEnabled: false, remote: "git@github.com:from-git/repo.git" });
  run("malformed user config falls through", { user: "org: [unterminated\n", remote: "git@github.com:from-git/repo.git" });
  run("unknown key in user config", { user: "org: stored\nbogus: 1\n", remote: "git@github.com:from-git/repo.git" });
  run("empty org in user config", { user: 'org: ""\n', remote: "git@github.com:from-git/repo.git" });
  run("blank env ignored", { user: "org: from-config\n", env: "   ", remote: "git@github.com:from-git/repo.git" });
  out.config_service_org = cases;
  if (priorXdg === undefined) delete process.env.XDG_CONFIG_HOME; else process.env.XDG_CONFIG_HOME = priorXdg;
  if (priorOrg === undefined) delete process.env.QUOIN_ORG; else process.env.QUOIN_ORG = priorOrg;
}

// --- resolveOrg with explicit env ------------------------------------------
{
  const root = mkdtempSync(join(tmpdir(), "quoin-381-repo2-"));
  mkdirSync(join(root, ".git"), { recursive: true });
  writeFileSync(join(root, ".git", "config"), gitConfig("git@github.com:from-git/repo.git"));
  out.resolve_org_flag_env = [
    { opts: { flag: "from-flag", env: { QUOIN_ORG: "from-env" } } },
    { opts: { env: { QUOIN_ORG: "from-env" } } },
    { opts: { env: {} } },
    { opts: { flag: "   ", env: { QUOIN_ORG: "  " } } },
    { opts: { flag: "  padded  ", env: {} } },
  ].map((c) => ({ opts: c.opts, resolved: resolveOrg(root, c.opts) }));
}

writeFileSync(process.argv[2], JSON.stringify(out, (k, v) => (v === undefined ? null : v), 2) + "\n");
console.log("wrote", process.argv[2]);
