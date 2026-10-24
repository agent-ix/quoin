
# Quoin specification to plan

Start through the installed skill as described in SKILL.md. Record absolute repository/config roots and
targets in the run. Follow the installed spec-to-plan skill and live catalog
contracts. Resolve readiness, scope, verification strategy, dependencies, risks,
and failure domains before drafting tasks.

Record interview `plan` answers with `plan_path`, `index_path`, `task_paths` and
`readiness_notes`. The plan and index share a bundle directory; every task belongs
under that bundle's tasks directory. Obtain contracts using
`quoin write <repo> --types Plan,Task,index`; author the documents directly.

Advance intake -> evidence_recorded -> plan_drafted -> validated. The provider
opens and validates every declared document using the installed catalog.
A missing task or unavailable catalog cannot pass. Before accepted, present the
bundle and wait for explicit human approval; acknowledge only that decision.
Acceptance revalidates the files. Never automatically acknowledge the gate.

Use the launch result's --config-root and --state-dir for ix-flow status, resume,
record-answers, advance and ack. Existing external-provider runs are not silently
converted into this workflow; create a new run and revalidate existing artifacts.
