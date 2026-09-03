<!-- MODE: PROD -->
# Todo

**A work queue that outlives the conversation.**

`TODO.json` — one file, read with the `todo` command, holding every task that
must survive a restart, a handoff, or a context compaction. Tasks nest under
tasks, and every closed item carries the evidence that closed it.

## What you get

- **One queue, visible everywhere.** Not five chat transcripts and a sticky
  note — a register any agent reads in one command and prints user-ready.
- **Nesting that matches reality.** Sub-tasks hang off their parent; finishing
  the last one is what the parent's done-note points at.
- **Closures with receipts.** Setting a task `done` without a note is refused.
  Every closed line says *how* it was verified.
- **One binary, not hand-editing.** `todo add` / `todo update` write through the
  same rules the register is read with: an out-of-vocabulary status fails at the
  read, duplicate ids are refused before anything is written, stamps are
  automatic, and sort order is maintained. It needs no shell and no other tool,
  so it works the same under bash, zsh or anything else.
- **A queue that stays short.** `todo prune` moves closed tasks out to a dated
  archive — kept, never deleted — and holds back anything an open task still
  depends on.

## Quick start

> Queue: refactor the retry tests, then update the README table.
> What's still open?
> Close T23 — done, verified by the focused test run.

## Good to know

Before filing three or more items, the skill asks whether you want them
registered — it never auto-files a wall of tasks behind your back. Short
in-chat checklists stay in chat.
