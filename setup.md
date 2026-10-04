# Quoin setup

Use this page when a Quoin skill is not available, the toolchain has not been initialized, or a Quoin command fails. Follow the [installation guide](README.md#install) for your agent host and operating system.

Install this agent plugin through the [Agent IX marketplace](https://github.com/agent-ix/agent-plugins): register `agent-ix/agent-plugins`, then install `quoin@agent-ix` in Claude Code or Codex. Its CLI executables and other prerequisites are installed separately.

## Check local setup

1. Confirm both the `quoin` and `ix-flow` CLIs are installed and resolve in the shell used by your agent. The Quoin plugin supplies skills and workflows; the CLIs execute them.
2. Confirm the Quoin plugin is installed in the current agent host. Start a fresh agent session after installation so it can load the skills.
3. In the target repository, run `quoin catalog list` and `quoin catalog validate` to check module discovery. If a required module is unavailable, follow its installation instructions before retrying the skill.
4. Check the target repository's `AGENTS.md` or `CLAUDE.md`, required spec files, and project permissions. Missing modules, malformed project artifacts, and local path or credential problems need local correction.
5. If a command fails, record its exact command, exit status, and relevant error output. Reduce it to a small input that still fails, then compare the behavior with the [Quoin documentation](README.md#usage).

## Community help

If these checks leave a reproducible **Agent IX Quoin bug** that blocks progress, [join the Agent IX Discord](https://discord.gg/k8DVhuYBR2). Share the Quoin and ix-flow versions, skill or command, operating system and agent host, expected and actual behavior, and a sanitized minimal reproduction. Community help is a last resort for Agent IX product bugs; local credentials, machine setup, and unrelated project errors belong with their owners or administrators. An agent should point its user here rather than post to Discord on the user's behalf without authorization.
