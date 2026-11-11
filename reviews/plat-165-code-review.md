---
id: SR-20261103118171
title: "Code review of quoin PR 804 (content-addressed measurement attachments)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@037490c; rust/crates/quoin-store, rust/crates/quoin-measurement, rust/crates/quoin-core/src/ops/measurement/taxonomy.rs, rust/Cargo.lock"
review_set: subset
---

# Code review of quoin PR 804

## Summary

Ticket: PLAT-165. Scoped to `git diff origin/main...037490c` (16 files). Rust lane
(`rust-review`, repo `rust-style`) folded in. Tools: quoin 0.30.0, quire coverage engine as
installed. Checks run from a detached review worktree at the PR head:

- `cargo clippy -p quoin-core --lib -- -D warnings`: clean (the unconfirmed gate).
- `cargo clippy -p quoin-store -p quoin-measurement --all-targets --all-features -- -D warnings`: clean.
- `cargo test -p quoin-store --lib`, `-p quoin-measurement --lib --test tc_165_measurement_attachments --test tc_468_boundary`: 76 + 5 + 3 + 34 passed, 0 failed.

Attachment safety, traced through the code:

- Path traversal: `attachment_path` joins only a string that passes `is_sha256_hex` (64 lowercase
  hex), so a reference cannot name a path. `from_json` strips `sha256:` and applies the same test.
- Final-component symlink: `read_attachment` uses `read_regular_file_bounded` (`O_NOFOLLOW |
  O_NONBLOCK`, `fstat` on the handle), so a symlink, FIFO or directory at the address is refused
  as `QM-ATTACHMENT-UNAVAILABLE`. Tested (tc_165_004).
- Bounds: count <= 256 and duplicate addresses are refused while parsing, before any file is
  opened; `byteLength` must be whole and <= 64 MiB; the read is bounded by the declared length, so
  allocation never exceeds 64 MiB per attachment. `retain_attachment` refuses > 64 MiB.
- No-replace and tampered-vs-idempotent: retention goes through `store::publish` with
  `StorePath::ContentAddressed`, i.e. EA's hard-link no-replace publisher. Identical bytes are
  `AlreadyPresent` (idempotent); different bytes at the address are `ContentCollision`, mapped to
  `QM-ATTACHMENT-TAMPERED`, and nothing is replaced. Tested (tc_165_001, tc_165_004).
- TOCTOU: publish verifies attachments and then writes the collection. A swap between the two is
  not prevented, but it is caught by `verify_collection_attachments` afterwards, which is the
  design. Concurrent retains cannot clobber each other (hard link, not rename). Reads use the
  opened handle. No defect.
- The error codes are new, appended, mapped in the core taxonomy and in the intervention intake's
  exhaustive match; the taxonomy count test moved 30 -> 33.

## Verdict

**PASS WITH FINDINGS (one medium, one low).** The code is careful and the safety properties the
brief named hold for the file at the address. The medium finding is that the no-follow
guarantee stops at the final path component.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Only the last path component is opened no-follow. If `spec/evidence/attachments`, `spec/evidence` or `spec` is a symlink, `retain_attachment` (through `create_dir_all` and `publish`) writes outside the repository, and `read_attachment` and verification read from there. The ticket names symlinked and escaping inputs. FR-044-AC-13 already refuses a symlink at any component for artifacts | rust/crates/quoin-store/src/attachment.rs:41-52, rust/crates/quoin-measurement/src/attachment.rs:160-176, rust/crates/quoin-measurement/src/attachment.rs:197-205 |
| FND-002 | low | `sha2 = "0.10"` is a caret range declared locally in the crate. The repo's rust-style asks for exact pins, with shared versions kept in `[workspace.dependencies]`. The crate's own `libc = "=0.2.189"` sets the precedent. It should be `=0.10.9`, the version in the lockfile | rust/crates/quoin-store/Cargo.toml:19 |
