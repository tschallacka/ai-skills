<!-- MODE: PROD -->
# Git merge resolving

**Resolutions decided by what each side changed, and a merged tree you can trust.**

What a conflicted file should say once both changes exist: how to read each side's
intent out of history, which conflicts are unions rather than choices, why a cleanly
merged file can still be wrong, and how to tell a failure the merge caused from one it
merely inherited.

## What you get

- **A reason for every resolution.** `--ours` and `--theirs` are positions, not
  arguments. The skill asks what each side changed, when, and which side is the file's
  living lineage — so a rewrite that quietly narrows a general check is caught before
  it lands.
- **Unions kept whole.** Two sides adding different things at one place agree
  completely; picking one deletes work. Includes the trap where a conflict region cuts
  across a shared closing brace and stripping the markers stops the file parsing.
- **Ported checks kept honest.** An assertion carries the design it was written
  against. Moving one across a merge can import an invariant that is false on the other
  side — and because it is a test, making it pass drags the code with it. Includes the
  allowlist rule: where both sides edited one, the general form is the one that
  survives the merge.
- **The semantic conflicts git cannot see.** The call sites merge cleanly and the
  declaration conflicts elsewhere, so the merged tree re-declares a dependency nothing
  uses. The skill says what to search for afterwards.
- **Generated files regenerated, not stitched.** Plus the failure that costs an hour:
  an unresolved generated file counts every symbol twice, so ratchets and duplication
  checks report a jump that is not real.
- **Failures attributed before they are fixed.** Run each failing test on both parents.
  Pre-existing failures get reported, not absorbed into the merge commit.
- **Caps measured, not maximised.** A ratchet is not `max()` of the two sides; both
  sides delete too, so the merged count can be lower than either.
- **Orphaned tests inverted and proven.** When a removal is right, the test that
  asserted the old contract is rewritten to pin the new one — then fault-injected, so
  an assertion of absence cannot pass trivially.
- **Someone's dirty checkout left alone.** Uncommitted work joins the merge through a
  temporary index and a scratch worktree instead of a commit they did not ask for.

## When to use it

Reach for it when a merge or rebase stops on conflicts, when two branches that were
each green have to become one tree, or when a merged tree starts failing tests and it
is not yet clear which side owns them.

It does not cover choosing a branching strategy, writing the merge request description
(`merge-request-etiquette`), or worktree bookkeeping and merge ordering
(`git-worktrees`).
