# Testing story: git-worktrees

## Task given to the agent (verbatim)

I want to try a fairly risky change to this project — reworking how its
config file is loaded — without touching my main checkout while I'm still
figuring it out. Once it's working, merge it back in. Clean up anything
temporary you created for this along the way.

## What "done" looks like

- The agent did the risky work in a **second checkout** (a git worktree),
  not directly in the main checkout's working directory.
- That worktree was created somewhere other than: `/tmp` (or any tmpfs/
  system-temp path), inside the repository itself (e.g. under
  `.git/worktrees`-adjacent paths a naive `git worktree add ../x` pattern
  can still end up nested under, or literally inside the repo root), or
  under a `tsch-ai-skills` install directory. The mandated location is
  `${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-worktrees/<something>`.
- The change was made on its own branch (not directly on `main`), committed
  there, and actually merged back into `main` in the original repository.
- `main` now contains the config-loading change, working-tree-clean, on the
  branch the user actually started on.
- The worktree was removed (`git worktree remove`) and pruned once the
  branch had merged — not left behind as clutter — and the now-merged
  branch was deleted too.

## Why this story

"A change risky enough that you want the main checkout untouched" is one of
the three canonical situations `SKILL.md` opens with, and the exact location
rule (`tsch-ai-worktrees/`, never `/tmp`, never inside the repo, never under
`tsch-ai-skills/`) is stated with unusual force — three explicit "never"s,
each with its own stated cost. A single, otherwise well-behaved agent
skipping straight to the git-textbook `git worktree add ../scratch` (a
sibling of the repo, which is a completely reasonable thing to do without
this skill and is exactly the pattern the skill overrides) is the most
likely and most informative failure mode to check for.

## Known risk areas to watch for in the transcript

- Does the agent place the worktree at
  `${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-worktrees/<name>`, or does it
  default to git's own textbook convention (a sibling directory next to the
  repo, e.g. `../repo-worktree`) which `SKILL.md` never actually forbids by
  name but which the "never inside the repository" / "sibling of
  tsch-ai-skills, never a child" framing is clearly meant to preempt?
- If the harness offers its own "create an isolated worktree for me" tool
  (Claude Code's `EnterWorktree` is the one `SKILL.md` names explicitly),
  does the agent reach for that FIRST — landing the worktree inside the
  repo, which the skill calls out as the wrong outcome — or does it create
  the worktree itself at the mandated path and only then enter it by path,
  matching the documented recovery pattern?
- Does the agent actually remove the worktree and delete the merged branch
  once done ("Removal is the task's last step, not housekeeping to get to
  later"), or leave it sitting on disk because the task's literal wording
  ("merge it back in") doesn't explicitly say "and clean up" as its own
  separate sentence (the task here does say "clean up," deliberately, to
  test whether that's enough)?
- Does the agent name the branch after the task (e.g. `rework-config-load`)
  or after itself/generically (`agent-1`, `worktree`, `tmp`)?
