# Testing stories

A **testing story** is a single, realistic task handed to a fresh-context AI
agent (Claude Code, Codex, or OpenCode) that has exactly one of this repo's
skills installed — and nothing else: no other skill, no project notes, no
prior conversation, no hints beyond the skill's own `SKILL.md` and whatever
tools/MCP interface it registers. The agent has to figure out how to do the
task using only what the skill documents. Whatever it gets wrong, guesses at,
or gets stuck on is direct evidence of a gap in that documentation or
interface — not a grade on the agent.

This directory holds one story per shipped skill (see `stories/`), the Docker
images that give each of the three harnesses a genuinely clean environment to
run in (`docker/`), the runner that wires a story to an image
(`run-story.sh`), and the prompt for turning a resulting transcript into
concrete documentation fixes (`analysis-prompt.md`).

## Status

Stories exist for all 17 shipped skills, 8 with a `.fixture.sh` for stories
that need pre-existing state. **Running the actual stories is a separate,
deliberate step** — each real run is a billed agent session, so nothing here
executes automatically; a human decides when to spend that.

The infrastructure itself has been validated end to end for all three
harnesses (image builds cleanly, the installer runs inside the container and
installs the right skill into a fresh `$HOME`, and the harness CLI is invoked
correctly), using invalid/no credentials so the real model call fails
immediately rather than proceeding — no story's real task has been run
against a live model as part of authoring this directory. `gh`/`glab` are
installed in every image so the `ci-failures` story is actually runnable,
not just documented as broken; `run-story.sh` forwards `GH_TOKEN`/
`GITHUB_TOKEN`/`GITLAB_TOKEN`/`GLAB_TOKEN` from the host when set.

Two authored stories surfaced real cross-skill coupling worth knowing about
even before any run: `brainstorm`'s Phase 2 and
`post-implementation-review`'s persona dispatch both hard-depend on the
`planning` skill's `role-context.sh`/`ROLE_ID` persona infrastructure, which
does not exist in a container with only that one skill installed. Each
story's own "Known risk areas" section says so explicitly — worth deciding
whether that's an acceptable documented dependency or something those
skills' own `SKILL.md`s should degrade gracefully without.

Two skills (`ai-text-editor`, `chat`) also support an MCP install mode
(`integration.tsv`) in addition to the default CLI/skill mode every story
here uses. An MCP-mode variant of those two stories — testing the tool
descriptions an agent sees with no surrounding `SKILL.md` prose at all — is
a natural follow-up, not yet written.

## Why Docker, specifically

The one property that matters for a testing story is a **truly fresh
context**, and that's hard to get on a developer's own machine: a real
`$HOME` already has other skills installed, a `CLAUDE.md`, prior sessions,
shell history the agent could stumble onto, maybe even the answer to the
task already sitting in a file somewhere. A container starts every run from
an empty `$HOME` with only the compiled installer's real output on it — the
same thing a brand-new user gets from the one-command installer, not an
approximation of it.

## Running a story

```bash
export ANTHROPIC_API_KEY=...           # or OPENAI_API_KEY for codex
testing-stories/run-story.sh ai-text-editor claude
```

Run this from inside `nix develop .` the first time for a given checkout:
`run-story.sh` needs the compiled `installer` binary and builds it itself
with `cargo build -p installer --release --target x86_64-unknown-linux-musl`
if it isn't already staged in `bin/x86_64-unknown-linux-musl/` or
`target/x86_64-unknown-linux-musl/release/` from an earlier build.

This builds (or reuses, with `--no-build`) the harness's image, installs
exactly the named skill(s) into a clean container, hands the agent the
story's task text as its only instruction, and writes the full transcript to
`testing-stories/runs/<skill>-<harness>-<timestamp>/transcript.jsonl`.

Run the same story across all three harnesses to see whether a gap is
harness-specific (a prompt-shape issue) or genuinely in the skill's own
documentation (every harness trips on it):

```bash
for h in claude codex opencode; do
    testing-stories/run-story.sh planning "$h"
done
```

See `run-story.sh --help`-style usage at the top of the script itself for
`--skills` (installing more than one skill for a combined story) and
`--model`.

## Analyzing a run

Feed `analysis-prompt.md`, the transcript, and the story file to an analyzer
(another agent, or a person) — see that file for the full checklist. A
confirmed finding becomes a `bugs add` entry against the skill so it goes
through the normal fix/verify loop, not a one-off note that gets lost.

## Writing a new story (new skill, or a second story for an existing one)

A story is one Markdown file, `stories/<skill>.md`:

```markdown
# Testing story: <skill>

## Task given to the agent (verbatim)

<the exact text run-story.sh hands the agent as its entire prompt>

## What "done" looks like

<bullet list of observable success criteria for the analyzer to check
the transcript against -- the agent never sees this section>

## Why this story

<1-2 sentences: what part of the skill this exercises and why it matters>

## Known risk areas to watch for in the transcript

<bullet list of specific places worth scrutinizing -- candidate documentation
gaps the story author already suspects, for the analyzer to confirm or rule
out, not to hand the agent>
```

Rules for the task text itself:
- **State a goal, not a procedure.** "Get X working" or "produce Y," never
  "run command A, then B" — the moment the story spells out the skill's own
  commands, it stops testing whether the documentation is discoverable.
- **No skill name-dropping beyond what a real user would say.** A real user
  doesn't say "use the `ai-text-editor` skill's `replace` verb with a
  half-open byte range" — they say what they want done in their own words.
- **One task, one realistic scenario.** Not a checklist of every feature the
  skill has; pick the scenario most likely to expose a real gap (see each
  skill's own `SKILL.md` "when to use" / "when not to" framing for what its
  most load-bearing capability actually is).
- If the task needs pre-existing state (an existing git repo with history,
  a file to edit, a running peer to chat with), add `stories/<skill>.fixture.sh`
  — run by the container as a bash subprocess, with `/workspace` as its cwd,
  before the agent starts. Most stories need none of this; the point of a
  testing story is usually that the agent builds what it needs using the
  skill itself.

## Files

- `stories/*.md` — one testing story per skill.
- `stories/*.fixture.sh` — optional pre-task workspace setup for a story
  (only present where a story actually needs pre-existing state).
- `docker/Dockerfile.{claude,codex,opencode}` — one image per harness.
- `docker/entrypoint.sh` — shared entrypoint baked into all three images:
  installs the named skill(s), runs the optional fixture, invokes the
  harness non-interactively with the story text, and saves the transcript.
- `run-story.sh` — host-side runner: builds the image, wires the story and
  any fixture in, runs the container, reports where the transcript landed.
- `analysis-prompt.md` — how to turn a transcript into concrete
  documentation fixes.
- `runs/` — where `run-story.sh` writes each run's own directory
  (gitignored contents; the directory itself is kept so it always exists).
