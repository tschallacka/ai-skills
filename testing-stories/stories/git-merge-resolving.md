# Testing story: git-merge-resolving

## Task given to the agent (verbatim)

Two teammates each added a new command to this project's command registry
on their own branches — `feature/add-export` and `feature/add-import`.
Bring both of those branches into `main` so the final registry has
everything both of them added.

## What "done" looks like

- Both `feature/add-export` and `feature/add-import` end up merged into
  `main` (via merge commits, rebase-and-fast-forward, or any other
  mechanism — the skill takes no position on merge strategy, only on how a
  conflicting hunk gets resolved).
- `commands.md`'s table on `main` afterward contains **all four** rows: the
  two that existed before either branch (`list`, `search`) plus **both**
  new ones (`export` and `import`) — neither new row was dropped.
- No leftover `<<<<<<<`/`=======`/`>>>>>>>` conflict markers anywhere in the
  final tree.
- The table is still valid Markdown (a plausible wrong resolution is
  stitching the two conflicting hunks together in a way that duplicates the
  header separator row or otherwise breaks the table).

## Why this story

Both branches insert a new row **at the exact same position** in the same
file, so a real, unavoidable textual conflict occurs on merge — this is not
a conflict git can auto-resolve. It is precisely the shape `SKILL.md` calls
out as "a union, not a choice": two teammates adding a different thing at
the same place, where the correct resolution is *both*, and where reaching
for `--ours`/`--theirs` (or accepting whichever side's version the tool
shows first without looking at the other) silently deletes one teammate's
work while leaving the merge looking completely clean.

## Known risk areas to watch for in the transcript

- Does the agent recognize the conflict as additive (both changes belong in
  the result) rather than resolving it by picking one side's version of the
  line, which is the fast, plausible-looking, and wrong move here?
- Does the resulting `commands.md` stay valid Markdown (correct row count,
  no doubled separator line, header still intact) after the manual edit, or
  does concatenating both conflicting hunks produce a malformed table?
- Does the agent explain *why* it resolved the conflict the way it did
  (which the skill's own "Landing it" section asks for: "name each
  non-obvious resolution and the reason"), or does it just silently commit
  a merge with no explanation?
- If the agent merges the two feature branches in sequence (rather than
  merging them both into `main` some other way), does the SECOND merge's
  conflict (against the tree that already contains the first branch's row)
  get resolved the same carefully-additive way, or does familiarity with
  the first conflict make the agent careless on the second?
