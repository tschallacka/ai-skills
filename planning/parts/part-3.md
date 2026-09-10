<!-- MODE: PROD -->
> Generated from `skill-source.txt` by `scripts/generate-skill-docs.sh` — do not edit.
> Part 3 of 4 — mandatory classification and independent review.
>
> Before treating this part as read: find the line below matching
> `<!-- SKILL-LOAD-PROOF part=part-3 token=... -->` — its position moves on every
> regeneration — and run `planning/scripts/verify-skill-load.sh --part part-3
> --token <the-token-you-found>` before continuing. Naming a token is not
> enough; the command must succeed. If it refuses, you have not finished
> reading this part.

<!-- REVIEWER_SECTION:START mandatory-review -->
## 3. Mandatory classification and independent review

`create-plan.sh` creates the mandatory UI classification and adversarial-review
sections. Keep both concise and update structured values through the CLI.

When `UI affected: yes`, the UI validation reference applies and its `Required`
value must be `yes`. Never use `UI affected: no` to avoid browser validation
for an HTML, CSS, template, component, route, or user-facing behavior change.

Before a plan is ready to execute, a **fresh secondary agent** must create
`adversarial-review.md`. Give that agent the request, repository context, and
plan artifacts, but not the planning agent's conclusions. It must identify
every unplanned file, symbol, behavior, test, browser interaction, dependency,
and bug-recovery path needed to execute the request. The plan is rejected
until every finding is resolved and the review verdict is `✅ approved`.

**Environment facts go to a reviewer from the plan's own working context, not
from prose in the brief.** A reviewer brief that hard-codes a schema name,
socket, or active theme and gets it wrong carries a false fact into every
parallel review session. Prefer putting such facts in `working-context.md`
(verified) and telling the reviewer to read it from there; when a fact cannot
be verified, mark it as an assumption rather than asserting it.

The fresh adversary must be **bounded-read locked**. Hand it the exact reader
command, plan directory, and supported entry ids/views. Its starting prompt
must include verbatim:

"Read plan files and artifacts ONLY through the gated reader:
  "<PLANNING_SKILL_DIR>/scripts/plan-context.sh" read --plan-dir <PLAN_DIR> --document ID
  "<PLANNING_SKILL_DIR>/scripts/plan-context.sh" read --plan-dir <PLAN_DIR> --unit WNN
Valid --document IDs: plan, inventory, coverage, progress,
goal-progress:<goal>, adversarial-review, stories, bugs, fixes, fix-keys,
approval,
goal:<goal id>, step:<goal>/<step>. Each read returns one PAGE, not the
document. The documents that are not narrative — inventory, coverage,
adversarial-review, stories, bugs, fixes, fix-keys, approval — default to the
whole-document `full` view; every other id defaults to `summary`, and
`--view full` is available for
any of them. A page that withheld records reports next_token — pass it back as
--token and keep going until no next_token comes back. You have NOT read a
document until a page returns without one; treat a page you stopped early on as
an unread document and say so. **The `summary` view is an excerpt of the head of
the file, not a condensation of it, and it truncates before paging — so it
returns no next_token however much it left out, and prints an `excerpt=` line
saying so. No next_token from `summary` does NOT mean you have read the
document. Reviewing a plan, judging a finding, or approving anything requires
`--view full` paged to exhaustion.** Plan-read bytes are capped at
the per-role budget when ROLE_ID is set (the gate lowers --max-bytes to it) —
do not rely on --max-bytes above that cap; page instead. Never load a whole
plan file, an entire plan directory, or the `.plans/` tree wholesale. A
wholesale file read of a plan artifact is a context-overflow violation. If the
gate cannot give you something, report it as a limitation — do not bypass it."

The fresh adversary assumes the **chris placeholder persona** (oriented scout):
spawn it with `ROLE_ID=chris`, have it load its scoped role docs and voice via
`"<PLANNING_SKILL_DIR>/scripts/role-context.sh" chris` (which injects its
stance preamble), and require it to state its persona id in the returned
findings. The adversary forms its own findings from the bounded-read gate and
its scoped role docs; it never receives the planning agent's conclusions. A
spawn that cannot resolve ROLE_ID=chris fails closed (the reader refuses) and
must be respawned with a valid identity.

**Scope note: the persona, capsule, and Reviewer A/B machinery describe the
review harness.** When the role-context/capsule tooling (`role-context.sh`, a
capsule workspace) is present in the environment, use it as described. When it
is not — an ordinary plan in a generic environment — the requirement reduces
to: use a **fresh secondary agent with a new session and no prior conclusions**
(bounded-read locked and skill-locked as above) to produce the adversarial
review; the persona, capsule manifest, and two-reviewer A/B split are
harness-specific and OPTIONAL.

Do not hand the adversary a command that dumps a plan file or directory in
full. Require the adversary's returned findings to state that all plan reads
went through the gate and to list any wholesale read it performed, so a
violation is visible (soft audit).

Do not allow the planning agent to approve its own review. **Re-run a fresh
reviewer whenever a revision changes scope, ownership, dependencies, or
acceptance criteria (a material change), or when a bug is discovered** — see
"Execution order is mandatory" below for the exact cadence.

The reviewer protocol is version `1.4.2`. Fresh-review mode remains the
default. Iterative mode is opt-in and must be bounded by a maximum of three
verification passes per reviewer and three fresh-review cycles per benchmark.
Reviewer records use `review_cycle`, `reviewer_session`, `finding_owner`,
`verification_pass`, `closed_findings`, `reviewer_handoff`, and
`review_mode`. Reviewer A MAY close only findings it owns and MUST NOT issue
overall plan approval. Reviewer B must perform the final independent review
and write one reviewer-owned `approval.json` containing
`reviewer_session_id`, `mode`, `approved_findings`, `rejected_findings`,
`approved_at`, and boolean `overall_plan_approval`. `false` is valid terminal
review evidence for detection grading, but it is never an adoption pass. Each
finding uses a stable `AR-NN` ID and records precise file/section evidence,
impact, observed contradiction, and required correction. A finding may
consolidate multiple defects; one finding per defect is not required. A fresh
reviewer must use a new session and capsule, receive no prior conclusions,
and perform the final independent approval.
Exceeding a pass or cycle limit, inheriting prior conclusions, or changing the
task contract or safety boundary marks the run unresolved and requires a fresh
review.

**A finding is a hypothesis, not a work order.** Verify a finding's factual
claims against the codebase before acting. Findings are evidence-backed
hypotheses: some are wrong, some are narrower than stated, and some name the
right defect at the wrong location. Acting on a finding without verifying it
produces a second defect on top of the first.

**A review that finds nothing blocking is a valid and valuable result.** Say so
plainly. Prior cycles finding real defects do not obligate this one to.

**Resolving a finding.** A finding names a symptom, not the full extent of the
defect. A work unit's behaviour is defined across **seven surfaces**, and a
finding cites exactly one. Before recording a resolution, sweep all seven:

1. **Instructions** — step `.md` §5.x; the implementer builds what is written
   here, so a fix that lands only here has not reached the other six.
2. **Acceptance criteria** — step `.md` §6.x; an unfixed criterion is worse
   than an unfixed pair, because the gate now certifies the defect. Move the
   criterion with the instruction, in the same edit.
3. **Inventory description** — the `work-unit-inventory.md` row; the scheduler
   reads the old intent here, and no prose edit reaches a table row. Check the
   row's description, target, type, and dependencies explicitly.
4. **Change target / file / scope** — the same row plus the step header; if it
   lags, work lands on the wrong file.
5. **Goal owned-unit roster** — `goal.md` §9.1; a unit omitted here is
   effectively unowned.
6. **Dependency edges** — the row's Depends-on column; a verification runs
   before what it verifies when the edge lags.
7. **Testing companion** — `<step>-testing.md`; the executor runs the old
   procedure when this lags. It is a real surface with its own writer:
   `update-plan-content.sh -ss <plan> <goal>/<step>-testing automated-tests
   -p 2.1: …` (and `create-step-testing.sh --overwrite` to replace it). Read
   the companion first, not last — on plans with verification-heavy goals it is
   where execution actually happens.

Two further artifacts became readable in later cycles and are worth sweeping
when a finding mentions them: the **Definition-of-done coverage table** and
**`ui-user-stories.md`** (call them 8 and 9 for an exhaustive checklist) — a
finding is not closed until every surface that mentions the behaviour says the
same thing.

Mechanically: search the whole plan for the **old** wording, not the new
(`plan-content.sh find <plan> "<old phrase>" --in all`), fix every site it
appears in including sibling units and goal documents, check the inventory row
(3), move the acceptance criteria with the instruction (2), confirm the unit is
named in its goal roster (5), and re-run the search to confirm the only
remaining hits are deliberate references to the corrected history.

A resolution recorded without the sweep is a claim, not a fix. The
verification-one-unit-away variant is the hardest: a unit may be correct across
all seven surfaces while the verification unit that grades it still checks the
old behaviour. Whenever a unit's change target, scope, or behaviour changes,
re-read the verification unit that grades it (`--propagation` surfaces which
verification units name it).

**Prose ordering is not a plan addition.** Goals and steps append only
(`NN-kebab-case` is enforced; there is no renumbering helper). A prose note that
"execution order differs from step numbering" is sanctioned documentation when
the dependency edges are also recorded; it is not a substitute for those edges.
Reviewers reject ordering prose used as a substitute for recorded dependencies.
**Numbering gaps are not a defect**: steps reading 02 and 04 with no 01 or 03
need no repair when the Depends-on edges state the real order — a rename is a
five-surface edit (file, inventory row, roster, companion, tracker) and risks
more than the gap it fixes.

The review boundary is filesystem-enforced: each worker and reviewer receives
only its capsule and workspace, and each fresh reviewer receives a newly built
capsule. The capsule manifest, lifecycle records, and audit events are part of
the retained evidence. Missing identity, provenance, lifecycle, or
independence evidence is a publication failure rather than an inferred pass.

When the secondary review approves the plan, synchronize both status fields in
one atomic command (only after the independent reviewer has actually approved
it):

```bash
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" --review-status \
  <plan-directory> approved
```

Use `--review-status <plan-directory> pending` when reopening the review. The
approved form refuses to proceed while the review still contains an open or
in-progress `AR-` finding. The validator rejects a missing, pending, or
mismatched plan-description status.

Execution order is mandatory: write the complete draft plan first, invoke the
fresh reviewer, wait for its artifact, resolve every finding by revising the
plan, run the coordinator's own self-coherence pass (below), then invoke a
fresh reviewer again when revisions were material. Only after an approved
artifact exists may the planning agent run the readiness validator and create
progress trackers.

**The coordinator's self-coherence pass.** Between resolving every finding and
re-dispatching a reviewer, run `validate-plan.sh` over the plan and read its
output before deciding the plan is ready for another cycle -- not only its exit
code. Six reviewer cycles on a real plan (BUGS.json B110) were each gated by a
purely intra-document inconsistency a mechanical sweep could have caught
before the reviewer was ever dispatched, because this pass was advisory prose
rather than a required step: each cycle correctly reported the inconsistency
it found, so the loop looked like it was working, and nobody measured that the
same defect class was recurring in a new surface each cycle. `validate-plan.sh`
runs two exact checks for exactly that shape, as FAILs rather than the --stale
sweep's advisory WARNs: a paragraph that quotes a claim as retracted (`an
earlier version ... said "X"`) while another, non-retraction paragraph of the
same document still carries that exact claim; and a handoff or similar
paragraph naming a count ("the following four steps") with no explicit member
(a WNN/BNN/TNN id, or a file path) named alongside it. Skipping this pass and
dispatching a reviewer straight from a finding-resolution edit is what let six
cycles in a row spend a reviewer's judgement on what a script would have
caught for free.

After review, revise only the named document target with the flagged update
commands. Use `-dp`/`--description-paragraph`, `-gp`/`--goal-paragraph`,
`-sp`/`--step-paragraph`, or `-rp`/`--review-paragraph` for one paragraph; use
the corresponding `-ds`, `-gs`, `-ss`, or `-rs` flag for a section with one or
more `-p N.N: content` paragraphs. For example:

```bash
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" \
  --description-paragraph <plan-directory> 6.1 "<revised affected area>"
"$PLANNING_SKILL_DIR/scripts/update-plan-content.sh" \
  --step-section <plan-directory> 01-build/02-step-verify acceptance-criteria \
  -p 6.1: "<updated pass/fail criterion>"
```

Use `--table-paragraph <plan> <document-id> <N.N> <columns> "<CSV>"` to
replace one paragraph with a validated Markdown table. Quote the CSV fields;
use doubled quotes (`""`) for CSV-standard literal quotes, or `\"` when a
shell-friendly escaped quote is clearer. Use
`--insert-after` or `--insert-before` with a document ID and paragraph label
to add one paragraph; later labels in that same section shift automatically.
The `-p N.N:` forms auto-create only when the section's labels are contiguous
`1..max` with no trailing unlabeled content — a section with gaps or
unlabeled paragraphs must be re-authored (e.g. re-run
`create-step-testing.sh --overwrite` so every paragraph gets its `§ N.x`
label) instead of silently appending.

A section number is fixed per section name, not by position: automated tests
are `§ 2.x`, browser `§ 3.x`, backend `§ 4.x`, manual `§ 5.x`, whichever of
them a companion carries. A section form can only rewrite a section the
companion already has, so supply the section when you create it — `-ss` cannot
add one, and re-creating with `--overwrite` only helps if you pass the
section flag as well.

Run the validator again after revisions and reopen the adversarial review when
the change affects scope, ownership, dependencies, or acceptance criteria.

Reviewer duties for reviewer-gated fix keys: when writing or updating the
`## Findings` table, keep the mandatory-with-blank-allowed `Work unit` column
(see 3.1 below) and let `update-adversarial-review.sh` re-mint the derived fix
keys. When recording which key a fix used, write one claim line per
(finding, work unit) into `fixes.md` (`finding_id`, `work_unit`, `key`,
tab-separated). The approval gate auto-verifies `fixes.md` claims against
`fix-keys.json` before flipping the review status to `approved`.

**Reviewers must be allowed to write `adversarial-review-incoming.md`** (the one
plan file a reviewer may write, so findings survive the coordinator's context).
Instructing a reviewer subagent to be *strictly* read-only breaks the fix-key
gate rather than tightening it: the reviewer cannot publish its findings, so the
coordinator mints the keys from the reviewer's returned prose, and minting and
claiming then happen in one session -- self-certification by construction, which
`verify-fix-keys.sh` refuses. Read-only everywhere else, this one file
excepted.
`update-adversarial-review.sh` consumes it as its findings source and removes
it after the table is rewritten. A reviewer writes its Findings CSV rows there;
the coordinator runs `update-adversarial-review.sh <plan>` (no `--file`) to
land them.

**A plan records what it assumed.** `## Assumptions` (§ 11.1) holds what was
assumed rather than confirmed, and what would change if the assumption is wrong.
It is not a place for open questions — those are § 8 — but for the choices made
silently, where nobody was asked and the plan proceeded anyway.

The value is at review time. An adversarial reviewer can attack a stated
assumption; an unstated one is invisible until it turns out to be false, and then
it reads as a defect in the work rather than a decision nobody recorded. One
comparable run made five interpretations that were only written down afterwards,
in a postmortem, once they had already shaped the plan.

**A reviewer report records what the cycle cost.** The review-scope block carries
the reviewer's session id, the wall time, and the number of findings this cycle
produced. The session id is what lets a claim be traced to the run that made it;
the other two are the only signal anyone has that a review is converging.

A falling findings count across cycles means the plan is improving. A flat one
means the cycles are not finding less, and the plan may not be the thing at
fault — one comparable run reached seventeen cycles and 41.7 million tokens
before anyone asked that question, because no cycle recorded what it had cost.
Nothing enforces a ceiling; the point is that the number is visible when someone
decides whether to run another.

**A reviewer runs `--check` on its own rows before handing them over.** The shape
gate and the mint preview already run on the write path, so a malformed row can
never land — but it fails at *consumption*, which is after the reviewer has
finished and left. The coordinator is then holding a refusal about rows it did
not write, and the one session that could explain the intent is gone. One command
before returning moves the refusal to where it can be answered:

```bash
"$PLANNING_SKILL_DIR/scripts/update-adversarial-review.sh" <plan-directory> --check
```

The commonest cause is a row naming more than one work unit in the Work unit
cell, which the fix-key gate cannot mint. One primary unit per finding; if a
finding genuinely spans two, it is two findings that cross-reference each other.

**Reviewers mint fix keys; fixers claim them. Never the same session.** A
reviewer mints the keys by publishing its findings; the fixer claims the keys
in `fixes.md`. Minting and claiming in the same session is self-certification:
`verify-fix-keys.sh --claimed-by <session>` FAILS when the claiming session is
the session recorded as `minted_by`, and the approval gate always passes a
claiming session, so a self-certified fix set cannot be approved. If a fixer
must mint its own keys to record a finding the reviewer
missed, surface it as an open finding for a fresh review rather than resolving
it on its own authority.

<!-- REVIEWER_SECTION:END mandatory-review -->

### 3.1 Reviewer-gated fix keys

Gated findings bind a reviewer finding to an owning work unit. The
`## Findings` table in `adversarial-review.md` has a
**mandatory-with-blank-allowed** `Work unit` column (the last of five): every
row carries a final `WNN` cell, or an empty/`N/A` cell when the finding needs
no fix key. `update-adversarial-review.sh` mints a
per-(finding, work-unit) SHA-256 fix key (secret concatenated before the message) for every gated row and stores only
the derived keys in `fix-keys.json` beside the review file; the secret itself
lives in the private scratch dir `$(planning_tmpdir)/review-fix-keys/<session-id>/`
(`chmod 700` dir, `chmod 600` secret) and never enters the plan. Finding IDs
must match `^AR-[0-9]+$` and work-unit IDs `^W[0-9]+$`: minting warns per
non-conforming gated row and fails the run if any gated row could not be
minted, so a typo cannot silently disable the whole gate. `fix-keys.json`
records `minted_by` (the session that minted; override with `MINTED_BY`), and
`verify-fix-keys.sh --claimed-by <session>` fails when the claiming session is
the minting session (self-certification).

The fixer records which key each fix used with `add-fix-claim.sh <plan>
--finding AR-NN --work-unit WNN --key <hex>`, one call per gated pair. It writes
the tab-separated claim line (`finding_id \t work_unit \t key`) into `fixes.md`,
refuses a pair the review does not gate and a key that is not in
`fix-keys.json`, and never derives a key — that needs the minting session's
secret, and a fixer that could read it could mint its own. The approval gate
runs `verify-fix-keys.sh --claimed-by "${CLAIMED_BY:-<the minting session>}"`
before `--review-status approved` flips the verdict: every gated pair must be
claimed with a matching key by a session that is not the minting one, so export
`CLAIMED_BY=<fixer session>` at approval — the default is the minting session
and the gate refuses it as self-certification. Then the session secret
dir is removed (invalidation) so a stale `fix-keys.json` fails closed on
re-approval. Plans without `fix-keys.json` (ungated) and plans whose findings
all carry no work unit approve without verification.

The same "session secret missing" refusal also fires when the secret is lost
to an ordinary temp-directory eviction between mint and claim, not only at a
deliberate approval-time invalidation: the store is designed to be fresh per
boot (`planning_tmpdir.sh`) while the mint-claim-verify protocol spans
sessions, so mint, claim and approve must complete in one sitting or risk it.
The refusal names the recovery — re-run `mint-fix-keys.sh` — which starts a
new session and rewrites `fix-keys.json`, at the cost of every prior claim and
the audit trail of which session claimed which key.

#### Key reuse and rotation

Fix keys are scoped to one review session. Reuse the same derived key within a
session for repeated claims on the same (finding, work-unit) pair: minting is
idempotent while the session dir exists, so re-running `update-adversarial-review.sh`
re-derives identical keys. A new secret is minted only when the session dir is
gone — which happens at approval (invalidation by the approval gate); keys from
an invalidated session fail verification (stale keys never pass), and
re-approval of such a plan refuses. Rotating the secret within a live session
is never done.

### 3.2 Add verification instructions

For every step with verifiable behavior in a goal whose testing requirement is
`yes`, create a companion file with the same number and slug:

```text
<goalname>/steps/01-step-<short-slug>-testing.md
```

Omit the file when the goal's testing requirement is `no` or when there is
genuinely nothing to verify, such as a pure documentation step. If a step is
updated through the CLI and its companion already exists, review that
companion for accuracy and completeness before continuing.
The validator enforces companions for every non-documentation step in a goal
marked `yes`.

Include only the relevant sections:

- **Browser verification:** exact navigation and actions, the expected result,
  and explicit pass/fail criteria. Use the browser tools or browser-testing
  skill available in the environment.
- **Backend verification:** concrete commands and inputs to exercise the
  behavior against the running system, plus expected output.
- **Automated tests:** required unit or integration tests, their locations,
  commands, and relevant project testing conventions.
- **Artifact comparisons (optional):** when a proof compares a produced file
  against a reference, declare it as a table rather than in prose, so the gate
  can check that the comparison is one the target can actually produce:

  ```
  update-plan-content.sh -tp <plan> step:<goal>/<step>-testing <N.N> \
      'Artifact,Comparison' 'pub/media/invoice.pdf,text-layer'
  ```

  `planning/artifact-comparisons.json` lists the comparisons and the artifacts
  that cannot be reproduced byte for byte. Asking for `exact` on a PDF, an image
  or any zip-backed document is refused: those embed a creation timestamp or an
  encoder version, so the criterion would fail correct work. Say what tolerance
  the proof allows instead.

A step may require multiple verification methods. Do not mark it complete until
all listed checks have actually passed.

**The application still serves (P0).** A goal that changes module state,
schema, or configuration (`etc/module.xml`, `etc/config.php`, `db_schema.xml`
or equivalent — see `state-change-registry.json`) must carry an acceptance
condition that exercises the running application, not just the changed
artifact: for a Magento plan, a plain request returning HTTP 200. The
validator WARNs when such a goal's verification units contain no request or
health-check phrase. This is the check that catches "all artifacts verified,
site is down".

**Command registry (P1).** Every command literal in step instructions or
testing companions must be registered in the plan's `commands.json` with its
"when" context, so the purpose travels with the command instead of being lost
when steps are copied:

```bash
"$PLANNING_SKILL_DIR/scripts/register-command.sh" <plan-directory> cache-flush \
  'bin/magento cache:flush' 'routine; after a constructor change'
```

`create-plan.sh` seeds an empty registry; `register-command.sh` adds,
removes, and lists entries; `validate-plan.sh` WARNs on any unregistered
command literal (and FAILs under `--complete`). When the validator flags a
literal, register it with its correct "when" — never delete the literal to
silence the check.

Detection is language-agnostic: a candidate is a path that resolves to an
executable non-directory, a path whose last segment sits under a bin-like
directory (`bin/`, `sbin/`, `.bin/`, `Scripts/` — covering `vendor/bin/`,
`node_modules/.bin/`, `.venv/bin/`), or a first token in the small universal
core (`git make docker sh bash zsh env sudo npx`). Those are the only entry
points — arguments strengthen a candidate but never qualify a span on their
own. Data/markup extensions (the rjq-matched list in
`never-executable-extensions.json` — e.g. `.xml`, `.sql`, `.php`, `.md`),
`:line`/`#Lnn` citation suffixes, and route/prose shapes (a leading `/`
without a bin-like segment) never flag. Each registered command's first token
teaches the detector that tool word, so registering `pytest -q` makes
`pytest` a word — no per-language list to maintain.

### 3.3 Validate, then create progress trackers

Run the validator before creating trackers or presenting the plan as ready:

```bash
PLANNING_SKILL_DIR="<installed-planning-skill-directory>"
"$PLANNING_SKILL_DIR/scripts/validate-plan.sh" <plan-directory>
"$PLANNING_SKILL_DIR/scripts/validate-plan.sh" --propagation <plan-directory>   # surface-consistency checks (on by default); --no-propagation disables it
"$PLANNING_SKILL_DIR/scripts/validate-plan.sh" --stale <file-of-phrases> <plan-directory>   # advisory: WARN on a listed phrase in an unmarked paragraph
"$PLANNING_SKILL_DIR/scripts/validate-plan.sh" --stale default <plan-directory>              # advisory: bundled wording list; sweeps companions too
```

`--stale` is a **wording review aid, not a gate.** Every finding is a WARN and it
never changes the exit status, so a plan is not blocked by it and its output does
not need clearing before the plan is ready. Read the warnings and judge each one:
measured on real plans the count phrases were right 0 times in 24 hits, because a
count that has drifted reads exactly like one that cannot. What *is* gated is the
part that can be decided: an acceptance criterion declaring a comparison in the
step's `## Artifact comparisons` table is checked against
`planning/artifact-comparisons.json`, so asking for `exact` on a PDF or an image
fails.

<!-- SKILL-LOAD-PROOF part=part-3 token=eaf2a31ae377241d -->


Beyond structure, propagation, the advisory wording sweep, and the placeholder
registry, the validator checks two more things. The **serve check** WARNs when a goal
that changes module state, schema, or configuration (per
`state-change-registry.json`) has no verification acceptance condition
mentioning a request or health check. The **command registry** WARNs on any
command literal in a step or testing companion that is not registered in the
plan's `commands.json` with its "when" context (and FAILs under
`--complete`); register flagged literals with `register-command.sh`.

`--propagation` encodes the surface rule (§ "Resolving a finding") and runs
by default. It flags a verification unit that grades a sibling with no
dependency path to it (honouring deliberate reverse/baseline orderings and
transitive ordering), a graph leaf in a goal that owns a verification unit, and
a goal whose §9.x roster does not match the units the inventory assigns to it,
and a step's `## Handoff` paragraph naming a later unit with no dependency path
in either direction — a handoff is a licence for that unit to run early, and a
promise the graph does not order is exactly what lets it run too early. A
paragraph that also records a history marker (the same vocabulary `--stale`
checks) is exempt, since a corrective paragraph legitimately restates an old,
disproven claim rather than making a new ordering promise.
It also WARNs (never blocks) when a unit's instructions mention a project
symbol (one whose namespace root or path prefix the plan edits) that no
inventory row owns — this rule cannot distinguish "edit this" from "this is
where we attach" from text alone, so it is a skimmable signal, not a gate. It
does not flag mere vendor/core seams (`Magento\...`, `Amasty\...`,
`Vendor_Module::path` templates), `X::class` constants, or cross-plan
references. `--no-propagation` disables it. `--stale` turns the "sweep for the
old wording" discipline into a gate: a phrase listed in the file fails unless
every paragraph containing it also records a history marker such as
"previously" or "an earlier version". `--stale default` runs a bundled
case-count phrase list (`all four`, `the six states`, etc.) — case-count
wording is the anti-pattern because it drifts when a case is added, and "every
case enumerated in the instructions" is the drift-proof form. The stale sweep
covers the same documents as `find --in all`, including the `*-testing.md`
companions. Add a closed finding's old wording to the phrase file so each fix
becomes a permanent regression guard rather than a one-time correction.

**Coverage rows.** The Definition-of-done coverage table's header is
`Required outcome or proof | Work unit IDs` — it deliberately maps both
outcomes *and* their proofs to units, so crediting a `test` or `verification`
unit is the sanctioned convention, not drift. A coverage row should name the
unit that produces the outcome **as well as** the one that proves it; this is
enforced by review, not by the validator (there is no mechanical form that
avoids false warnings). A testing companion may reference a same-goal
`test`/`verification` unit ("automated tests: covered by WNN") — that is
proof-coverage prose, not a dependency claim.

Do not waive validation failures. Correct the inventory, goal boundary, or
step files and run it again. The validator checks the structural guarantees;
the decomposition review remains required for semantic completeness.

For a goal marked `Test required: yes`, every implementation, markup, style,
configuration, data, or generated work unit must have a downstream `test` or
`verification` unit in the dependency graph. A goal marked `no` may omit that
proof when its rationale records why testing is not meaningful or possible.
Do not use `no` to avoid testing observable behavior.

Create plan and goal progress trackers with the bundled creation helpers;
they enforce the table shape and initialize every item as `💤 incomplete`.

Use these statuses consistently:

- `💤 incomplete` — not started
- `⏳ in progress` — currently being worked on
- `✅ completed` — implementation and all applicable verification passed

Progress percentages count completed items equally. A goal is complete only
when all its steps and applicable verification are complete. The initiative is
complete only when all goals are complete.
