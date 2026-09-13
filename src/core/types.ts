/**
 * The quoin-core boundary types (FR-097). GENERATED — DO NOT EDIT.
 *
 * Written by `quoin-schemas/quoin-schemas-gen` from the JSON Schema `schemars`
 * reads off the canonical Rust declarations in
 * `rust/crates/quoin-core/src/protocol.rs`, `.../src/ops/` and the
 * domain crates those operations answer from.
 * Rust is the source of truth: where this file and a Rust type
 * disagree, the Rust type is right and this file is stale.
 *
 * Regenerate with `make types`. A hand edit does not survive review
 * and does not survive the gate: `quoin-schemas` asserts the digest
 * below against BOTH these bytes and a fresh render, so an edit here
 * fails `make rust-test` at an unchanged path, and a Rust type change
 * that was never regenerated fails it too.
 *
 * The machine-readable provenance is the `CORE_TYPES_PROVENANCE`
 * record below, and it is the ONLY copy: a second, prose copy up
 * here would be one more thing that can disagree with the artefact
 * it describes.
 */

/**
 * What produced this file, readable by the TypeScript side.
 *
 * Generated status is established by THIS record, not by the
 * artefact's directory name (FR-097-CON-2): moving the file does not
 * make it hand-written, and writing a file into a `generated/`
 * directory does not make it generated.
 */
export const CORE_TYPES_PROVENANCE = {
  generator: "quoin-schemas/quoin-schemas-gen",
  generatorVersion: "0.1.0",
  sourceSchemaSha256:
    "9d6a775327486558c05ed7e5d9e120e4c29584505e0ba6820e910b29297b54ee",
} as const;

/**
 * A resolved git commit id, as forty lowercase hex characters.
 */
export type CommitSha = string;

/**
 * One entry of the stderr array.
 *
 * A `Serialize` struct rather than a hand-built `serde_json::Value`: the
 * field list is then reviewable, and the compiler checks that every branch
 * populated it.
 */
export interface Diagnostic {
  /**
   * The stable code, from the catalogued enum — never a literal invented
   * at the call site.
   */
  code: string;
  /**
   * Ordered context. `BTreeMap` for byte-stable serialisation.
   */
  context: Record<string, string>;
  /**
   * A sentence for an operator.
   */
  message: string;
}

/**
 * One located gate-that-gates-nothing defect.
 *
 * `subject`, `changeTarget`, `remedy` and `summary` are advisory prose for the
 * operator. Verdict parity is defined on `(kind, obligation, path, line,
 * wiredBy)`; the prose is reproduced faithfully because it is part of the
 * emitted payload, not because its wording is contractual.
 */
export interface EmptyGateFinding {
  /**
   * `path:line`, the exact locus an operator must edit.
   */
  changeTarget: string;
  /**
   * Always `GateThatGatesNothing` today.
   */
  kind: FindingKind;
  /**
   * The 1-based line of the unasserted count.
   */
  line: LineNumber;
  /**
   * The obligation the gate comment claims to enforce.
   */
  obligation: ObligationId;
  /**
   * The shell script holding the unasserted count.
   */
  path: RepoPath;
  /**
   * What to change.
   */
  remedy: string;
  /**
   * Human label for the gate.
   */
  subject: string;
  /**
   * The full three-way join, stated once.
   */
  summary: string;
  /**
   * The build or CI file that wires the script, proving it is a gate.
   */
  wiredBy: RepoPath;
}

/**
 * The payload `modules.ensure_defaults` writes to stdout.
 */
export interface EnsureDefaultsPayload {
  /**
   * Entries installed for the first time.
   */
  installed: string[];
  /**
   * Entries skipped because `defaultEnabled` is `false`.
   */
  skipped: string[];
  /**
   * Entries already present and correctly pinned; no network was used.
   */
  unchanged: string[];
  /**
   * Entries re-resolved to a different commit.
   */
  updated: string[];
}

/**
 * The request accepted by `modules.ensure_defaults`.
 */
export interface EnsureDefaultsRequest {
  /**
   * The `~/.ix` home to reconcile.
   */
  home?: string | null;
  /**
   * The text of `default-modules.yaml`.
   *
   * Carried in the request rather than located here: the file ships inside
   * the npm package, so the caller already knows where its own package root
   * is and the boundary does not need to guess at one.
   */
  manifest: string;
  /**
   * `lazy` (the default) installs only what is missing or re-pinned; `sync`
   * re-resolves every entry.
   */
  mode?: Mode;
}

/**
 * The class of defect a finding reports.
 *
 * One variant today. It is an enum and not a `&'static str` because the kind is
 * the payload's discriminant: a second validator adds a variant here and every
 * `match` on it becomes a compiler-checked edit site.
 */
export type FindingKind = "gate-that-gates-nothing";

/**
 * The payload `modules.install` writes to stdout.
 */
export interface InstallPayload {
  /**
   * The registry record that was written.
   */
  module: InstalledModule;
  /**
   * Whether a previous version of the same module was replaced.
   */
  replaced_previous: boolean;
}

/**
 * The request accepted by `modules.install`.
 */
export interface InstallRequest {
  /**
   * The `~/.ix` home to install into.
   */
  home?: string | null;
  /**
   * The CLI source argument, in the `path:` / `github:` / `package:`
   * spellings `src/plugins.ts`'s `parseSourceArg` accepted.
   */
  source: string;
}

/**
 * One installed module's registry record.
 */
export interface InstalledModule {
  /**
   * RFC 3339 timestamp of the install.
   */
  installedAt: string;
  /**
   * The module's declared name; also its directory name.
   */
  name: ModuleName;
  /**
   * The git ref (tag/branch) that was requested, if any.
   */
  ref?: string | null;
  /**
   * The cache path the content was materialized from.
   */
  resolvedPath: string;
  /**
   * The semantic contract pin, when the module declares a semantic block.
   */
  semantic?: SemanticPin | null;
  /**
   * The resolved commit id — the durable pin used for drift detection.
   */
  sha?: CommitSha | null;
  /**
   * Where it came from.
   */
  source: Source;
  /**
   * The materialized path under the modules directory.
   */
  targetPath: string;
}

/**
 * A 1-based line number inside a source file.
 *
 * `NonZeroU64` rather than `usize`: line 0 does not exist, and the payload
 * crosses a JSON boundary where a 0 would be read as "unknown".
 */
export type LineNumber = number;

/**
 * The payload `modules.list` writes to stdout.
 */
export interface ListPayload {
  /**
   * Every installed module, in registry order.
   *
   * Registry order and not sorted here: `src/commands/module/list.ts`
   * printed what the registry held, in the order it held it, and a sort
   * introduced at the boundary would be a user-visible change smuggled in
   * under a port.
   */
  modules: InstalledModule[];
}

/**
 * The `home` a request may name, with its bound already checked.
 */
export interface ListRequest {
  /**
   * The `~/.ix` home to read, or absent for the one the host resolves.
   */
  home?: string | null;
}

/**
 * How hard a reconcile should look, in the wire spelling.
 */
export type Mode = "lazy" | "sync";

/**
 * The declared name of a spec module.
 *
 * A module name is used directly as a directory name under
 * `~/.ix/filament/modules`, so it is validated against path separators and
 * relative-path components at construction. Every place that joins a name onto
 * a directory takes a `ModuleName`, not a `String`, which is what makes that
 * check unskippable.
 */
export type ModuleName = string;

/**
 * A requirement obligation as it was written in the gate comment, e.g.
 * `FR-001-AC-1`.
 *
 * The claim regex is case-insensitive, so this deliberately preserves the
 * author's spelling rather than normalising it: the finding must point at what
 * the file actually says.
 */
export type ObligationId = string;

/**
 * The payload `core.ping` writes to stdout.
 */
export interface PingPayload {
  /**
   * The `quoin-core` crate version.
   */
  core_version: string;
  /**
   * Whatever `echo` held, unchanged.
   */
  echo?: string | null;
  /**
   * The protocol revision this build speaks.
   */
  protocol_version: number;
}

/**
 * The request accepted by `core.ping`.
 *
 * `deny_unknown_fields` so a caller that misspells a field is refused rather
 * than silently ignored — a field the boundary drops is a field the caller
 * believes it sent.
 */
export interface PingRequest {
  /**
   * An opaque token returned unchanged, for correlating a call with its
   * answer across the pipe. Absent is fine.
   */
  echo?: string | null;
  /**
   * The protocol revision the caller believes it is speaking. When it
   * disagrees with this build's, the answer is still complete — it is how
   * the caller learns which revision it is actually talking to — so the
   * disagreement is reported as `Partial`, not as a
   * failure.
   */
  expect_protocol?: number | null;
}

/**
 * The payload `modules.remove` writes to stdout.
 */
export interface RemovePayload {
  /**
   * The module that was removed.
   */
  removed: string;
}

/**
 * The request accepted by `modules.remove`.
 */
export interface RemoveRequest {
  /**
   * The `~/.ix` home to remove from.
   */
  home?: string | null;
  /**
   * The installed module's name.
   */
  name: string;
}

/**
 * A path relative to the repository root, always written with `/` separators.
 *
 * Findings are compared, sorted, and matched against build wiring by this
 * string, so the separator normalisation is part of the identity and not a
 * display concern — a Windows-shaped `scripts\gate.sh` and a POSIX
 * `scripts/gate.sh` are the same gate.
 */
export type RepoPath = string;

/**
 * The payload `config.resolve_org` writes to stdout.
 *
 * Field-for-field `ResolvedOrg` in the deleted `src/org.ts`, so the caller in
 * `src/core/org.ts` is a rename and not a reshaping: `org` is omitted rather
 * than null when unresolved, exactly as the TypeScript `org?: string` was.
 */
export interface ResolveOrgPayload {
  /**
   * Whether a config layer fell back rather than contributing its content.
   *
   * New at the boundary, and not a widening of the contract: `src/org.ts`
   * discarded this fact silently, which is why a malformed config file
   * resolved to "no stored org" with nothing to show for it. It rides the
   * payload; the run is still a success, because a broken config must not
   * stop an author writing specs (FR-027-AC-5).
   */
  degraded: boolean;
  /**
   * The organization, omitted when nothing yielded one.
   */
  org?: string | null;
  /**
   * Which source won: `flag`, `env`, `config`, `git` or `none`.
   */
  source: string;
}

/**
 * The request accepted by `config.resolve_org`.
 *
 * Every field is state the caller already holds. Nothing here is a path this
 * operation would open.
 */
export interface ResolveOrgRequest {
  /**
   * The environment the resolution reads, supplied rather than read.
   *
   * `QUOIN_ORG` is a *declared binding* (`QUOIN_ENV_BINDINGS`), so it is
   * layered over the config document by the schema machinery rather than
   * consulted directly — one precedence rule in one place, which is the
   * property `src/org.ts` went out of its way to keep and this preserves.
   */
  env?: Record<string, string>;
  /**
   * The explicit `--org` value, when one was passed.
   */
  flag?: string | null;
  /**
   * The repository's `.git/config`, absent when there is none to read.
   */
  git_config?: string | null;
  /**
   * The project-level `.ix` config document, absent when no project layer
   * applies or the file does not exist.
   */
  project_config?: string | null;
  /**
   * The user-level config document, absent when the file does not exist.
   */
  user_config?: string | null;
}

/**
 * The payload `validators.run` writes to stdout.
 *
 * One field, named `findings`, because that is the byte shape
 * `quoin validate --json` has always emitted and the cutover is not licence to
 * change a user-visible document.
 */
export interface RunPayload {
  /**
   * Every finding, ordered by `(path, line, obligation)`.
   */
  findings: EmptyGateFinding[];
}

/**
 * The request accepted by `validators.run`.
 *
 * A snapshot of the repository, not a path to it. `deny_unknown_fields` so a
 * caller that misspells a field is refused rather than silently ignored — a
 * field the boundary drops is a field the caller believes it sent.
 */
export interface RunRequest {
  /**
   * Every file the caller found, keyed by repository-relative,
   * `/`-separated path, holding the file's lines split on `\n`.
   *
   * `null` means the caller found the path but could not read it. That is
   * not the same as omitting it: an unreadable file still classifies as a
   * shell script or as wiring, and refuses only if the analysis reaches for
   * its text — which is the on-demand read the TypeScript oracle performs.
   *
   * Lines rather than one string because that is the shape the golden corpus
   * already carries. Joining the lines with a newline reconstructs the file,
   * so a caller must split on a newline alone and leave any carriage return
   * on the line.
   */
  files: Record<string, string[] | null>;
  /**
   * Directories the caller could not list, repository-relative. The empty
   * string names the repository root itself.
   *
   * Separate from an unreadable file because the walk **aborts** on one: a
   * subtree nobody could read means the answer would be computed over a
   * repository nobody has seen, and an empty result must mean the validator
   * looked and found nothing.
   */
  unlistable?: string[];
}

/**
 * The registry pin recorded under an installed module's `semantic` key.
 *
 * Field names match `SemanticRegistryPin` in `src/semantic/package-manifest.ts`
 * because the registry file is shared with the retained TypeScript.
 */
export interface SemanticPin {
  /**
   * Digest per exported symbol.
   */
  exports: Record<string, string>;
  /**
   * The module's declared semantic package.
   */
  package: string;
  /**
   * The semantic core it binds to.
   */
  semanticCore: string;
}

/**
 * A GitHub repository whose module root is the repository root.
 */
export interface SourceGithub {
  /**
   * A tag or branch to pin to.
   */
  ref?: string | null;
  /**
   * `owner/repo`, or a full clonable url.
   */
  repo: string;
  /**
   * A commit id to pin to; outranks `ref`.
   */
  sha?: string | null;
  type: "github";
}

/**
 * A module living in a subdirectory of a git repository.
 */
export interface SourceGitSubdir {
  /**
   * The subdirectory holding the module root.
   */
  path: string;
  /**
   * A tag or branch to pin to.
   */
  ref?: string | null;
  /**
   * A commit id to pin to; outranks `ref`.
   */
  sha?: string | null;
  type: "git-subdir";
  /**
   * `owner/repo`, or a full clonable url.
   */
  url: string;
}

/**
 * A git repository by url.
 */
export interface SourceGit {
  /**
   * A tag or branch to pin to.
   */
  ref?: string | null;
  /**
   * A commit id to pin to; outranks `ref`.
   */
  sha?: string | null;
  type: "git";
  /**
   * A clonable url.
   */
  url: string;
}

/**
 * A plain url. Accepted structurally, not resolvable.
 */
export interface SourceUrl {
  /**
   * A tag or branch, carried for round-tripping.
   */
  ref?: string | null;
  /**
   * A commit id, carried for round-tripping.
   */
  sha?: string | null;
  type: "url";
  /**
   * The url.
   */
  url: string;
}

/**
 * A local directory.
 */
export interface SourcePath {
  /**
   * The directory.
   */
  path: string;
  type: "path";
}

/**
 * An npm package. Accepted structurally, not resolvable here.
 */
export interface SourceNpm {
  /**
   * The package name.
   */
  package: string;
  /**
   * A registry override.
   */
  registry?: string | null;
  type: "npm";
  /**
   * An exact version.
   */
  version?: string | null;
}

/**
 * Where a module's content comes from.
 */
export type Source =
  | SourceGithub
  | SourceGitSubdir
  | SourceGit
  | SourceUrl
  | SourcePath
  | SourceNpm;

/**
 * The message shown when no source yielded an organization.
 *
 * Served over the boundary so the sentence has ONE home. It was a `const` in
 * `src/org.ts` and a `const` in `quoin_config::org`, and two copies of a
 * user-facing sentence is exactly the drift FR-101 retires.
 */
export interface UnresolvedOrgMessagePayload {
  /**
   * The sentence.
   */
  message: string;
}

/** The IPC protocol revision this build of the boundary speaks. */
export const PROTOCOL_VERSION = 1;
