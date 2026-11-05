# tailpipe-gate-plugin

A Claude Code plugin with one soft reminder, steering an agent toward
`tailpipe` instead of a shell pipe or redirect when it is about to watch an
ongoing command's output rather than read something that has already
finished.

## What it ships

- **`PreToolUse`, matched to `Bash`.** A non-blocking reminder
  (`additionalContext`, `permissionDecision: "allow"` always) on a command
  that matches either:
  1. `tail` given a follow-style flag (`-f`, `-F`, or `--follow`) -- the
     direct "live-follow a file or a process's output" case.
  2. A backgrounded command (trailing `&`) that also redirects or pipes its
     own output (`>`, `>>`, `2>&1`, or `|`) -- "start something long-running
     and plan to come back to its log".

  Deliberately narrower than "any pipe into `less`/`head`": that would fire
  on everyday, already-finished-by-the-time-you-read-it commands (`ls |
  less`) that have nothing to do with watching an ongoing process. Never
  denies -- a pipe or a redirect always works; this only names what
  `tailpipe` adds on top: every line stays queryable by id, and a later poll
  does not need to re-read from the start.

## Why soft, not a hard gate

Unlike `editor-gate-plugin`'s `sed -i`/heredoc gate, piping a command's
output to a pager or a plain redirect is not a silent-failure risk -- nothing
is lost or corrupted, it is only less convenient to search or poll later than
a `tailpipe` stream would be. That is a reminder's job, not a gate's.

## State

None of its own. The reminder text is stateless; `tailpipe-server-rs` owns
whatever stream state an agent that follows the reminder ends up creating
(see the `tailpipe` skill).
