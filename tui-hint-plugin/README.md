# tui-hint-plugin

A Claude Code plugin that reminds an agent, at the moment it is about to run
a program via a plain Bash call, that this repository ships a reference
profile for that program under interactive-shell/appprofiles/ (or the agent's
own prior session left one under appprofiles.d/) -- and that the program
can usually be driven more effectively through the `interactive-shell` skill
(which gives a real PTY, screen observation, and verified keystrokes) than a
headless invocation, which cannot observe the screen at all.

## Why this exists

interactive-shell/SKILL.md already argues this in general terms: a
headless/piped invocation is a different program from the interactive one,
and answers a different question. This plugin makes that argument concrete
and specific, at the exact moment it matters -- naming the actual program
about to run and pointing at its own shipped or self-written profile --
rather than relying on the agent to remember a general principle from a
skill it may not have loaded recently.

## What it ships

- **`PreToolUse`**, matched to the `Bash` tool only. Reads the command about
  to run, strips a leading `VAR=value` assignment or `sudo`/`env` wrapper,
  and looks for a matching profile in two places, in order:
  - `${XDG_CONFIG_HOME:-~/.config}/tsch-ai-skills/appprofiles/` -- vendor
    profiles, trusted by presence alone.
  - `${XDG_CONFIG_HOME:-~/.config}/tsch-ai-skills/appprofiles.d/` -- the
    vendor directory's agent-writable sibling (interactive-shell/SKILL.md),
    trusted only when the file's first line is the literal marker
    `<!-- tui-app-profile: v1 -->`; an unmarked file is never treated as a
    profile.

  If a profile is found, the hook returns `additionalContext` naming it --
  surfaced to the model itself, not just the user-visible transcript -- and
  always `permissionDecision: "allow"`. **This hook never blocks or modifies
  a tool call**; it is purely advisory.

## Matching is data-driven, not hardcoded per program

The common case is the fast path: a profile whose filename stem equals the
command's leading program word (`mc.md` for `mc`, `nano.md` for `nano`, ...)
matches by that alone. A program whose profile filename isn't simply its
leading word -- a git subcommand (`git bisect` -> `git-bisect.md`), a
flag-gated mode -- declares its own `### Invocation` section: newline-
separated extended regexes, checked against the command line, in
`interactive-shell/appprofiles/FORMAT.md`. Adding a new such profile needs no
change to this plugin's own code; the hook (`hooks/lib.sh`) reads every
profile's declared patterns from disk.

## Why this generalizes, and where it does not (yet)

The delivery mechanism is Claude-Code-specific: `PreToolUse` with
`additionalContext` is this harness's own hook contract. opencode has no
equivalent side channel for a *completed* tool call; its variant
(`opencode/tui-hint-plugin.js`) instead appends the same advisory text to the
bash tool's own result via a `tool.execute.after` hook, which the model reads
back as part of the tool's output -- see
`.agents/knowledge/opencode-plugin-loading-and-advisory-injection.md` for how
that was verified. codex's equivalent is not yet built -- see
chat/SKILL.md's own re-arm mechanism writeup for the shape this repository
uses when a mechanism has to be verified per host rather than assumed to
generalize.

## The profile list is read from disk, not hardcoded

Every filename under `appprofiles/` (minus `FORMAT.md`), plus every marked
file under `appprofiles.d/`, is a candidate; a new profile shipped by a
later interactive-shell release, or a note an agent writes mid-session, is
picked up automatically, with nothing in this plugin to update.
