# Strategy Table

The full `property` → test-strategy mapping. `extraction` decides the lane (see
`references/step-3-strategy-selection.md`); this table decides the test.

`shape` (`assertion | obligation | given-when-then | unstructured`) never appears here —
it is the FR-047 grammar axis, not the property axis.

---

## `round-trip`

- **Needs** two symbols, an encoder and its inverse. Only one found → queue.
- **Domain** the *inner* type — generate `x`, not the encoded form, so every value is
  reachable and shrinking is meaningful.
- **Oracle** `dec(enc(x)) == x`.
- **Lossy variant** when the FR says normalization happens: `norm(dec(enc(x))) == norm(x)`.
  Cite the FR line that states the loss; do not assume it.
- **Trap** encoding invalid inputs. If `enc` is partial, filter the domain rather than
  catching an error — an error there is a different criterion.

## `idempotence`

- **Domain** `x`, **seeded with `f`'s own outputs** so the already-normalized branch is hit.
  A generator that never produces a fixpoint tests only half the property.
- **Oracle** `f(f(x)) == f(x)`.
- **Trap** `f` that is idempotent only on valid input — add the validity precondition from
  `## Behavior`, cited.

## `ordering`

- **Domain** `xs : list<T>`, length `0..32`. Include the empty and singleton cases; they
  are where ordering code breaks.
- **Oracle — pick one by the criterion's verb**:
  - *sorted*, *in order*, *ascending* → `is_sorted_by(key, f(xs))`
  - *regardless of order*, *independent of order* → `f(xs) == f(shuffle(xs))`
  - *reorders*, *preserves*, *without loss* → `multiset(f(xs)) == multiset(xs)`
- **Trap** asserting a total order when the criterion states a stable partial one. Sort by
  the stated key only.

## `invariant`

- **Domain** `x : Domain`.
- **Oracle** `P(f(x))` for a pure function. For stateful code, generate an op sequence and
  assert `P(state)` **after every op**, not only at the end.
- **`P` is the entire risk.** It must come from a `SHALL` predicate or a code-level bound,
  cited. `assert result is not None` is not an invariant — that is a refusal.

## `error-case`

- **Domain** the **negative** one: the complement of the valid domain, built with
  `prop_filter` / `fc.pre` / `assume`.
- **Oracle** two assertions, both required:
  1. it fails with the stated error class;
  2. the payload matches the stated shape (message fragment, exit code, error variant).
- **Where the error class comes from**: the code. The engine emits
  `span:refused-weak-boundary` on these criteria, meaning it *declined* to guess the
  boundary. Read the error enum or exception class; do not infer it from the prose.
- **Trap** a filter so narrow the generator exhausts. Prefer constructing the negative
  domain directly over filtering a large valid one.

## `lifecycle`

- **Generatable only if** the states and transitions are enumerable from `## Behavior`.
  Otherwise refuse with `no-state-machine`.
- **Domain** an op sequence, length `1..24`, drawn from the op set.
- **Oracle** per op: `sut.apply(op).ok == model.allows(op)`; then the invariant; then the
  terminal state at the end of the sequence.
- The model is a few lines of plain data — a set of legal ops per state. If writing it
  requires reimplementing the SUT, refuse instead.

## `concurrency`

- **Domain** interleaved or parallel op sequences over a small op set.
- **Oracle** the concurrent result is *some* linearization of the sequential model, and no
  invariant is violated at any interleaving.
- **Rust** `loom` for exhaustive interleaving of a small model; `proptest-state-machine`
  parallel mode when the op set is large.
- **TypeScript** `fc.scheduler()` — deterministic and shrinkable.
- **Python** hypothesis has no interleaving scheduler. Emit a `RuleBasedStateMachine`
  sketch over the async model, **always queued**, never unattended.

## `universal`

- The default family and the largest bucket in practice.
- **Domain** the clause subject, typed from the code signature.
- **Oracle** a direct translation of the criterion's predicate.
- **Trap** translating the implementation instead of the criterion. If the assertion reads
  like the function body, it proves nothing — re-derive from `## Behavior`.

## `example`

- **Not a property.** Step 5 owns it.
- May become a reclassified property (with citations) or a `Unit` witness test. A witness
  is queued and is **not** counted as property coverage.

## `unclassified`

- **Not a property.** Step 5 owns it.
- Absence of a signal is not absence of a property — but it is also not evidence of one.
  Ground it or record nothing.
