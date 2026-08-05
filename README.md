# AI Skills

A small collection of reusable `SKILL.md` instructions for coding agents.
The skills are plain Markdown, version-controlled, and portable across
compatible agent tools.

## Skills

| Skill | Purpose | Documentation |
|---|---|---|
| Planning | Creates durable plans with goals, steps, verification, progress tracking, and handoffs. | [planning/SKILL.md](planning/SKILL.md) |
| Project-specific deviations | Records confirmed project behavior and environment quirks that future agents should not rediscover. | [project-specificies/SKILL.md](project-specificies/SKILL.md) |
| Resource-limited testing | Runs resource-intensive commands under platform-appropriate CPU and memory controls. | [resource-limited-testing/SKILL.md](resource-limited-testing/SKILL.md) |

Use a skill only when its frontmatter trigger matches the task or when the
user explicitly requests it. Each skill documents when not to activate.

## One-command installer

Run this command and choose the skills and agent destination interactively:

```bash
curl -fsSL https://raw.githubusercontent.com/tschallacka/ai-skills/refs/heads/master/install.sh | bash
```

The installer can install all three skills or one skill, and supports these
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

If an installed file differs from the repository version, the installer asks
before replacing it. Approved replacements create a `.bak` backup first. A
symlinked skill is skipped for manual review rather than following the link
and modifying an unexpected location.

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

Use `--yes` only when unattended replacement is intentional; changed files are
still backed up before replacement:

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
skips unchanged files, asks before replacing local changes, and creates a
backup for every replaced file.

## Development checkout

Run the installer directly from a checkout to use its local files without
downloading an archive:

```bash
./install.sh
```

The installer also accepts `AI_SKILLS_REPO_URL` and `AI_SKILLS_REF` when a
different repository or branch must be used.

## Notes

- Skills are instructions, not standalone applications. They add no
  dependencies unless a skill explicitly documents one.
- `resource-limited-testing` uses systemd and cgroup v2 for strongest Linux
  isolation, with weaker fallbacks documented for unsupported Linux sessions
  and macOS.
- Read and review third-party skills before enabling them in an agent with
  access to sensitive files, credentials, or external systems.

## License

Distributed under the [MIT License](LICENSE).
