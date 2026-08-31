---
name: git-merge-resolving
description: Use when a merge or rebase stops on conflicts, when two branches that were each green have to become one tree, or when deciding what a resolved file should say. Covers reading each side's intent from history, the conflicts that are unions rather than choices, generated files, and proving which post-merge failures the merge actually caused. Do not use for choosing a branching strategy, for writing the merge request description (see merge-request-etiquette), or for routine worktree bookkeeping (see git-worktrees).
---
<!-- MODE: PROD -->

# Resolving a merge

A conflict marker is not a question about which side wins. It is a report that two
edits touched the same lines, and the answer is usually neither side verbatim. The
work is to reconstruct what the file should say now that both changes exist, then to
prove the result is a tree nobody has built or tested before — because it is.

## Resolve by content, never by side

`--ours` and `--theirs` name positions, not arguments. Reaching for one is how a
resolution gets made without a reason, and the cost is invisible: the merge succeeds,
the tests that remain still pass, and the coverage the discarded side carried is gone.

Before deciding, answer three questions about the conflicting hunk:

- **What did each side change, and why?** `git log -p <ref> -- <file>` for each branch,
  and read the commit messages. A commit that says "ship two binaries per skill" tells
  you the shape the file is moving toward.
- **When was each change made, and which is the file's live lineage?** `git log
  --format='%h %ad %s' --date=short -- <file>` on both sides. Recency is not authority
  on its own, but a side that has kept evolving a file usually should keep it.
- **Did one side rewrite what the other extended?** This is the dangerous shape. A
  wholesale rewrite that narrows a general thing to the author's immediate case looks
  like a clean, modern replacement and silently drops every check it did not need.

Where one side rewrote and the other evolved, take the evolved lineage as the base and
port the rewrite's genuine additions into it. Then say in the commit message which arm
came from where, so the next person does not have to re-derive it.

### A ported check carries the assumptions it was written under

An assertion is only true of the design it was written against. Moving one across a
merge moves an invariant that may hold on one side and be false on the other — and
because it is a test, making it pass then drags the *code* toward the assumption.

The shape to watch for: one side delivers something unconditionally, the other selects
per host or per platform. A check that says "the manifest must name every declared
artifact" is right for the first and wrong for the second, and satisfying it means
declaring artifacts that do not exist. That breaks the real operation the manifest
drives while every test still looks satisfiable.

So before porting a check, find the mechanism it assumes and confirm the merged tree
has only that mechanism. If both mechanisms now live in the tree, the check belongs to
neither — leave it out rather than bending the code to it.

**Where both sides edited the same allowlist, prefer the general form.** One side
narrows an exemption to the specific names it knows; the other keeps a broad pattern
with a reason attached. After the merge there are more names than either side listed,
so the narrow form fails on the ones it never heard of. The broad pattern was written
by whoever already had two things to cover.

## Some conflicts are unions, not choices

Two sides adding a different thing at the same place conflict textually and agree
completely. Two new functions at the same point in a file, two entries appended to the
same list, two rows in the same table — the resolution is both, and picking one is a
silent deletion.

A registry that gained a second kind of entry is the common case: if a table keyed by
one column now holds two rows per key, check that its **consumers** were updated too.
The table and the code that reads it conflict in different files, and only one of them
may have shown up as a conflict.

**A conflict region can cut across a shared trailer.** When both sides' hunks end
mid-construct — an unclosed brace, an open `if` — the closing lines sit *below* the
`>>>>>>>` marker and belong to whichever side survives. Stripping the three markers and
keeping both halves then produces a file that will not parse. Read the lines after the
marker before assuming both blocks are self-contained, and re-close the first block
yourself.

## A file that merged cleanly can still be wrong

Git merges lines; it does not know what they mean. The semantic conflict usually lives
in files that never conflicted:

- A side removes a dependency by rewriting every call site. Those call sites merge
  cleanly. The **declaration** of the dependency conflicts somewhere else, and taking
  the wrong side re-declares something nothing uses any more.
- A side deletes a file the other side only edited. Git asks about the deletion
  (modify/delete) but nothing asks whether the replacement really covers it.

So after resolving, search the merged tree for the thing that was supposed to go and
confirm it is gone from live code — not just from the file that conflicted. If the
answer is that the replacement takes over, verify the replacement does not quietly
reintroduce the same dependency.

**Deletion beats a rename inside the deleted file.** When one side removes a script
outright and the other only renamed a symbol within it, the deletion wins; the rename
was maintenance on something that no longer exists.

## Generated files are regenerated, never merged

Installers, lock files, compiled tables, projections carrying a source hash, anything
stamped with a timestamp: take either side, re-run the generator, commit that. A
hand-merged generated file looks plausible and is wrong.

The failure mode worth knowing: **an unresolved generated file poisons measurements
taken from the tree.** A conflicted file still on disk contains *both* versions, so
every function, row and symbol in it counts twice. A ratchet or a duplication check run
against that tree reports a large jump that has nothing to do with the code. Resolve
and regenerate before trusting any count.

## Prove which failures the merge caused

A merged tree is new, so its test results mean nothing until they are attributed. For
every failure, run the same test on each parent before touching it:

- **Fails on a parent too** — pre-existing. Not yours to fix inside the merge, and
  fixing it here hides where it came from.
- **Passes on both parents, fails merged** — caused by the merge, and yours.

This is the difference between a merge commit that resolves a merge and one that
quietly absorbs somebody's unfinished work. Report the pre-existing failures plainly,
with the control result that proves they predate the merge, and leave them.

**Counters and caps are not `max()` of the two sides.** A ratchet, a cap, an allowed
count of exceptions: both sides may add *and* remove entries, so the merged number can
be lower than either side's. Measure it against the merged tree and set it to what you
measure. For a ratchet that only ever tightens, taking the larger side is a silent
loosening.

## When a declaration goes, its tests go with it

Removing a capability orphans the tests that assert it, and they fail on the merged
tree even though the removal was right. Do not delete such a test to make the suite
green — invert it to pin the new contract: the thing is absent, and installing or
running without it behaves correctly.

Then **fault-inject to prove the inverted assertion still bites.** Re-introduce what you
removed, confirm the test fails, and put it back. An assertion that something is absent
passes trivially against a tree where the check itself is broken.

## Do not finish the other side's unfinished work

A merge often exposes work that was mid-flight: a plan with open steps, a marker
convention half-applied. Completing it inside the merge commit mixes two changes and
makes both harder to review or revert. Resolve the conflict, note what is unfinished
and whose it is, and leave it.

## Merging around a dirty checkout

Uncommitted work is still a side of the merge, but committing it in someone's working
copy to get at it is a change they did not ask for. Build the merge without disturbing
them: write the tree to a temporary index (`GIT_INDEX_FILE`), `git commit-tree` it
against `HEAD` for a throwaway commit, and merge that in a scratch worktree. The
working copy stays exactly as it was, and `git merge-tree --write-tree` will list the
conflicts before committing to anything.

Exclude nested repositories from that snapshot. A directory with its own `.git` stages
as a gitlink and drags a submodule-shaped entry into the merge.

## Landing it

Land one branch at a time and re-run the suite after each; merging three branches and
then testing tells you something broke, not which one. Write the merge commit so it
names each non-obvious resolution and the reason — what each side was doing, which
lineage you kept, and what you ported across. A resolution nobody can reconstruct is
re-litigated at the next conflict in the same file.
