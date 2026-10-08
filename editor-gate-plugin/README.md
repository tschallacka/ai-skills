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

## State and token store

Tokens live under `${XDG_STATE_HOME:-~/.local/state}/editor-gate/tokens/`
(mode 700; each token file mode 600), one file per token: expiry epoch, then
the token's normalized command, one file removed the instant it is spent or
found expired. `audit.log` (tab-separated: timestamp, why, command) records
every mint, kept even after its token is consumed or expires, so a mint is
attributable after the fact.
