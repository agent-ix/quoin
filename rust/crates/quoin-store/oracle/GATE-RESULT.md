# Cutover gate result — 2026-09-12

Recorded artifact for the differential replay described in `COMPATIBILITY.md`.
The oracle capture itself is 687 MiB of newline-delimited JSON and is not
committed; it is identified here by its digest and reproduced by the commands
below.

## The run

```bash
# Population: every `spec/evidence` store reachable under /home/peter/dev,
# excluding node_modules and this branch's own .worktrees.
find /home/peter/dev -maxdepth 6 -type d -path '*/spec/evidence' \
  -not -path '*/node_modules/*' -not -path '*/.worktrees/*' \
  | sed 's|/spec/evidence||' | sort > repos.txt

QUOIN_SRC_ROOT=/home/peter/dev/quoin node --loader ts-node/esm \
  oracle/capture-store-oracle.mjs --out store-oracle.ndjson $(cat repos.txt)

cargo +1.98.1 run --release --bin quoin-store-replay -- \
  --oracle store-oracle.ndjson $(cat repos.txt)
```

TypeScript half, final line, verbatim:

```
typescript oracle: 90 stores, 464 json files, 7 raw outputs, 10748590 digests, 0 serialization failures -> /tmp/claude-1000/store-oracle.ndjson
```

Capture identity:

| | |
|---|---|
| sha256 | `c6f55fec00734c0cd02d7d1019c5cf0a67ff0379e4d53c3dceda8f9a60776472` |
| bytes | 720,326,915 |
| lines (one per store entity) | 471 |

## Rust half, verbatim, exit status 0

```
stores replayed            : 90
store files found          : 464
store files parsed         : 464
digests replayed           : 10748598
  of which raw outputs     : 7
digest mismatches          : 0
sealed records verified    : 8
retained outputs verified  : 7
round-trip byte-identical  : 413 of 464
files rust refused to read : 0
files with __proto__       : 0
foreign sha256 references  : 36796
store integrity findings   : 0
COMPARED POPULATION        : 471 of 471 store entities (464 files + 7 raw outputs)
oracle entries unmatched   : 0
store entries unmatched    : 0

GATE: PASS
```

## What the compared population means

`COMPARED POPULATION` is part of the gate, not commentary. `gate_passes()`
requires it to be non-zero **and** equal to the store entities walked — 464
files plus 7 retained raw outputs — with no unmatched entry on either side. A
run that compared nothing, or that compared a proper subset because the oracle
capture was truncated or keyed differently, reports `GATE: FAIL`. That is
`FR-098-CON-3` and `FR-098-AC-7`, and it is asserted by
`tests/tc_replay_gate.rs`.

The 10,748,598 digests the Rust half reports exceed the oracle's 10,748,590 by
8: the 8 sealed records whose own `digest` member is re-verified against the
record, which is a store-integrity check internal to this crate and has no
oracle counterpart.

## The 90 stores

```
/home/peter/dev/.research-synaptic/quire-rs-main
/home/peter/dev/.research-synaptic/quoin-main
/home/peter/dev/assurance-local-evidence/quoin-v02-cleanroom-pilot-repo
/home/peter/dev/assurance-local-evidence/quoin-v02-final-pilot-repo
/home/peter/dev/assurance-local-evidence/quoin-v02-pilot-repo
/home/peter/dev/engineering-assurance/corpus
/home/peter/dev/qa-corpus
/home/peter/dev/quire-contract-codegen
/home/peter/dev/quire-contract-ir
/home/peter/dev/quire-contract-ir/target/assurance/store
/home/peter/dev/quire-contract-runtime
/home/peter/dev/quire-corpus
/home/peter/dev/quire-rs
/home/peter/dev/quire-rs/corpus
/home/peter/dev/quoin
/home/peter/dev/quoin/corpus
/home/peter/dev/tl-mltl
/home/peter/dev/tl-mltl-corpus-spec
/home/peter/dev/tl-mltl-verification-spec
/home/peter/dev/tl-parse
/home/peter/dev/tl-parse-bindings
/home/peter/dev/tl-rewrite
/home/peter/dev/tl-rewrite-property-grounding
/home/peter/dev/tl-rewrite-semantic-identity
/home/peter/dev/tl-syntax
/home/peter/dev/tl-syntax-past-profile
/home/peter/dev/tl-syntax-qualification-readiness
/home/peter/dev/tl-syntax-rust-1.98.1
/home/peter/dev/worktrees/codegen-25-kani
/home/peter/dev/worktrees/contract-core-codegen-integration
/home/peter/dev/worktrees/contract-core-codegen-kani
/home/peter/dev/worktrees/contract-core-codegen-terminal
/home/peter/dev/worktrees/contract-core-formal-profile
/home/peter/dev/worktrees/contract-core-ir-binding
/home/peter/dev/worktrees/contract-core-ir-recovery
/home/peter/dev/worktrees/contract-core-runtime-snapshot
/home/peter/dev/worktrees/ea-65-semantics-final/corpus
/home/peter/dev/worktrees/ea-65-semantics/corpus
/home/peter/dev/worktrees/ea-66-discovery-workflow/corpus
/home/peter/dev/worktrees/ea-67-matrix-truth/corpus
/home/peter/dev/worktrees/ea-68-observer/corpus
/home/peter/dev/worktrees/ea-69-v030/corpus
/home/peter/dev/worktrees/ea-compat-rust/corpus
/home/peter/dev/worktrees/ea-fix-tc039-status/corpus
/home/peter/dev/worktrees/ea-pending-sweep/corpus
/home/peter/dev/worktrees/ea-tc126/corpus
/home/peter/dev/worktrees/engineering-assurance-agent-c-28/corpus
/home/peter/dev/worktrees/engineering-assurance-agent-c-59-ci-spec/corpus
/home/peter/dev/worktrees/engineering-assurance-agent-c-59-evidence-rust/corpus
/home/peter/dev/worktrees/engineering-assurance-agent-c-59-registry-spec/corpus
/home/peter/dev/worktrees/engineering-assurance-agent-c-59-rust-198/corpus
/home/peter/dev/worktrees/engineering-assurance-agent-c-59-semantic/corpus
/home/peter/dev/worktrees/engineering-assurance-agent-c-59/corpus
/home/peter/dev/worktrees/engineering-assurance-agent-c-chain-spec/corpus
/home/peter/dev/worktrees/engineering-assurance-agent-c-evaluation-removal/corpus
/home/peter/dev/worktrees/engineering-assurance-agent-c-manifest-cutover/corpus
/home/peter/dev/worktrees/engineering-assurance-agent-c-manifest-host/corpus
/home/peter/dev/worktrees/engineering-assurance-agent-c-onboarding-removal/corpus
/home/peter/dev/worktrees/engineering-assurance-agent-c-remove-aggregate/corpus
/home/peter/dev/worktrees/engineering-assurance-agent-c-remove-compatibility-script/corpus
/home/peter/dev/worktrees/engineering-assurance-agent-c-tc109/corpus
/home/peter/dev/worktrees/qa-corpus-agent-c-20
/home/peter/dev/worktrees/qa-corpus-agent-c-403
/home/peter/dev/worktrees/qa-corpus-provenance
/home/peter/dev/worktrees/qa-corpus-provenance-verify
/home/peter/dev/worktrees/quire-407-ir-proof
/home/peter/dev/worktrees/quire-contract-ir-agent-c-54
/home/peter/dev/worktrees/quire-contract-ir-predicate-e
/home/peter/dev/worktrees/quire-contract-ir-rust-1.98.1
/home/peter/dev/worktrees/quire-contract-ir-temporal-contracts-clean-e
/home/peter/dev/worktrees/quire-contract-ir-temporal-contracts-e
/home/peter/dev/worktrees/quire-contract-ir-temporal-correspondence-clean-e
/home/peter/dev/worktrees/quire-contract-ir-temporal-correspondence-e
/home/peter/dev/worktrees/quire-contract-ir-temporal-e
/home/peter/dev/worktrees/quire-rs-agent-c-403
/home/peter/dev/worktrees/quire-rs-agent-c-403/corpus
/home/peter/dev/worktrees/quire-rs-agent-c-412
/home/peter/dev/worktrees/quire-rs-agent-c-412/corpus
/home/peter/dev/worktrees/quire-rs-agent-c-414
/home/peter/dev/worktrees/quire-rs-agent-c-414-impl
/home/peter/dev/worktrees/quire-rs-agent-c-414-impl/corpus
/home/peter/dev/worktrees/quire-rs-agent-c-417
/home/peter/dev/worktrees/quire-rs-agent-c-417-impl
/home/peter/dev/worktrees/quire-rs-agent-c-417-impl/corpus
/home/peter/dev/worktrees/quire-rs-provenance-verify
/home/peter/dev/worktrees/quire-rs-provenance-verify/corpus
/home/peter/dev/worktrees/quire-rs-status-column
/home/peter/dev/worktrees/quire-rs-status-column/corpus
/home/peter/dev/worktrees/quoin-provenance-verify
/home/peter/dev/worktrees/quoin-provenance-verify/corpus
```
