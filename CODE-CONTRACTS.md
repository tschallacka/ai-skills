# Cross-script contracts

`CODE-STYLE.md` says how a shell file is *written*. This file says how a script
must *behave* toward things other scripts and other runs depend on: a document
surface, a generated artifact, a user's file, an exit code, a check's verdict.

Every contract below exists because it was broken, and the breakage is named. A
rule with no incident behind it is a preference; these are not preferences.

Each entry states the rule, what it cost when violated, and what enforces it.
**Enforced** means a test fails if you break it. **Review item** means nothing
catches it but a reader — treat those as the dangerous ones.

---

## 1. A section's shape determines its only legal writer

A generated document's section is exactly one of three shapes, and each has one
writer. Using another writer destroys content.

| shape | looks like | sole writer | wholesale rewrite |
|---|---|---|---|
| narrative | numbered `§ N.M` paragraphs | `-ds`/`-gs`/`-ss`/`-rs`, `-dp`/`-gp`/`-sp`/`-rp` | yes, that is the point |
| fields | `- Label: value` lines | `-f\|--field`, one label per call | **never** |
| table | a Markdown table | a dedicated helper (`add-coverage.sh`, `update-adversarial-review.sh`, `-tp`) | never via a paragraph form |

Two corollaries:

- **A `valid=` allow-list in `plan_mutable_sections` may name only narrative
  sections.** A field- or table-shaped section in that list is a live defect.
- **A field may be owned by a different mechanism than the section it sits in.**
  `- Status:` sits in `## Verdict` but belongs to the review-status gate, so a
  rewrite of that section does not merely lose text — it removes the gate's
  input.

**Cost:** `update-plan-content.sh -rs <plan> rationale` rewrote `## Verdict` and
dropped `- Status:`. The plan became unapprovable — `-rv approved` refuses with
"Review must contain exactly one Status field" — and no helper could restore the
field, because `-f` only updates a label that already exists. Recovery meant
deleting and regenerating `adversarial-review.md`. `-rs review-scope` does the
same to `- Repository/context inspected:` and the plan still *validates*, so
nothing tells you.

**Enforced** by `test-document-sections.sh` against `planning/document-sections.json`.

`planning/document-sections.json` records each section's **heading** as well as
its shape, so a shape claim is checked against the document itself. Deriving the
heading by scraping the library's case statement resolved 29 of 36 sections —
the regex pinned eight spaces of indentation and ids of `[a-z-]+` — leaving six
of the nine field, table and hybrid shapes unverified. The test still passed,
because a scrape that finds less simply verifies less.

**Enforced** by `test-document-sections.sh`: the allow-list may hold only
narrative sections, every registered shape must match the document, and the
library's section-form labels must each yield a pair agreeing with the registry.

## 2. An irreversible step is the last statement of its branch

Anything that cannot be undone — deleting a secret, removing a consumed input,
overwriting a record — goes after every step that can still fail.

**Cost:** twice. `--review-status approved` destroyed the fix-key session secret
*before* two `plan_die` sites and both `mv`s, so one missing `- Status:` line left
a review not-approved **and** permanently un-approvable, needing the fixer to
re-mint and re-claim every gated pair — a deadlock across a role boundary. And
`update-adversarial-review.sh` deleted `adversarial-review-incoming.md`
unconditionally, destroying a reviewer's findings in `--file` mode.

**Enforced** by `test-fix-keys.sh` (the secret survives a failed write, and a
repaired retry then succeeds) and `test-adversarial-review-sources.sh`.

## 3. A refusal must stop its own body

`context_die` and friends **return**; they do not exit. A guard that calls one
must end the branch itself:

```sh
[ -n "$row" ] || { context_die "usage: …"; return; }
```

Without the braces, execution runs on and the caller sees the *follow-on*
failure instead of the refusal.

**Cost:** five sites. `--view testing` on a document with no companion refused,
then ran its `awk` anyway and reported `cannot open …` at exit 2. A malformed
`goal-progress:` id was accepted outright; `step:nogoal` was caught only
downstream by a file-existence check, reporting 66 "missing file" for what is
really a malformed id. I then reproduced the same bug in new code one commit
after documenting it.

**Enforced** by `test-progress-entry-ids.sh` and
`test-plan-context-unit-entry.sh`.

## 4. A sourced library must not change the caller's shell options

No `set -e`/`-u`/`-o pipefail` at the top level of a sourced file. The caller
chose its options; a library that overrides them changes the meaning of the
caller's code.

**Cost:** `lib-test.sh` ran `set -euo pipefail` at load.
`test-plan-context-paging.sh` deliberately runs *without* errexit because it
invokes commands that exit non-zero on purpose, and sourcing the library aborted
it mid-run at exit 65.

**Review item.** `test-plan-context-paging.sh` fails if errexit leaks back in,
but nothing checks the general rule.

## 5. A check either decides exactly, or advises with a remedy

A check that cannot separate a defect from correct usage does not get to fail
the build. It warns, and the warning names what to do instead.

**Cost:** the `--stale` wording sweep failed plans on 13 findings of which **0
were defects**; two independent reviewers dismissed the entire output, which is
the correct response to a gate reporting style as defect — and it means the next
real hit gets dismissed too. Measured precision: count phrases **0 of 24** on
real plans, identical-output phrases **2 of 4**.

The corollary is the useful half: **if the deciding fact exists but is not in the
text, move it into data.** `## Artifact comparisons` declares the artifact and
the comparison, so `exact` on a PDF is refused exactly where a keyword sweep was
a coin flip.

**Enforced** by `test-stale-sweep.sh` (severity and remedy text) and
`test-artifact-comparisons.sh`.

## 6. A message names the remedy, not just the diagnosis

An error says what to run or write next. A diagnosis without an action gets
rationalised away.

**Cost:** one message covered six distinct CSV faults, so a CRLF file was
reported as having the wrong column count and the reader went looking for a
missing comma.

**Enforced** by `test-csv-table-errors.sh` (each fault produces a *distinct*
message, and two faults may not share one) and `test-goal-testing-row.sh` (the
finding names the rebuild helper).

## 7. Generated artifacts are regenerated, never merged or hand-edited

| artifact | built by | checked by |
|---|---|---|
| `install.sh` | `installer/build.sh` | `test-installer-build.sh` |
| `PORTABILITY.md` | `generate-portability.sh` | `test-portability-contract.sh` |
| `planning/REVIEWER.md` | `planning/scripts/generate-reviewer.sh` (pins `SKILL.md`'s SHA-256) | `test-reviewer-projection.sh` |

Regenerate `PORTABILITY.md` **last** in any batch touching `*.sh`: its staleness
signal is commit ordering.

A generator's inputs must be **tracked sources only**. When it walks the
filesystem, prune everything that is not the repo's own source — `.git`,
`.plans`, `.claude`, `benchmark/results` — because a freshness gate that
regenerates and diffs turns any local-only input into a permanent false
`stale` for everybody else.

**Cost:** two worktree merges conflicted *only* in generated files. Merging them
is meaningless — the resolution is to regenerate.

**Cost:** `PORTABILITY.md` was committed with `.claude/worktrees/agent-*/`
paths, so every finding appeared once per agent worktree. The freshness gate
diffs regenerated content against the committed file, so the catalogue read as
fresh on the machine that generated it and stale in every clone —
`test-portability-contract` passed for one person and failed for everyone else.
Verifying in a clean clone is what surfaced it; the working tree cannot.

**Enforced** by the three tests above — but only when they run somewhere that
does not carry the local state. Run the suite in a clean clone before trusting a
generated artifact's freshness check.

## 8. A finite set that needs judgement becomes a registry plus a gate

When a check depends on "which of these is legitimate", the answer belongs in
JSON next to a test, not in a pattern or a maintainer's head:
`placeholders.json`, `state-change-registry.json`,
`never-executable-extensions.json`, `portability-rules.json`, `goal-tables.json`,
`artifact-comparisons.json`, `document-sections.json`, `coupling.tsv`.

A registry read at runtime through `skill_root` **must ship** — see contract 10.

**Enforced** per registry by its own test.

## 9. Destroying a user's content requires proof it is ours, or a recovery path

Before overwriting a file in a user's tree, prove we wrote it and nobody changed
it. Absence of proof means the change must be recoverable.

**A backup file is the recovery path of last resort, not the default.** Inside a
git work tree the user already has history, so writing `.back` files there only
clutters the tree; say what was replaced and let git be the path. Outside a work
tree the backup is the only path, so take it.

**Cost:** the installer decided per *skill*, from the `.version` marker. A
version transition replaced every file "without backups" — including ones the
user had edited, with no git behind them.

**Residual risk, accepted:** an *uncommitted* local edit inside a git work tree
is not recoverable from git, and is now replaced without a backup. The install
names the file it replaced so the loss is visible rather than silent.

**Enforced** by `test-installer-backups.sh` (our own older content is replaced
silently; an edit outside a work tree is preserved; an edit inside one is
reported, not backed up).

## 9a. A helper that discards content names what it discarded

An operation that removes something a person wrote reports each item on stderr,
as a fact after the removal, not a count and not a prediction.

| helper | what it discards | what it says |
|---|---|---|
| `plan-reconcile-lib.sh` | a coverage row that empties | `row dropped`, per row |
| `remove-work-unit.sh --confirm-cascade` | Depends-on edges | one line per pruned edge, plus the restore remedy |
| `add-coverage.sh --replace` | rows sharing the outcome | one line per dropped row, naming its work units |
| `update-work-unit.sh` | the previous value of each field it sets | `replaced WNN <field>: <previous> -> <new>`, one line per field, silent when the value did not change |
| `update-adversarial-review.sh` | the previous Findings table | names the history file it archived to, and the rows are there |

**Cost:** `--confirm-cascade` pruned dependency edges with only `--help`
mentioning it; the reviewer who reported it said a notice would have caught the
finding at the moment it was created. `add-coverage.sh --replace` collapsed three
rows into one and reported `Replaced coverage for W04`, so two rows of a person's
work vanished with nothing naming them.

State it as a fact after the change, not before: a notice printed ahead of the
mutation is a promise that a later failure turns into a lie.

A notice must also be **true**: one that points at an archive which did not
receive the content is worse than silence, because it stops the reader looking.
So the test asserts the destination holds the rows, not merely that the line was
printed.

**Enforced** by `test-plan-commands.sh` (cascade, coverage collapse, replaced
field values) and `test-adversarial-review-sources.sh` (the archive notice and
its truth). All five are mutation-tested.

## 10. A new file under `planning/` ships only if it is registered

Three rows plus a rebuild: `PACKAGE-MANIFEST.txt`, `PACKAGE-MAP.tsv`,
`installer/src/50-manifest.sh`, then `installer/build.sh`.

A registry that is not registered makes the gate that reads it die looking for a
file that was never installed.

**Enforced** by `test-installer-manifest.sh`, and reported by `./blast-radius.sh`
before you commit.

## 11. One id vocabulary across every reader

`plan-content.sh` and `plan-context.sh` accept the same document ids, and an id
names the document it serves. Two spellings for one document is the defect, so
a rename is a rename — never an alias.

**Cost:** one document was `review` in one reader and `adversarial-review` in the
other, while five documents a reviewer must audit were readable only through the
ungated path that `SKILL.md` prohibits.

**Enforced** by `test-document-id-parity.sh`.

## 12. Exit codes stay distinguishable

`64` you invoked this wrong · `65` data present but malformed · `66` a required
input is missing · `69` a tool is unavailable · `70` our bug · `73` refusing to
overwrite · `78` the environment is wrong. Full table in `CODE-STYLE.md` §5.

**A caller must be able to tell "not there" from "you typed it wrong."**

**Cost:** "Document not found" exited 64 at nine sites, so absent files were
indistinguishable from rejected ids — which made my own first measurement of the
reader vocabularies wrong.

**Enforced** by `test-document-id-parity.sh`.

## 13. The plan directory is accepted both ways

Positionally and as `--plan-dir`. A helper that legitimately takes a plans
*root* (`cleanup-plans.sh`) must not accept the flag, because it would name the
wrong thing.

**Enforced** by `test-plan-dir-synonym.sh` and
`test-flag-form-equivalence.sh` — each case also asserting the invocation *had
an effect*, because two equally broken paths compare equal.

## 14. A test reports every finding, and its findings survive a subshell

Report all findings then exit once; record them through `t_record` so a finding
raised inside `$( )` is not discarded with the subshell. Bare `[ … ]` tests call
`t_trap_assertions`. Details in `CODE-STYLE.md` §12.

**Cost:** 16 test files exited 1 with **zero output**. Separately, a reporter
called inside a command substitution left its exit-code assertions inert until a
mutation exposed them.

**Enforced** by `test-portable-helpers.sh` (the subshell property) and by
`CODE-STYLE.md` §12's own assertions.

## 15. A generated document is for a human to read

Markdown is the format because a person has to be able to open a plan and follow
it — to review a decomposition, audit a finding, or check a proof — **without
running a helper to interpret it**. The helpers are the only writers; humans are
the primary readers. If a document only makes sense through a script, the format
has failed and the review it exists to support cannot happen.

What that obliges of a script that writes one:

- **It reads top to bottom.** Headings, `§ N.M` paragraphs and tables exist to
  make a document scannable by eye, not to encode machine state.
- **Machine-only state stays out of it.** Manifests, digests, keys and caches
  live in `.env`, `.filehashes`, `fix-keys.json`, `context/` — never smuggled
  into readable prose.
- **An unfilled slot is obvious to a person.** `<like this>`, registered in
  `placeholders.json`, so a reader sees an unanswered question rather than a
  sentence that merely reads oddly.
- **Prefer the name over the identifier.** A work unit shows its file and its
  intended change, not just `W07`; a finding states what was wrong, not a code.
- **A message names something a human can open** — the document, the section, the
  row — because the reader's next move is to go look at it.

**Cost:** this is the reason a table must not be rewritten as paragraphs and a
field section must not be flattened (contract 1). Those are not merely lossy to a
parser: they destroy the structure a reviewer navigates by.

**Review item.** The placeholder registry, the paragraph contract and contract 1
enforce parts of it; whether a document is genuinely followable is a judgement a
reader makes. Read the document you just generated before believing it is
finished.

---

## Writing a new script

1. Read `CODE-STYLE.md` for the shape of the file, then this document for what it
   owes everything else.
2. If it writes a document surface, find the section's shape in
   `planning/document-sections.json` and use that shape's writer (contract 1).
3. If it makes a judgement about a finite set, add a registry and a gate rather
   than a pattern (contract 8).
4. Put anything irreversible last (contract 2), make refusals stop their own
   branch (contract 3), and give every message a remedy (contract 6).
5. Register it if it ships (contract 10), then run `./blast-radius.sh`.
6. Open the document it wrote and read it as a reviewer would (contract 15).
7. Mutation-test every assertion you add: revert the fix, watch the test fail,
   restore. An assertion never seen to fail is not verified.
