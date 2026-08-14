# AI Skills

A small collection of reusable `SKILL.md` instructions for coding agents.
The skills are plain Markdown, version-controlled, and portable across
compatible agent tools.

## Skills

| Skill | Purpose | Documentation |
|---|---|---|
| Planning | Creates durable plans with goals, steps, verification, progress tracking, and handoffs. | [planning/SKILL.md](planning/SKILL.md) |
| Brainstorm | Shapes an under-specified idea into a recorded, agreed picture (`brainstorm.md`) before planning, with an adversarial completion pass and a plan-vs-implement gate. | [brainstorm/SKILL.md](brainstorm/SKILL.md) |
| Post-implementation review | After-the-fact review of built code with proposed fixes, backed by implementer analysis, an independent solutions agent, and a critical-feedback agent. | [post-implementation-review/SKILL.md](post-implementation-review/SKILL.md) |
| Project-specific deviations | Records confirmed project behavior and environment quirks that future agents should not rediscover. | [project-specificies/SKILL.md](project-specificies/SKILL.md) |
| Resource-limited testing | Runs resource-intensive commands under platform-appropriate CPU and memory controls. | [resource-limited-testing/SKILL.md](resource-limited-testing/SKILL.md) |

Use a skill only when its frontmatter trigger matches the task or when the
user explicitly requests it. Each skill documents when not to activate.

## One-command installer

Run this command and choose the skills and agent destination interactively:

```bash
curl -fsSL https://raw.githubusercontent.com/tschallacka/ai-skills/refs/heads/master/install.sh | bash
```

The installer can install all the skills or one skill, and supports these
global skill roots:

| Destination | Agent or standard |
|---|---|
| `~/.agents/skills` | Universal Agent Skills root; recommended shared destination |
| `~/.codex/skills` | Codex CLI |
| `~/.claude/skills` | Claude Code |
| `~/.config/opencode/skills` | OpenCode |
| `~/.openclaw/skills` | OpenClaw managed skills |
| `~/.cline/skills` | Cline |

The universal root is also discovered by OpenCode and OpenClaw. Installing the
same skill into multiple roots can create duplicate definitions or precedence
conflicts, so choose only the roots you need.

The interactive installer checks which supported agents are present and omits
roots for agents it cannot detect. Custom roots are saved in
`~/.config/tsch-ai-skills/custom-locations` and are offered again when they
still exist.

### npm installation

Install the package globally to expose the installer command. The npm package
keeps the skills in this repository and links `ai-skills-install` directly to
the existing `install.sh` script:

```bash
npm install -g @tschallacka/ai-skills
ai-skills-install
```

For a one-off run without a global install:

```bash
npx --yes --package @tschallacka/ai-skills ai-skills-install
```

The npm package does not install skills automatically as an npm lifecycle
side-effect; run the installer command when you are ready to choose a target.

Each installed skill contains a `.version` marker identifying its tag, branch,
and commit. If an installed file differs from the repository version, the
installer asks before replacing it. For a managed version transition that the
user approves, old files are replaced without backups; the previous version
can be restored by running the installer against its tag. Unmanaged changes
still receive `.bak` backups. A symlinked skill is skipped for manual review
rather than following the link and modifying an unexpected location.

### Non-interactive selection

Use `--all`, `--skill`, and `--target` when the choices are already known:

```bash
# Install all skills into the shared Agent Skills root
curl -fsSL https://raw.githubusercontent.com/tschallacka/ai-skills/main/install.sh \
  | bash -s -- --all --target "$HOME/.agents/skills"

# Install or update only planning for Codex
curl -fsSL https://raw.githubusercontent.com/tschallacka/ai-skills/main/install.sh \
  | bash -s -- --skill planning --target "$HOME/.codex/skills"
```

Use `--yes` only when unattended replacement is intentional. Managed version
transitions are replaced without backups; unmanaged changed files are backed
up:

```bash
curl -fsSL https://raw.githubusercontent.com/tschallacka/ai-skills/main/install.sh \
  | bash -s -- --all --target "$HOME/.agents/skills" --yes
```

Review the installer before running it if you do not trust the source. Skills
are instructions that may guide agents to run commands or access files.

## Supported agent documentation

- [Agent Skills](https://agentskills.io/)
- [Codex skills](https://github.com/openai/codex/tree/main/codex-rs/skills)
- [Claude Code skills](https://code.claude.com/docs/en/agent-sdk/skills)
- [OpenCode skills](https://opencode.ai/docs/skills)
- [OpenClaw skills](https://docs.openclaw.ai/skills)
- [Cline skills](https://docs.cline.bot/customization/skills)

## Updating

Run the installer again. It compares each installed file with the repository,
skips unchanged files, and records the installed source in `.version`.

## Development checkout

Run the installer directly from a checkout to use its local files without
downloading an archive:

```bash
./install.sh
```

The installer also accepts `AI_SKILLS_REPO_URL` and `AI_SKILLS_REF` when a
different repository or branch must be used.

## Notes

- The planning-skill benchmark harness is agent-agnostic. `benchmark/planning/runtime/`
  makes the worker/reviewer/analyzer launch, session-id extraction, and token
  telemetry pluggable per CLI: the active agent defaults to `codex` and is
  selected with `BENCHMARK_AGENT` (`opencode`, `claude`), with a shared launcher
  (`lib-agent.sh`) owning all setsid/timeout/process-group control. See
  `benchmark/planning/runtime/README.md` for the contract and first-time setup.
- Skills are instructions, not standalone applications. They add no
  dependencies unless a skill explicitly documents one.
- `resource-limited-testing` uses systemd and cgroup v2 for strongest Linux
  isolation, with weaker fallbacks documented for unsupported Linux sessions
  and macOS.
- Read and review third-party skills before enabling them in an agent with
  access to sensitive files, credentials, or external systems.

## License

Distributed under the [MIT License](LICENSE).
