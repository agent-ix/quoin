# Quoin setup

Use this page when a Quoin skill is not available, the toolchain has not been initialized, or a Quoin command fails. Follow the [installation guide](README.md#install) for your agent host and operating system.

Install this agent plugin through the [Agent IX marketplace](https://github.com/agent-ix/agent-plugins): register `agent-ix/agent-plugins`, then install `quoin@agent-ix` in Claude Code or Codex. Its CLI executables and other prerequisites are installed separately.

## Check local setup

1. Confirm both the `quoin` and `ix-flow` >=0.3.1 CLIs are installed and resolve in the shell used by your agent. The Quoin plugin supplies skills and workflows; the CLIs execute them.
2. Confirm the Quoin plugin is installed in the current agent host. Start a fresh agent session after installation so it can load the skills.
3. In the target repository, run `quoin catalog list` and `quoin catalog validate` to check module discovery. If a required module is unavailable, follow its installation instructions before retrying the skill.
4. Check the target repository's `AGENTS.md` or `CLAUDE.md`, required spec files, and project permissions. Missing modules, malformed project artifacts, and local path or credential problems need local correction.
5. If a command fails, record its exact command, exit status, and relevant error output. Reduce it to a small input that still fails, then compare the behavior with the [Quoin documentation](README.md#usage).

## Community help

If these checks leave a reproducible **Agent IX Quoin bug** that blocks progress, [join the Agent IX Discord](https://discord.gg/k8DVhuYBR2). Share the Quoin and ix-flow versions, skill or command, operating system and agent host, expected and actual behavior, and a sanitized minimal reproduction. Community help is a last resort for Agent IX product bugs; local credentials, machine setup, and unrelated project errors belong with their owners or administrators. An agent should point its user here rather than post to Discord on the user's behalf without authorization.

## Owned workflows and upgrading old installations

Quoin 0.29.1 ships review and planning definitions beside its agent plugin skills.
The CLI supplies the Rust validation provider. Install the published runner with
`npm install -g @agent-ix/ix-flow@latest`. No archived workflow repository,
workflow npm provider, or IX_SPEC_WORKFLOWS_ROOT override is required.

Run `command -v quoin`, `quoin --version`, `command -v ix-flow`, and
`ix-flow --version` in the agent's shell. Confirm Quoin CLI/plugin >=0.29.1 and
ix-flow >=0.3.1. Load updated skills in a fresh session without stopping other
agents automatically.

Invoke the spec-review or spec-to-plan skill. Its frontmatter declares the local
workflows directory, and its instructions start `ix-flow run review --path
<installed-spec-review-directory> --json` or the equivalent to-plan workflow.
The agent records repository context and drives ix-flow's normal commands.
The convenience CLI commands accept the same explicit skill directory through
`quoin review --path <skill-directory> --target spec/ --json`; they initialize
context automatically. A missing --path is an actionable error. Definitions are
read from that directory and are never extracted from the binary.

CI needs no agent host or plugin manager: check out the released Quoin tag and
use `skills/spec-review` or `skills/spec-to-plan` as the skill directory. Install
the Quoin and ix-flow CLIs and the required catalog, then drive the same commands.
The release smoke copies the released skills into an isolated plugin directory
and tests their real workflows against the packaged native executable.

Old external-provider runs are preserved. Start a fresh owned run, register the
existing review documents, revalidate them, and obtain fresh human acceptance.
Direct analyses are documents, not evidence of an enforced acceptance receipt.
An absent receipt does not establish that the underlying analyses are invalid.

Inventory old receipts without modifying them:
`cd rust && cargo run --locked --target-dir target -p quoin-workflow-check -- audit <state-dir> <repo>...`. The inventory
distinguishes documents from run receipts and labels legacy enforcement unverified.
It does not backfill acceptance or judge review quality.
