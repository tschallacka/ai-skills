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

## Where TODO.json actually lives

The first time `todo` (or `bugs`/`decisions`) runs in a project with
nothing decided yet, and only on a real terminal, it asks once:

```
No registers worktree is set up for this project yet. Create a dedicated
sparse git worktree for BUGS.json/TODO.json/DECISIONS.json on branch
"registers" (recommended -- avoids merge conflicts between concurrent
agents)? [y/n, default: y]
```

Accepting the default (`y`, or an empty line/Enter) creates a sparse git
worktree — holding only `BUGS.json`, `TODO.json` and `DECISIONS.json`, not
the whole project tree — on branch `registers`, shared with `bugs` and
`decisions` for the same project, and remembers the choice: no later
invocation, from any of the three tools, ever asks again. Declining offers
a different branch name instead of an immediate no; only an empty answer
there is a real decline. Once declined, or once accepted, the choice
sticks — there is no re-offer-later mechanism short of removing the
recorded marker or config by hand.

A non-interactive run (CI, a piped invocation) never sees this question at
all: it silently keeps today's plain `./TODO.json` behavior, prints a note
saying so, and — deliberately — never records a decline on your behalf, so
a later interactive run still gets asked for real.

An explicit `--file PATH` or `TODO_JSON` environment variable always wins
over all of this, exactly as before this existed. `todo resolve-path`
prints the path a mutating command would actually use — without ever
prompting or creating anything — for a script or a mod to find the real
file instead of guessing the project root.

Both the worktree choice and the plain default are also real, directly
settable config values (no interactive prompt required): a project can
pre-seed `registers_access` (`dedicated-worktree` or `main-checkout`) and
`registers_branch` in its own tsch-ai-skills config file ahead of time.

## Good to know

Before filing three or more items, the skill asks whether you want them
registered — it never auto-files a wall of tasks behind your back. Short
in-chat checklists stay in chat.
