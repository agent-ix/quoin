---
id: TC-001
title: "Differential report v2 adapter transcribes every disposition and refuses malformed reports"
type: TC
relationships:
  - target: "ix://agent-ix/quoin/FR-151"
    type: "verifies"
  - target: "ix://agent-ix/quoin/NFR-032"
    type: "verifies"
---

# TC-001: Differential report v2 adapter transcribes every disposition and refuses malformed reports

## Description

Verify that the `differential-report` adapter maps each `filament.differential-report/v2` disposition to the run
entry or `unrepresented` result of FR-151, and refuses reports that are the wrong version, malformed,
self-contradictory or oversized, writing no run in any refusal.

## Test Procedure

1. With an empty evidence store and a suite registered, run `quoin evidence record --adapter differential-report` over
   one report captured from `filament-analyze run-package` for each disposition the runner can produce: agreement
   satisfied, agreement refuted, disagreement, inconclusive, and unavailable (missing executable); and over a
   captured or hand-constructed v2 document, accepted by the owner's validator, with an engine `unsupported-platform`
   (a Linux runner reports `cleanup-failed` instead, so this one is not a runner capture). Also run it over an agreement report with a closed `selection` member, captured unedited from
   `filament-analyze run-package --select`. Read the store and the
   command's human and JSON output after each.
2. Run the same command, each time against a fresh store, over the refusal table of FR-151-AC-14, one row per
   refusal class, each derived from a capture by one stated change: wrong (including the retired
   `quire.differential-report/v2` namespace), absent or non-string `schema`; a missing,
   extra (top level and engine), wrongly typed or out-of-set member; empty, truncated, non-UTF-8, array, scalar and
   non-JSON input; the four self-contradiction reports; a reordered copy and a pretty-printed copy; and, from the
   selection-bearing capture, two further rows: a `selection` with an extra third member, and a `selection` with a
   missing list (FR-151-AC-16).
3. Run the command over an input of `filament_analyze::MAX_REPORT_BYTES` + 1 bytes and one of exactly `MAX_REPORT_BYTES`
   bytes, and over one valid report twice from different working directories.
4. Run `quoin evidence record --tool filament-analyze`, `--tool differential-report`, `--tool quire-analyze`,
   `--tool tl-mltl`, `--tool r2u2` and `--tool c2po` over a valid report, and list the adapters.
5. Attempt `cargo check --locked -p quoin-cli --target x86_64-pc-windows-msvc` on the Linux host through
   `/home/peter/dev/worktrees/locked-build.sh <worktree>/rust/target`. Record whether this cross-target check
   completes. When the host has no compatible Windows cross-assembler, record the toolchain failure without
   attributing it to the dependency; verify target support against the existing `windows-2025` matrix entry in
   `.github/workflows/native-release-build.yml`, which builds `quoin-cli` natively for
   `x86_64-pc-windows-msvc`.

## Expected Results

Step 1: agreement satisfied records one `pass` entry and agreement refuted one `fail` entry, both under symbol
`<analysisKind>`; inconclusive and each unavailable report record one `error` entry; disagreement
records a run with zero entries and prints one `unrepresented` result of state `disagreement` naming both engines
with status and outcome verbatim. No entry carries `traceIds`.

Step 1 with `selection`: the valid report is recorded as without it, and its projection holds `selection` equal to the source lists (FR-151-AC-16).

Step 2: every input, including the two malformed-`selection` rows, is refused with a non-zero exit and an error naming the schema value or member found (the
owner's validator error for the rest), and the store holds no run.

Step 3: the `MAX_REPORT_BYTES` + 1 input is refused naming the size limit without being parsed, the input of exactly
`MAX_REPORT_BYTES` bytes is not refused for size, and the two valid runs return identical results.

Step 4: `filament-analyze` and `differential-report` select the adapter; `quire-analyze`, `tl-mltl`, `r2u2` and `c2po`
do not, and the listing names `filament.differential-report/v2` and no `differential-summary`.

Step 5: the direct cross-target command's exit status and build failure location are recorded. On a Linux host
without a Windows cross-assembler, `psm` may invoke the host GNU assembler for COFF input and fail; that result
does not establish whether the dependency builds on Windows. The existing native Windows release job is the
target-platform build procedure and must remain green for the target to be considered supported.

Recorded verification for PLAT-1707: `x86_64-pc-windows-msvc` was installed, but this Linux host had no
`clang`, `clang-cl` or MinGW cross compiler. The command
`/home/peter/dev/worktrees/locked-build.sh <worktree>/rust/target cargo check --locked -p quoin-cli --target x86_64-pc-windows-msvc`
exited 101. The `psm` 0.1.32 build script selected the Windows GNU assembly and host `cc` rejected COFF
`.def`/`.scl` directives; `aws-lc-sys` 0.45.0 also failed under host `cc` (`pthread_rwlock_t` unavailable).
This Linux cross-check is inconclusive for the dependency's native Windows build. The existing
`.github/workflows/native-release-build.yml` matrix assigns `x86_64-pc-windows-msvc` to `windows-2025` and
builds `quoin-cli` natively with `cargo build --locked --release -p quoin-cli --bin quoin`; that job is the
target-platform verification.
