---
name: git-worktrees
description: Use when work in one repository needs its own checkout — several agents on separate tasks at once, a long verification that must not see later edits, a risky change kept off the main checkout — and when those branches later have to be merged back. Covers scoping parallel work so it does not collide, the concurrency hazard that silently fails a running harness, and the merge order and conflict classes that follow.
---
<!-- MODE: PROD -->

# Working in git worktrees

A worktree is a second checkout of the same repository on its own branch. It earns
its keep in three situations: several agents working at once without overwriting each
other, a long-running command that must see a fixed tree, and a change risky enough
that you want the main checkout untouched.

The rules below are the ones that cost real debugging time. They are cheap to follow
and expensive to rediscover.

## Several agents at once

**One worktree per agent, one branch per worktree.** Two agents in one checkout
overwrite each other's edits with no error and no conflict marker — git sees a single
working tree and the later write simply wins.

**Give each agent a disjoint file scope, and say so explicitly.** Overlapping scopes
produce conflicts that are tedious rather than informative. When an agent finds it
needs a file outside its scope, it should stop and report that, not reach for it —
the coordinator decides whether to widen the scope, hand the file to another agent,
or serialise the two.

**Name the branch after the task, not the agent.** `fix-roster-abort` survives the
agent that made it; `agent-3` does not.

**Branch from the shared base, not from another agent's branch.** A branch based on
work that is still moving inherits every later force-push and rebase. If two tasks
genuinely depend on each other, they are one task, or they are sequential.

**Never merge an integration branch downward into a task branch.** Integration
branches accumulate everyone's in-flight work; merging one into a task branch drags
that unreviewed work into the task's eventual merge, irreversibly. A task branch that
needs to catch up rebases onto, or merges from, the shared base it started from.

## One long verification at a time

**A second concurrent run can delete the first's worktree.** Harnesses that sweep
stale worktrees before creating their own will remove a live one, because a worktree
in use looks exactly like one abandoned by a crashed run. The first run then fails
across most of its suite with missing-file errors — a signature that reads as a
catastrophic regression and is nothing of the kind.

- Check whether a verification is already running before starting one.
- **When a suite fails wholesale with missing-file or "no such file" errors, suspect
  this before you believe the tree is broken.** Look for a concurrent run, then
  re-run alone.
- Two agents each invoking the harness collide the same way. Serialise them, or give
  each its own worktree *and* its own scratch location.

If a harness sweeps stale worktrees, let it tell live from abandoned by recording the
owning pid and checking it:

```sh
printf '%s\n' "$$" > "$(dirname "$wt")/harness.pid"

owner="$(cat "$(dirname "$stale")/harness.pid" 2>/dev/null || true)"
if [ -n "$owner" ] && kill -0 "$owner" 2>/dev/null; then
    continue          # someone is using it
fi
git worktree remove --force "$stale"
```

`kill -0` tests for the process without signalling it; a stale pid whose process is
gone correctly falls through to removal.

## Merging the work back

**Land one branch at a time, and re-run the gates after each.** Merging three
branches and then testing tells you something broke, not which one. Each merge is a
new state that nothing has verified.

**Merge in dependency order**, and prefer the branch that touches shared or generated
files first — later branches then rebase onto a base that already has it, instead of
conflicting with it.

**Regenerate generated artifacts; never hand-merge them.** A generated installer,
lock file, or compiled bundle will conflict on almost every parallel change, and a
textually merged one is plausible and wrong. Take either side, re-run the generator,
and let the freshness gate confirm.

**Registers and append-only files conflict constantly.** Two agents appending to the
same JSON register (a task list, a bug list, a changelog) collide on the closing
bracket. Resolve by taking one side and re-applying the other's entries with whatever
writer owns the file — not by stitching the text together, which is how ids get
duplicated.

**After merging, remove the worktree**: `git worktree remove <path>`, then
`git worktree prune` to clear records of directories already gone. A stale worktree
holds a branch checked out, so the branch cannot be deleted and later runs may sweep
the directory unexpectedly.

## Staging changes from a worktree

**Never `git add -A` when a separate repository is nested inside the checkout** —
notes, plans, fixtures, a vendored tool. `-A` stages it as a **gitlink**: a bare
commit pointer, silently, with none of its content. The commit looks fine and the
sibling's work is not in it.

Stage explicit paths, or `git add -u` for tracked files only. Before committing,
check `git status --porcelain` for an unexpected mode `160000` entry.

## Traps in the worktree itself

- **A worktree checks out committed state.** A harness meant to verify the *working
  tree* must copy uncommitted changes in, or it silently tests the last commit and
  reports green for code you have not saved. Commit first, or copy the dirty tree
  deliberately.
- **Keep the path short.** A test that opens a unix socket inside the worktree can
  exceed the platform limit — about 104 bytes on macOS/BSD, 108 on Linux — and fail
  with a confusing bind error rather than a length complaint. Put per-test temporary
  directories somewhere short (`mktemp -d /tmp/t.XXXXX`) rather than nested under a
  long worktree path.
- **Do not clobber a trap the test set.** A test installing its own `EXIT` trap
  replaces the harness's, leaking the worktree and its temp directories. Chain traps
  rather than assuming yours is the only one.
- **The parent's configuration still applies**: hooks, `core.hooksPath`,
  `.gitignore`, and the shared `.git/info/exclude`.
- **A branch can only be checked out in one worktree.** `git worktree list` shows
  what exists and on which branch.

## Reporting

Run long commands in the background and report from their output, not from
expectation. If a run was interrupted, aborted, or collided with another, say that
rather than presenting its partial results as a verdict. When several agents report,
say which branch each result came from — a green suite on one branch says nothing
about the merge of all of them.
