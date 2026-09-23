# quoin

## Commands

`quoin` is a native Rust CLI (`rust/crates/quoin-cli`, binary name `quoin`).
There is no Node/JavaScript build here — no root `package.json`, no `jest`,
no `pnpm`. `src/semantic/` holds JSON schemas the Rust workspace reads, not a
second build tree.

```bash
make build   # cargo build --workspace --locked
make test    # make rust-gate: fmt --check, clippy -D warnings, cargo deny, cargo test
make lint    # cargo fmt --all --check + cargo clippy --all-targets --all-features -D warnings
make format  # cargo fmt --all
make clean   # cargo clean
```

See `Makefile` for the exact targets; `rust-build`, `rust-lint`, `rust-deny`
and `rust-test` are the granular pieces `build`/`test` compose. CI
(`.github/workflows/build-test.yml`) runs `make rust-lint`, `make rust-build`
and `make rust-test` directly, and releases (`.github/workflows/native-release.yml`)
build with `cargo build --locked --release -p quoin-cli --bin quoin` and ship
the resulting binary as a GitHub Release archive. A separate workflow,
`.github/workflows/release.yml`, then repackages that same release's assets
into npm packages (`@agent-ix/quoin` plus one platform package per target,
FR-112) via the shared `agent-ix/nodejs-actions/publish-native-npm` action —
it never builds anything itself, and no packaging logic lives in this repo.

## Agent worktrees

**Never as a sibling in the dev root.** `/home/peter/dev` is the scan path for
`ts-build-chain` (`IX_DEV`), so a worktree there carries a second
`package.json` claiming `@agent-ix/quoin` and every chain command fails before
it starts:

```
Package name collision (user-fix class): @agent-ix/quoin is claimed by multiple repositories:
  /home/peter/dev/quoin
  /home/peter/dev/quoin-ea004
Remove or rename the duplicate before continuing.
```

`trace-chain`, `check-updates` and `audit-registry` are all blocked by it, for
every repo in the chain and not just this one. Eight such trees were left
behind during EA-ticket work (quoin#195).

Put them **under the repo**, where no package scan reaches:

```bash
git worktree add .worktrees/<name> -b <branch> origin/main
```

Claude Code's own `EnterWorktree` already does this — it nests under
`<repo>/.claude/worktrees/<name>`.

**Teardown is part of the work.** A worktree is not small: the eight above held
316 MB, one of them 202 MB. When the branch is pushed, remove it:

```bash
git worktree remove .worktrees/<name>
```

If a tree is being kept, say so and why. `ticket-runner` is the model — its
`removeWorkspace()` runs `git worktree remove --force` gated by
`keepWorkspaces` (default `false`), for the stated reason that _"worktrees are
not small, and an unattended fleet left running will fill a disk."_ Same class
as quoin#184, where `mkdtempSync` fixtures were created with no teardown path.

## Dogfooding an unreleased quoin

**There is no local-publish path, and that is a deliberate choice rather than
something to discover (quoin#196).** Releases are cut by `native-release.yml`
on `workflow_dispatch` against an existing tag; there is no local equivalent.

To run unreleased `main`:

```bash
make build
./rust/target/debug/quoin <command>          # debug build, from the repo root
# or: cargo run --manifest-path rust/Cargo.toml -p quoin-cli -- <command>
```

A release-profile binary, matching what `native-release.yml` ships, is
`cargo build --manifest-path rust/Cargo.toml --locked --release -p quoin-cli`
(binary at `rust/target/release/quoin`).

The only path to a published release archive is a **git tag plus CI**. That
matters when `main` carries unreleased fixes: a downloaded release archive
reflects the last tag, so anyone installing that way gets a build without
them. Check what you are actually running — `quoin --version` reports the
build-time `git describe`, so a `-<n>-g<sha>` suffix means the binary is ahead
of its tag.

**Version provenance is load-bearing.** Every SpecReview records the tool
version it measured with, and three reviews in `agent-ix/filament-ide-rs` cite
numbers from a binary whose self-reported version was wrong. Check
`--version`/`--help` agreement and a clean tag reporting itself before tagging.

**`verificationStack.buildProfile: "release"` in a measurement record is a
self-declared attestation, not something quoin checks about its own binary.**
`quoin-measurement`'s intake validator (`rust/crates/quoin-measurement/src/validate/stack.rs`,
`src/validate/mod.rs`) only checks that the field, if present, is the string
`"debug"` or `"release"` (and requires `"release"` for a new collection) — it
never inspects how the `quoin` binary that produced the record was itself
compiled. Running a debug build of quoin does not stop you from writing
`buildProfile: "release"` into a record, and running a release build does not
set the field for you. This is why one evaluation lane got through cleanly and
another concluded "release" was unreachable from a debug-built test run: both
readings were about the *content* of a JSON field, never about the *build
profile of the tool measuring it*.

## Rust

`rust/` is a Cargo workspace inside this repository, beside `src/` — the Rust
burn-down (#373) ports quoin's engine logic behind a `quoin-core` subprocess
one stage at a time, and FR-101 requires both trees to be exercised at one
candidate revision, which two repositories cannot do.

```bash
make rust-gate       # fmt --check, clippy -D warnings, cargo deny, cargo test
```

Difftest-style comparisons against canonical JSON bytes live inside the Rust
test suite itself (e.g. `quoin-assurance`'s golden cases) — there is no
separate `rust-difftest` make target.

**Read `.claude/skills/rust-style/SKILL.md` before writing or reviewing
anything under `rust/`.** It is the repo-level idiom doc and it outranks the
ecosystem `rust-style` and `rust-review` skills here; the ecosystem
`rust-review` checklist defers to it by name in its own §0. It states the
boundary contract (exit taxonomy, canonical JSON, stream discipline), the error
envelope, the workspace lint policy and the `tc_NNN` / `/// Trace:` test
convention, and every rule in it is enforced by a lint or a test.

Invoke cargo **from `rust/`**: rustup selects a toolchain from the working
directory, so `cargo --manifest-path rust/Cargo.toml` run from the root ignores
`rust/rust-toolchain.toml` and silently builds with `rustup default`.

## Adding or improving a spec check

quoin's analyses and quire's validators both point at the `~/dev` corpus, where a
new check will fire in the hundreds or thousands. **That is the expected result,
not a signal the check is wrong.**

A high finding count means exactly one of two things, and which one is a question
of fact:

- **Bad rule** — the check misreads data that is correct.
- **Bad corpus** — the check reads correctly and the specs are wrong.

**Do not default to either.** Agents wrote most of these specs, and agents do not
write good specs — that is the reason quoin and quire exist at all.

**Settle it by opening flagged documents and reading them.** For each: _is the
thing the check complains about actually absent?_ If the document has it in a
form the check could not read, the rule is wrong. If the document genuinely lacks
it, the finding stands and is work to do. Report the split as a number — "sampled
10, 3 rule, 7 real" — because a finding count is a census, not a precision
estimate.

**Never widen a rule because it lowers the count.** A rule states what a _good_
spec looks like; its goal is not to fit the specs that exist. A widening needs a
justification true independent of the number — "these two verbs mean the same
thing in the declared edge vocabulary" is a reason, "this drops 400 findings" is
not. Where two forms do mean the same thing, prefer **unifying the corpus on one
and flagging the rest** over accepting both: a rule that accepts every spelling
enforces nothing.

Say which of the two conclusions you reached, and why, whenever a rule changes
after a measurement.

**Advisory-first is about blast radius, not about whether findings matter.** Ship
a new check at `warning` so findings land and stay visible; promotion is a
separate, measured, user-gated decision.
