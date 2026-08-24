<!-- MODE: DEV -->
# MEMORY — how work goes wrong in this repo

Not a state file. What is queued lives in `TODO.json`, what is broken in
`BUGS.json`, and what is already fixed lives in the git log — this file holds the
diagnostic lessons that are not rules and not defects, and would otherwise be
rediscovered one painful run at a time.

If a line here contradicts the tree, the tree is right: check before trusting it.
An earlier version of this file had two stale entries and one that was wrong from
the start.

## Where the authority actually lives

| Question | Answer |
|---|---|
| How code is shaped | `CODE-STYLE.md` |
| What a script owes everything else | `CODE-CONTRACTS.md` |
| What breaks on BSD or bash 3.2 | `PORTABILITY.md`, generated from `portability-rules.json` |
| What a change must update | `planning/MAINTAINER.md` §4 |
| How a release is cut | `RELEASE.md` |
| Who receives a file | its own `MODE:` / `PACKAGE:` marker (contract 10a) |
| What is queued or broken | `TODO.json`, `BUGS.json` |
| How the diagrams map to the tree | `planning/ARCHITECTURE.md` |

## The traps that keep producing the work

**A check that produces no output is not a passing check.** The single most
frequent source of wasted work here. A grep or `case` pattern that matches
nothing, a reporter called inside `$( )` so its findings go to a subshell, `set
-e` killing a test at an unguarded command substitution. **Run a positive control
before believing a zero.**

**A probe can be pointed where it cannot discriminate.** Worse than a wrong
answer, because it looks like a right one. A mutation dropping the id sort read
green because the rows it inspected happened to sort identically either way; an
allowlist entry looked load-bearing until its removal changed nothing; a
"reproduction" of a long-path failure could not reproduce it because it skipped
the layer that made the path long. Ask what result would *disprove* the probe.

**A cleanup that scans shared state must know what is its own.** Three separate
defects, same shape: `run-tests.sh` deleting another run's test roots, a test's
own `EXIT` trap silently replacing `lib-test.sh`'s, and the verify harness
sweeping a concurrent run's live worktree. The fix each time is an owner marker —
a run id, a pid — never "it matched my pattern, so it is mine".

**An exit code read through a pipe is the pipe's.** `cmd | tail` then `$?` twice
sent an investigation after a defect that did not exist.

**Set-level checks are not content-level checks.** A release list and an `npm
pack` diff both passed a compiled library with a function missing, because both
compared file *sets*. Compare contents when contents are the claim.

**A guard with no test is a claim.** And a guard inside a `while` at the end of a
pipeline is not even that: it runs in a subshell, and on bash 3.2 `set -e` does
not abort on the pipeline's status, so it refuses nothing. When a backstop looks
untestable, build the seam — a stub on `PATH` is usually enough.

**`jq` given empty input never runs its filter.** It exits 0 having written
nothing, so a writer reports success over a zero-byte file. `jq -e` also exits
**4** on empty output — but only from 1.7: under 1.6 an empty input still
exits **0**, so a guard keyed on `-e`'s status flips with the jq version
(B24). Decide emptiness in the script, not in the tool.

**Documented behaviour can still be a defect.** `--skill a --skill b` discarding
`a` was in the README, which made it documented and no less wrong. The manual was
the thing to change.

**A gate that forces a worse artifact is worse than no gate.** The UI prohibition
matched the whole run cache, so a reporter reworded truthful evidence to get past
it. Scope a rule to the field it is about.

**Back up before mutating, never after**, and commit before running anything
destructive. Work has been lost here to `git checkout --` on an uncommitted file.
The complement bit within one session: a mutation applied by sed hit a different
site than intended, the "restore" step was forgotten once, and the weakened rule
slipped into a commit. **A mutation is not closed until `git diff` shows the tree
back at HEAD** — verify the restore with the same suspicion as the mutation.

**Mutation-test every assertion you add.** An assertion never seen to fail is not
verified, and roughly one in five written here was inert until a mutation proved
otherwise. Two refinements from the same day: aim the mutation at the exact line
(a sed matched an identical pattern in another function, and the test correctly
stayed green), and give each refused class its own fixture — a value carrying
several metacharacters hides a dropped rule behind the others that still fire.

## Verified only on this machine

The bash 3.2 floor is real (the flake builds 3.2.57) but **BSD userland is
verified by CI, not locally**. Three defects reached the tree that only the macOS
legs could see: a GNU-only `\|` alternation in `sed` that silently stripped
nothing, BSD `od` padding a trailing space, and an exact path comparison against
`/var`, which is a symlink there. Do not claim BSD behaviour from reading.

**Run one verification at a time.** A wholesale failure with missing-file errors
is a second verification having deleted the first's worktree, not a regression.
