# Assurance workflows

These workflows use project-owned contracts and synthetic or independently
authored rule sets. They do not distribute or claim compatibility with an
external assurance-argument format.

## 1. Evaluate applicability

Quire evaluates a versioned module-supplied ClauseSet. Applicability has three
outcomes: `binding`, `not_binding`, and `unresolved`. An omitted context value
therefore cannot silently become false.

```bash
quire clauses evaluate \
  --module ./modules/widget \
  --authority example.invalid \
  --set widget-rules \
  --version 1.0.0 \
  --context product=widget \
  --format json > binding.json
```

The report binds the exact ClauseSet version and content digest. Use `quire
clauses diff` to review changes between exact versions.

## 2. Account for discharge

Create a JSON array of facts. A direct fact cites evidence; a disposition
records an authorized decision. Both carry an attestation and expiry.

```json
[
  {
    "kind": "direct",
    "clauseId": "SYN-001",
    "evidenceRefs": ["evidence://run/one"],
    "attestation": {
      "attestedBy": "reviewer-1",
      "authority": "release reviewer",
      "attestedAt": "2026-08-01T00:00:00Z",
      "expiresAt": "2026-09-01T00:00:00Z",
      "sourceRevision": "0123456789abcdef",
      "evidenceDigest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    }
  }
]
```

```bash
quoin discharge --binding binding.json --facts discharge-facts.json \
  --as-of 2026-08-28T00:00:00Z --json > discharge.json
```

Every binding clause appears exactly once as direct, disposition, or open.
Unresolved applicability remains separate, expired facts reopen their clauses,
and no aggregate score is emitted.

## 3. Render an authored argument

Author an `AssuranceArgument` document under `spec/` using the
engineering-assurance module. The top claim, reasoning, sufficiency criteria,
assumptions, participants, authority, independence, challenges, and expiries
are authored facts; Quoin does not invent them.

Each criterion needs a decision whose `decidedBy` participant and `authority`
exactly match the authored participant:

```json
[
  {
    "reasoningId": "ARG-1",
    "criterion": "Every binding synthetic clause has a current disposition.",
    "state": "satisfied",
    "evidenceRefs": ["evidence://discharge/widget-1"],
    "decidedBy": "reviewer-1",
    "authority": "may accept or reject this release",
    "decidedAt": "2026-08-01T00:00:00Z",
    "expiresAt": "2026-09-01T00:00:00Z",
    "sourceRevision": "0123456789abcdef",
    "evidenceDigest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
  }
]
```

```bash
quoin assurance --repo . --argument AA-001 \
  --decisions sufficiency-decisions.json \
  --discharge discharge.json \
  --as-of 2026-08-28T00:00:00Z
```

Missing, open, future, or expired decisions remain open. Accepted assumptions
whose review is due, unresolved challenges, accepted risks without a current
expiry, open binding clauses, and unresolved applicability also keep the top
claim open.

## 4. Publish immutable evidence

`record-experiment` accepts `experiment-record-v1`; `record-operational`
accepts `operational-evidence-record-v1`. Both require the same closed producer
provenance tuple:

```json
{
  "schemaVersion": "producer-provenance-v1",
  "identity": "example.invalid/synthetic-runner",
  "version": "1.2.3",
  "sourceRevision": "0123456789abcdef",
  "sourceState": "clean",
  "executableDigest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "configurationDigest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
  "capabilities": ["synthetic-experiment-v1"],
  "artifacts": [
    {
      "name": "raw-result.json",
      "digest": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
    }
  ]
}
```

The writer validates the complete input, derives `recordId` from canonical
content, and atomically publishes under `spec/evidence/experiments/` or
`spec/evidence/operational/`. Repeating identical input is idempotent. Different
bytes at an existing immutable path are reported and never overwritten.

The producer tuple applies to measurement collections and these two new record
kinds. Existing FR-030 run records remain unchanged; changing that contract
requires a separately versioned decision and migration.
