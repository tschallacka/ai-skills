<!-- MODE: PROD -->
# Git worktrees

**Parallel agents in one repository, without the trampling.**

Extra checkouts of the same repo so several agents can work at once, a long
verification can run without seeing later edits, or a risky change stays off
your main working tree — and then everything merges back in a sane order.

## What you get

- **Isolation per agent.** Each worker gets its own worktree: its own files,
  its own index, one shared object store. No checkout ping-pong, no
  clobbered files.
- **Verification that stays true.** A long test run in a worktree sees a
  frozen tree — the concurrency hazard where a background build silently
  picks up half-written edits is designed out.
- **A merge order that makes sense.** Merge the least-conflicting branches
  first, re-run the focused tests after each, and the conflict classes that
  remain are the ones worth your attention.

## Quick start

> Give each of the three agents its own worktree on a branch.
> Merge the agent branches back, least-conflicting first.

## Good to know

Worktrees share the repository's objects and refs — cheap on disk, but they
are one repo: branch discipline still applies.
