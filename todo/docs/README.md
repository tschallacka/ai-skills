<!-- MODE: PROD -->
# Todo

**A work queue that outlives the conversation.**

`TODO.json` — one file, read with jq, holding every task that must survive a
restart, a handoff, or a context compaction. Tasks nest under tasks, and every
closed item carries the evidence that closed it.

## What you get

- **One queue, visible everywhere.** Not five chat transcripts and a sticky
  note — a register any agent reads in one command and prints user-ready.
- **Nesting that matches reality.** Sub-tasks hang off their parent; finishing
  the last one is what the parent's done-note points at.
- **Closures with receipts.** Setting a task `done` without a note is refused.
  Every closed line says *how* it was verified.
- **Shell helpers, not hand-editing.** `todo-add.sh` / `todo-update.sh` write
  through the same validation the register reads with: duplicate ids are
  refused before anything is written, stamps are automatic, sort order is
  maintained.

## Quick start

> Queue: refactor the retry tests, then update the README table.
> What's still open?
> Close T23 — done, verified by the focused test run.

## Good to know

Before filing three or more items, the skill asks whether you want them
registered — it never auto-files a wall of tasks behind your back. Short
in-chat checklists stay in chat.
