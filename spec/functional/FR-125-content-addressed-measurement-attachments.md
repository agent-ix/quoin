---
id: FR-125
title: "Immutable content-addressed binary attachments for measurements"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-044"
    type: "extends"
  - target: "ix://agent-ix/quoin/NFR-025"
    type: "depends_on"
---

# FR-125: Immutable content-addressed binary attachments for measurements

## Description

A `MeasurementCollection` SHALL be able to reference binary artifacts that cannot
be embedded losslessly as JSON (a profile, a trace, sanitizer output) through an
optional `attachments` member. Each attachment is retained once, immutably, under
the SHA-256 of its bytes at `spec/evidence/attachments/<hex>`. `quoin measurement attach` retains a repository file; the attachment is verified against its address when a collection that references it is published and when `quoin measurement verify` runs.

## Rationale

`rawEvidence` is opaque JSON, and a raw-evidence file reference proves only that
a file exists. A binary retained by name can be edited or swapped after the
collection that cites it was sealed. The content address is the one digest that
binds a file to the exact bytes it held, so tampering is visible and two
different byte strings can never share a name.

## Behavior

- An `attachments` member is an array of `{ "digest": "sha256:<64 lowercase hex>", "mediaType": <non-empty text>, "byteLength": <whole number> }`.
- Retention writes through the store's no-replace publish; identical bytes are idempotent and differing bytes at an occupied address are refused.
- `quoin measurement attach --path <file> --media-type <type>` retains a repository-relative file and prints the reference to state in `attachments`.
- Quoin does not interpret attachment bytes and never claims their domain semantics are valid.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-125-AC-1 | Retaining bytes names them by their SHA-256: byte-distinct attachments get distinct addresses, identical bytes reproduce the same reference and are retained once, and reading the reference returns exactly the retained bytes, binary included. | Test |
| FR-125-AC-2 | Publishing a collection whose `attachments` member is not an array, exceeds 256 entries, repeats an address, or holds a reference with a malformed or path-like digest, an empty media type, or a `byteLength` that is not a whole number within 64 MiB, refuses as `QM-ATTACHMENT-INVALID` before any attachment file is read and writes no collection. Retaining an empty media type is refused likewise. | Test |
| FR-125-AC-3 | Publishing a collection that references an attachment with no retained bytes refuses as `QM-ATTACHMENT-UNAVAILABLE`, and one whose declared length or address disagrees with the retained bytes refuses as `QM-ATTACHMENT-TAMPERED`; each names the attachment and writes no collection. | Test |
| FR-125-AC-4 | After a collection is sealed, editing a retained attachment's bytes (including by symlink substitution at any component of the retained path) or deleting it makes `verify_collection_attachments` refuse, naming the attachment, and retaining through a symlinked component refuses; retaining the original bytes again over edited bytes refuses as `QM-ATTACHMENT-TAMPERED` and replaces nothing. | Test |
| FR-125-AC-5 | A collection that references retained attachments publishes, reads back with its `attachments` member intact, verifies clean, and each attachment is recovered byte for byte. | Test |
| FR-125-AC-6 | `quoin measurement attach` retains a regular repository file and prints its reference; it refuses as `QM-ATTACHMENT-INVALID`, before any byte is loaded and retaining nothing, an absolute path, a `..` segment, an empty path, a symlink at any path component, a directory or other non-regular file, a file over 64 MiB, and a missing file. | Test |
| FR-125-AC-7 | `quoin measurement verify` refuses, naming the attachment, when an attachment referenced by a stored collection is edited (`QM-ATTACHMENT-TAMPERED`) or deleted (`QM-ATTACHMENT-UNAVAILABLE`). | Test |

## Constraints

- The attachment address is the only digest Quoin computes for measurement evidence; campaign evidence stays digest-free (FR-114).
- Attachments are bounded: 64 MiB each, 256 per collection.

## Dependencies

- FR-044 supplies write-once collections; NFR-025 supplies the immutability of evidence bytes; the store's content-addressed publish supplies no-replace retention.
