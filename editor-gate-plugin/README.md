# editor-gate-plugin

A Claude Code plugin pairing a hard gate with soft reminders, all steering
an agent toward the ai-text-editor MCP/skill instead of a shell-driven edit
or a plain Read/Edit/Write.

## What it ships

- **`PreToolUse`, matched to `Bash`.** Denies a command that rewrites a file
  in place with `sed -i`, `perl -i`, or a heredoc-fed write (a script body
  that opens a path for writing, or a plain `cat > file <<EOF`) -- unless the
  command carries a token minted by `hooks/editor-token`, bound to that
  exact command, single-use, expiring in 120 seconds. Denial only: this hook
  never rewrites the command it refuses.
- **`PreToolUse`, matched to `Edit|Write`.** A non-blocking reminder
  (`additionalContext`, `permissionDecision: "allow"` always) naming the gap
  Edit's own uniqueness check does not cover: whether the file changed on
  disk since it was last read, not just whether the match text is
  unambiguous. ai-text-editor's `expected_text` and revision guard catch
  that case and refuse instead; its journal also survives a git checkout
  that would discard the edit. Never denies; Edit and Write already are the
  safer native path, this only names what the editor adds on top.
- **`PreToolUse`, matched to `Read`.** A non-blocking reminder that
  ai-text-editor's `search` (exact, regex, or fuzzy) can often answer what a
  full `Read` is about to spend context pulling in whole, and that a search
  hit's own byte range addresses a later `replace` directly -- no need to
  re-quote the matched text as Edit's `old_string` requires.
- **`PreToolUse`, matched to `Bash` and `Edit|Write|MultiEdit|Grep`.** Inert
  unless strict mode is on; see "Strict mode" below.
- **`hooks/editor-token`.** Mints a token: `--why '<reason>' --command '<exact
  command>'`. Refuses a placeholder reason (under 15 characters). Prints
  `EDIT_OK=<token>` to prefix the authorised command with.

## Why a hard gate on Bash but only a soft one on Edit/Write and Read

`sed -i` and a heredoc write are silent on failure -- `sed -i` exits 0
whether or not its pattern matched, and a heredoc stacks the shell's
escaping on top of the target file's own syntax with nothing checking the
result. Edit and Write are Claude Code's own native tools; they already
verify what they touch, and Read is never wrong, only possibly more
expensive than it needed to be. Neither is a defect worth blocking --
the reminders only name what the editor adds on top.

## Why the Edit/Write reminder leads with disk-divergence, not features

An earlier version of the Edit/Write reminder just listed what the editor
adds (journaling, mismatch-refusal) as features. A live-steered benchmark
session (`benchmark/ai-text-editor-usage/FINDINGS.md`) read that version on
every Edit call and, when asked afterward why it never reached for the
editor, said its own reasoning was "Edit's uniqueness check already gives me
mismatch protection" -- conflating "my match is unambiguous" with "the file
hasn't changed since I read it," which Edit's check cannot see at all. The
current wording leads with that distinction instead of leaving it implicit,
because listing journaling and expected_text as parallel features let a real
session read right past the one that actually mattered for its task.

## Strict mode (optional, recommended)

The default above leaves the choice with the agent, and an agent told to use
`grep` or `sed` by something injected into its context takes that instruction
over a reminder. Strict mode removes the choice. It is off unless you turn it on:

```bash
installer install --agent claude --skill ai-text-editor --editor-gate strict
```

An interactive install asks, marked as recommended. `--yes` and an "All" answer
never choose it. A reinstall keeps whatever was chosen, and `--editor-gate current`
turns it back off.

With it on, `hooks/pre-tool-use-strict.sh` hands every call to
`ai-text-editor strict-gate`, and these are denied with **no token**, because an
agent can mint a token itself:

- the native `Edit`, `Write` and `MultiEdit` tools, and native `Grep` on one file
- `grep`, `rg`, `ack` or `ag` on one file, including `grep x < file` and
  `cat`, `head` or `tail` of a file piped into one
- `sed` and `awk` used as a search over one file, and `perl -ne` or `ruby -ne`
- an inline or small script that only reads one file and searches it
- `echo`, `printf`, `tee` or a heredoc `cat` writing to a real path

Still allowed: plan mode and its plan file; searching a command's output, such as
`git log | grep x` or `tail -f log | grep x`; searching many files; non-search
reads such as `sed -n '10,20p' file`; and shell writes to temp paths and `/dev`.

Two things to know before relying on it:

- **It fails closed.** A missing client, or one older than this plugin, denies
  every gated call and says to re-run the installer. Start a session with
  `EDITOR_GATE_OFF=1` to get out of that; a prefix on an agent's own command
  cannot set it, because the hook reads the harness's environment.
- **It reads shell text, so it is not a sandbox.** `cp`, `patch`, `git apply` and
  `ed` still write files, and a large script is not inspected. It stops the reflex
  and an injected instruction, not an agent determined to get around it.

## State and token store

Tokens live under `${XDG_STATE_HOME:-~/.local/state}/editor-gate/tokens/`
(mode 700; each token file mode 600), one file per token: expiry epoch, then
the token's normalized command, one file removed the instant it is spent or
found expired. `audit.log` (tab-separated: timestamp, why, command) records
every mint, kept even after its token is consumed or expires, so a mint is
attributable after the fact.
