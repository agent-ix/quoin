# quoin

## Commands

```bash
make build                      # build library
make test                       # run jest
make lint                       # eslint + prettier check
make format                     # prettier format
make update-lock                # update pnpm-lock.yaml
make add-packages p=<name>      # add runtime dependency
make add-dev-packages p=<name>  # add dev dependency
make use-local p=<name>         # switch dep to local package
make use-upstream p=<name>      # switch dep back to upstream
make check-version              # every version surface agrees; a clean tag reports itself
```

## Dogfooding an unreleased quoin

**There is no local-publish path, and that is a deliberate choice rather than
something to discover (quoin#196).** `ts-build-chain` classifies quoin as an app
(it ships a `bin`, not a library entry), so a single-node chain runs green and
publishes nothing:

```
$ ts-build-chain start --skip-registry-audit quoin quoin
✔ Build
✔ Test
❯ Publish
↓ Publish [SKIPPED: Not a library (no publish target)]
```

To run unreleased `main`:

```bash
make build
npm i -g .        # or: node bin/quoin.js <command>
```

The only path to a registry is a **git tag plus CI**. That matters when `main`
carries unreleased fixes: npm serves the last tag, so anyone installing the
normal way gets a build without them. Check what you are actually running —
`quoin --version` reports the build-time `git describe`, so a
`-<n>-g<sha>` suffix means the binary is ahead of its tag.

**Version provenance is load-bearing.** Every SpecReview records the tool
version it measured with, and three reviews in `agent-ix/filament-ide-rs` cite
numbers from a binary whose self-reported version was wrong. `make check-version`
asserts that `--version` and `--help` agree and that a clean tag reports itself;
run it before tagging.

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
