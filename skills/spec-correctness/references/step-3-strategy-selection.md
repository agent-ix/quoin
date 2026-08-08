# Step 3: Strategy Selection

**Goal**: pick the assertion skeleton and the routing lane. Two independent axes.

- **Strategy** comes from `property` — the 10-value taxonomy.
- **Routing** comes from `extraction` — `extractable` / `candidate` / `not-extractable`.

`shape` (`assertion | obligation | given-when-then | unstructured`) is the FR-047 grammar
axis. **It never selects a strategy.** At most it tells you where the clause boundaries sit
in the sentence.

## Routing

| | `extractable` | `candidate` | `not-extractable` |
| --- | --- | --- | --- |
| One of the 8 generatable properties | emit **unattended** | emit → **queue** | grounded → queue; refused → step 5 |
| `example` / `unclassified` | n/a | n/a | **step 5 only** |

**Grounding outranks the table.** The lanes above assume step 2 grounded the criterion as
the classification describes it. Where step 2 returned one of its reclassifying reasons, use
that instead — a record can be `extraction: extractable` and still not be a property:

| step-2 reason | lane |
| --- | --- |
| `singleton-domain` | **queue as a `Unit` witness**, never unattended |
| `label-from-mention` | re-derive the strategy from the grounded oracle; lane unchanged |
| `criterion-describes-its-test` | find the existing test; record *already covered*, emit nothing |
| `static-or-demonstration` | refuse; no matrix row |
| any other refusal | queue with the reason, or step 5 |

`singleton-domain` is the common one. On a real run it moved 31 of 52 `universal` criteria
out of the unattended lane. Routing on `extraction` alone would have emitted all 52.

`candidate` means the criterion carries a metamorphic label the structural pass did not
corroborate — the label came from a declared idiom alone, or the shape landed but no
predicate marker supplied an oracle. It is the only field a module declaration can move,
and it is safe precisely because it is review-gated. Nothing reaches the unattended set
through it.

## Strategy per property

Full table with generator notes: [`../assets/strategy-table.md`](../assets/strategy-table.md).

| property | generator domain | assertion skeleton |
| --- | --- | --- |
| `round-trip` | `x : Domain` (the inner type) | `dec(enc(x)) == x`; if lossy, `norm(dec(enc(x))) == norm(x)` |
| `idempotence` | `x`, seeded with `f`'s own outputs so the fixpoint branch is hit | `f(f(x)) == f(x)` |
| `ordering` | `xs : list<T>`, length 0..32 | pick by verb — *sorted* → `is_sorted_by(key, f(xs))`; *regardless of order* → `f(xs) == f(shuffle(xs))`; *reorders/preserves* → `multiset(f(xs)) == multiset(xs)` |
| `invariant` | `x : Domain` | `P(f(x))`; if stateful, `P(state)` after **every** op in a generated sequence |
| `error-case` | the **negative** domain — complement generator plus `prop_filter` / `fc.pre` / `assume` | fails with error class `E` **and** the stated payload shape; never panics, never returns success |
| `lifecycle` | an op sequence drawn from the state machine | per op `sut.apply(op).ok == model.allows(op)`, then the invariant; terminal state at the end |
| `concurrency` | interleaved or parallel op sequences | the concurrent result is *some* linearization of the sequential model; no invariant violated at any interleaving |
| `universal` | the clause subject | `oracle(x)` — a direct translation of the criterion |
| `example` | — | not a property. Step 5 may reclassify it, or emit a `Unit` witness test |
| `unclassified` | — | not a property. Step 5 only |

`universal` is the default and the largest bucket. `round-trip` needs **two** symbols — if
grounding found only one side of the pair, route to the queue rather than inventing the
inverse.

## Harness-specific downgrades

- **`concurrency` in Python** — hypothesis ships no interleaving scheduler. Emit a
  `RuleBasedStateMachine` sketch over the async model, always queued, never unattended.
- **`lifecycle` anywhere** — generatable only if the states and transitions are enumerable
  from the FR's `## Behavior`. Otherwise the refusal is `no-state-machine`.
- **`concurrency` in Rust** — prefer `loom` for exhaustive interleaving of a small model;
  fall back to `proptest-state-machine`'s parallel mode when the op set is large.

## Weak assertions are a failure, not a fallback

If the strategy collapses to any of these, do not emit it — queue with the grounding
refusal instead:

- `assert result is not None` / `expect(x).toBeDefined()` / `assert!(r.is_ok())` alone
- a type check with no value check
- a test with no assertion at all
- an assertion that restates the implementation rather than the criterion

## Output of this step

Per record: `{row_id, strategy, lane, grounding}` — where `lane ∈ {unattended, queue,
second-pass}`.
