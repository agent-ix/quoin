# quoin

## Commands

```bash
make build                      # build library
make test                       # exact-source canonical verification stack
make test-with-quire QUIRE=/absolute/path/to/quire  # explicit inner test gate
make lint                       # eslint + prettier check
make format                     # prettier format
make update-lock                # update pnpm-lock.yaml
make add-packages p=<name>      # add runtime dependency
make add-dev-packages p=<name>  # add dev dependency
make use-local p=<name>         # switch dep to local package
make use-upstream p=<name>      # switch dep back to upstream
```

## Adding or improving a spec check

See **CLAUDE.md § Adding or improving a spec check**. The short form:

A new check pointed at the `~/dev` corpus will fire in the hundreds or thousands.
**That is expected, not evidence the check is wrong.** A high count means one of
two things, settled by reading flagged documents, not by preference:

- **Bad rule** — the check misreads correct data.
- **Bad corpus** — the check reads correctly and the specs are wrong.

Do not default to either. Agents wrote most of these specs and agents do not
write good specs — that is why quoin and quire exist.

**Never widen a rule because it lowers the count.** A rule states what a good spec
looks like; it does not fit the specs that exist. Where two forms mean the same
thing, prefer unifying the corpus on one and flagging the rest over accepting
both. Report the precision split as a number, and say which conclusion you
reached and why.

## Banking a finding

Follow `corpus/CONTRIBUTING.md` (the `qa-corpus` submodule) for every census or
dogfood finding: bank the case
before the fix, cover every applicable language or record a reasoned exclusion,
and provide a language-matched healthy control. If reproduction needs a full
real tree, use its Tier-2 snapshot and pin-move procedure; do not paraphrase the
failure into a smaller shape that no longer reproduces it.
