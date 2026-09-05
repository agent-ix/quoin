---
id: TASK-060
title: "Corpus enumeration, pinning and the read-only envelope"
type: Task
status: todo
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/FR-084"
    type: references
  - target: "ix://agent-ix/quoin/FR-092"
    type: references
  - target: "ix://agent-ix/quoin/TC-1500"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1501"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1502"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1503"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1504"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1505"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1566"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1567"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1551"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1552"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1553"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1554"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1555"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1583"
    type: verifies
---

# TASK-060: Corpus enumeration, pinning and the read-only envelope

## Scope

Build the population and the envelope it is measured inside. A directory is a
corpus repository when it carries a `.git` directory and a `spec` directory; a
`.git` file is a worktree or a submodule link and excludes that directory under
`git-link-file`; a nested `.git` below a repository root excludes its subtree
under `nested-repository` and is evaluated separately; a symbolic link is never
traversed and is recorded under `symlink`. The exclusion vocabulary is a declared
input recorded verbatim in the output. `HEAD` and cleanliness are read before and
after the walk, and a repository whose either value moved is recorded
`stable: false`.

## Subtasks

- [ ] Declare the corpus input: workspace root, exclusion vocabulary, corpus identifier, output directory.
- [ ] Enumerate repositories and documents, recording `origin`, `commit`, `clean`, `stable` and the document count.
- [ ] Record every excluded candidate with the rule that excluded it, `symlink` and `nested-repository` included.
- [ ] Re-read `HEAD` and cleanliness after the walk and record `stable`.
- [ ] Refuse an output directory inside another enumerated corpus repository before reading a file.
- [ ] Write the run manifest with a SHA-256 digest of every artifact written.
- [ ] Exit zero whatever is found; reserve non-zero for the measurement failing to run.
- [ ] Assert working-tree and ref invariance over every enumerated repository before and after a run.

## Deliverables

- A corpus record that states its own population, and a run that provably changes nothing outside its output directory.

## Notes

- The workspace holds a symlink (`filament`) and at least three submodules (`quoin/corpus` among them). Both rules exist because the workspace already exhibits them, not in the abstract.
- The same workspace yields 331,702, 24,643 or 7,587 documents depending on the exclusion vocabulary. That is why the vocabulary is an input somebody authors.
