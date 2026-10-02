# editor-gate-plugin

A Claude Code plugin pairing a hard gate with a soft reminder, both steering
an agent toward the ai-text-editor MCP/skill instead of a shell-driven edit.

## What it ships

- **`PreToolUse`, matched to `Bash`.** Denies a command that rewrites a file
  in place with `sed -i`, `perl -i`, or a heredoc-fed write (a script body
  that opens a path for writing, or a plain `cat > file <<EOF`) -- unless the
  command carries a token minted by `hooks/editor-token`, bound to that
  exact command, single-use, expiring in 120 seconds. Denial only: this hook
  never rewrites the command it refuses.
- **`PreToolUse`, matched to `Edit|Write`.** A non-blocking reminder
  (`additionalContext`, `permissionDecision: "allow"` always) that the
  ai-text-editor MCP/skill journals every change and logs it, on top of the
  `expected_text` mismatch-refusal `sed -i` and a native Edit/Write both
  lack. Never denies; Edit and Write already are the safer native path, this
  only names what the editor adds on top.
- **`hooks/editor-token`.** Mints a token: `--why '<reason>' --command '<exact
  command>'`. Refuses a placeholder reason (under 15 characters). Prints
  `EDIT_OK=<token>` to prefix the authorised command with.

## Why a hard gate on Bash but only a soft one on Edit/Write

`sed -i` and a heredoc write are silent on failure -- `sed -i` exits 0
whether or not its pattern matched, and a heredoc stacks the shell's
escaping on top of the target file's own syntax with nothing checking the
result. Edit and Write are Claude Code's own native tools; they already
verify what they touch. The reminder there is about what the editor adds
(journaling, logging), not about a defect in Edit/Write worth blocking.

## State and token store

Tokens live under `${XDG_STATE_HOME:-~/.local/state}/editor-gate/tokens/`
(mode 700; each token file mode 600), one file per token: expiry epoch, then
the token's normalized command, one file removed the instant it is spent or
found expired. `audit.log` (tab-separated: timestamp, why, command) records
every mint, kept even after its token is consumed or expires, so a mint is
attributable after the fact.
