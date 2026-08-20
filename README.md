# AI Skills

A small collection of reusable `SKILL.md` instructions for coding agents.
The skills are plain Markdown, version-controlled, and portable across
compatible agent tools.

## Skills

| Skill | Purpose | Documentation |
|---|---|---|
| Planning | Creates durable plans with goals, steps, verification, progress tracking, and handoffs. | [planning/SKILL.md](planning/SKILL.md) |
| Todo | A queue of work that outlives the conversation, in one JSON file read with jq: items nest under items, every closed item carries its evidence, and the read recipes print user-ready output so nothing is reformatted by hand. | [todo/SKILL.md](todo/SKILL.md) |
| Brainstorm | Shapes an under-specified idea into a recorded, agreed picture (`brainstorm.md`) before planning, with an adversarial completion pass and a plan-vs-implement gate. | [brainstorm/SKILL.md](brainstorm/SKILL.md) |
| Post-implementation review | After-the-fact review of built code with proposed fixes, backed by implementer analysis, an independent solutions agent, and a critical-feedback agent. | [post-implementation-review/SKILL.md](post-implementation-review/SKILL.md) |
| Project-specific deviations | Records confirmed project behavior and environment quirks that future agents should not rediscover. | [project-specificies/SKILL.md](project-specificies/SKILL.md) |
| Resource-limited testing | Runs resource-intensive commands under platform-appropriate CPU and memory controls. | [resource-limited-testing/SKILL.md](resource-limited-testing/SKILL.md) |

Use a skill only when its frontmatter trigger matches the task or when the
user explicitly requests it. Each skill documents when not to activate.

## Supported platforms

"Portable" elsewhere in this repository means portable across agent tools
(Claude Code, Codex, OpenCode, OpenClaw, Cline). Operating-system support is
separate and stated here:

| | Supported |
|---|---|
| Linux | any distribution, bash 4 or 5, GNU userland |
| macOS | 11+ with the stock `/bin/bash` 3.2, BSD userland; Homebrew bash not required |
| Windows | via WSL2, which is a Linux install. Git Bash / MSYS / Cygwin get dependency hints from the installer but are untested and have no CI leg |

The installer and the helper scripts need `bash`, POSIX `coreutils`, `awk`,
`sed`, `grep`, `git`, and `curl` for the one-command install. The `planning`
skill additionally needs `jq`, and on macOS `resource-limited-testing` needs
`memlimit`; the installer checks for both up front and prints a per-platform
install hint rather than failing partway through. Those two are the only extra
runtime dependencies any skill has — in particular `python3` is **not** required
by anything that gets installed, only by this repository's own benchmark
harness. `CODE-STYLE.md` is the contract these scripts are held to,
and CI runs the test suite on Linux and macOS — including under macOS's
bash 3.2.

One skill is genuinely OS-scoped: `resource-limited-testing` enforces a *hard*
RAM cap only on Linux, via a transient systemd `--user` cgroup v2 scope. On
macOS it uses [memlimit](https://github.com/pingiun/memlimit) (MIT, by Jelle
Besseling), which refuses allocations past the cap instead of killing the
process — a best-effort cap on real resident memory over the process tree, not a
cgroup-equivalent guarantee; `SKILL.md` lists what it does and does not promise.
On Apple Silicon macOS the installer declares memlimit a **soft** requirement:
without it the skill still installs and the run warns that the RAM cap is not
enforced. The wrapper then degrades to `nice` plus `cpulimit` (CPU throttling
only); on a Linux session without a user systemd instance it falls back to
`ulimit -v`. memlimit is not asked for on Intel Macs at all — it does not support
them. Every other skill behaves identically on both.

## One-command installer

Run this command and choose the skills and agent destination interactively:

```bash
curl -fsSL https://raw.githubusercontent.com/tschallacka/ai-skills/master/install.sh | bash
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

### Updating an existing install

Run the installer again against the same root. It compares every managed file
with the repository copy and reports one of three outcomes per destination:
`Up to date` (nothing differed), `Installed` (files were written), or `Skipped`
(you declined, or the destination needs manual review).

Each installed skill contains a `.version` marker identifying its tag, branch,
and commit. If an installed file differs from the repository version, the
installer asks before replacing it. For a managed version transition that the
user approves — the `.version` marker differs, so the change came from a new
release rather than from you — old files are replaced without backups; the
previous version can be restored by running the installer against its tag with
`AI_SKILLS_REF`. Unmanaged changes still receive `<file>.bak` backups (`.bak.1`,
`.bak.2`, … if one already exists). A symlinked skill is skipped for manual
review rather than following the link and modifying an unexpected location.

### Installing or updating one skill

Interactively, choose that one skill at the menu. Headless, name it:

```bash
curl -fsSL https://raw.githubusercontent.com/tschallacka/ai-skills/master/install.sh \
  | bash -s -- --skill planning --target "$HOME/.codex/skills"
```

`--skill` takes **one selection**, but that selection may be a comma-separated
list — `--skill planning,brainstorm`. Repeating the flag does not accumulate;
the last one wins. Menu numbers work too (`--skill 1,4`).

### Headless and CI usage

Use `--all`, `--skill`, `--target` and `--yes` when the choices are already
known. `--target` takes a single root, so installing into two roots is two runs.

```bash
# Install all skills into the shared Agent Skills root
curl -fsSL https://raw.githubusercontent.com/tschallacka/ai-skills/master/install.sh \
  | bash -s -- --all --target "$HOME/.agents/skills"

# Unattended replacement: managed version transitions replace without backups,
# unmanaged changed files are still backed up as <file>.bak
curl -fsSL https://raw.githubusercontent.com/tschallacka/ai-skills/master/install.sh \
  | bash -s -- --all --target "$HOME/.agents/skills" --yes
```

Every run ends with a **summary block on stdout** saying what was installed,
what was not, and why; the progress and diagnostics go to stderr, so
`install.sh … > summary.txt` keeps the outcome and `2>/dev/null` keeps it
readable. A blocked skill is reported once — not once per root — with the
commands that finish the job. This is what an `--all` run on a machine without
`jq` prints:

```
== Summary ==
Installed: /home/u/.agents/skills/project-specificies
Installed: /home/u/.agents/skills/resource-limited-testing
Installed: /home/u/.agents/skills/brainstorm
Installed: /home/u/.agents/skills/post-implementation-review
Skipped:   planning — a hard requirement is missing, nothing was written
To install planning once its requirements are met:
  1. install jq:
    sudo apt-get install -y jq
  2. replay this run:
  ./install.sh --skill planning --target /home/u/.agents/skills --yes
```

The install step is chosen for the detected platform and package manager, and
the replay line carries the same target and flags as the run that printed it. In
a piped run there is no script on disk, so the replay is emitted as the
`curl … | bash -s -- …` form instead of a path. The exit status is non-zero,
because four of five skills is a partial install and CI must not read it as
success.

### Runtime dependencies

Dependencies are declared **per skill**, so one unsatisfiable dependency never
stops the other skills from installing. Each skill ships a `requires.tsv` naming
what it needs, on which platform and architecture, and how badly:

| Strength | Effect |
|---|---|
| `hard` | The skill does not work without the tool. It is **not installed**, the run explains why and prints the replay commands, and the exit status is non-zero. |
| `soft` | The skill works in a degraded form. It **is installed**, with a warning naming the tool and the capability that is lost. The exit status is unaffected. |

Currently:

- `planning` requires `jq` (**hard**, every platform) — without it
  `validate-plan.sh` refuses to run and the plan gates stop firing.
- `resource-limited-testing` names `memlimit` (**soft**, Apple Silicon macOS
  only) — see [Supported platforms](#supported-platforms) above for what the
  degraded path still does.

No other skill has a runtime dependency.

### Exit codes

| Code | Meaning |
|---|---|
| 0 | Everything requested was installed. Soft warnings do not change this. |
| 1 | A requested skill was blocked by a hard requirement, or any other error. |
| 2 | `--install-skill` only: approval declined, nothing was written. |
| 3 | `--install-skill` only: an unsafe collision (an existing file that is not a managed version upgrade, or a symlink). |

Codes 2 and 3 belong to the machine-facing `--install-skill` mode that the
planning skill's own tooling uses; the interactive and `--all`/`--skill` paths
only ever return 0 or 1.

### Full-screen installer UI

*Placeholder — `install-ui.sh`, a full-screen terminal UI for the same
installer, is in development and is not yet wired into `install.sh`. Its
keybindings will be documented here once it is integrated.*

Review the installer before running it if you do not trust the source. Skills
are instructions that may guide agents to run commands or access files.

`install.sh` is a generated artifact assembled from `installer/src/` — see
[CONTRIBUTING.md](CONTRIBUTING.md) before editing it.

## Supported agent documentation

- [Agent Skills](https://agentskills.io/)
- [Codex skills](https://github.com/openai/codex/tree/main/codex-rs/skills)
- [Claude Code skills](https://code.claude.com/docs/en/agent-sdk/skills)
- [OpenCode skills](https://opencode.ai/docs/skills)
- [OpenClaw skills](https://docs.openclaw.ai/skills)
- [Cline skills](https://docs.cline.bot/customization/skills)


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
- `resource-limited-testing`'s platform behaviour is described under
  [Supported platforms](#supported-platforms); its `SKILL.md` documents each
  fallback in detail.
- Read and review third-party skills before enabling them in an agent with
  access to sensitive files, credentials, or external systems.

## License

Distributed under the [MIT License](LICENSE).
