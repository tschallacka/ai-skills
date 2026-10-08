<!-- MODE: PROD -->
# Bug report

**Every defect, one file, full story.**

When a bug is found but not fixed in the same breath, it goes here: `BUGS.json`
— one register read with rjq, where each entry carries its reproduction, the
measurement that proves it real, the mechanism once known, and the
verification that fails without the fix.

## What you get

- **Reproduction first.** A report without one is a rumour; the register
  refuses to accept it.
- **Observed vs expected.** The two sentences that define a defect, side by
  side, forever.
- **Closures with proof.** Marking a bug `fixed` requires the fix and *how it
  was verified* — including the mutation check. No silent healing.
- **One binary, not hand-editing.** `bugs add` / `bugs update` write
  through shared validation, so a malformed entry cannot land. Refusals name
  their reason; a rejected write leaves the register exactly as it was.

## Quick start

> File a bug: checkout fails with exit 73 when the target dir is a symlink.
> Repro: `installer install --all --target ~/current` twice.
> Close B31 as fixed — fix abc123, verified by the macOS CI leg going green.

## Where BUGS.json actually lives

The first time `bugs` (or `todo`/`decisions`) runs in a project with
nothing decided yet, and only on a real terminal, it asks once:

```
No registers worktree is set up for this project yet. Create a dedicated
sparse git worktree for BUGS.json/TODO.json/DECISIONS.json on branch
"registers" (recommended -- avoids merge conflicts between concurrent
agents)? [y/n, default: y]
```

Accepting the default (`y`, or an empty line/Enter) creates a sparse git
worktree — holding only `BUGS.json`, `TODO.json` and `DECISIONS.json`, not
the whole project tree — on branch `registers`, shared with `todo` and
`decisions` for the same project, and remembers the choice: no later
invocation, from any of the three tools, ever asks again. Declining offers
a different branch name instead of an immediate no; only an empty answer
there is a real decline. Once declined, or once accepted, the choice
sticks — there is no re-offer-later mechanism short of removing the
recorded marker or config by hand.

A non-interactive run (CI, a piped invocation) never sees this question at
all: it silently keeps today's plain `./BUGS.json` behavior, prints a note
saying so, and — deliberately — never records a decline on your behalf, so
a later interactive run still gets asked for real.

An explicit `--file PATH` or `BUGS_JSON` environment variable always wins
over all of this, exactly as before this existed. `bugs resolve-path`
prints the path a mutating command would actually use — without ever
prompting or creating anything — for a script or a mod to find the real
file instead of guessing the project root.

Both the worktree choice and the plain default are also real, directly
settable config values (no interactive prompt required): a project can
pre-seed `registers_access` (`dedicated-worktree` or `main-checkout`) and
`registers_branch` in its own tsch-ai-skills config file ahead of time.

## Good to know

This is for defects. Work that is merely queued belongs to the todo skill; a
design preference is neither — it belongs in a decision, not a register.
