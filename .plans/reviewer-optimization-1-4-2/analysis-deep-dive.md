# Analysis deep-dive: why the current 1.4.2 seeded runs scored 0/3

Status: research record for `reviewer-optimization-1-4-2` (asker: iterative/fresh seeded pair handling).
Basis: `analysis.md` in this directory, plus a full manual replay of `grade-blinded-run.sh` predicates against both archived Reviewer B approvals.
Date: 2026-08-11.

---

## 1. TL;DR

> **Plain English — one paragraph.** The reviewer did not fail to *find* the seeded defects:
> the semantics grader failed to *recognize* them. Running the grader's own matching
> logic by hand against the archived Reviewer B findings shows that in the fresh run,
> **all three** seeded contradictions had a finding whose "did the reviewer name the
> right problem and the right fix" checks **pass**; only a first-stage location filter
> (which requires the finding to point *exactly* at the file `plan-description.md`
> and at the exact location string `plan-description.md § 3.1`) threw them all out
> before the semantic checks ever ran. Reviewer B naturally wrote findings that list
> multiple affected files and use prose locations ("Initial button count
> requirements"), because the reviewer prompt tells it the finding may consolidate
> multiple defects but never tells it the hidden §-notation and single-file rules the
> grader enforces. The same location rule is the sole blocker for the entire
> 0/3 in both runs; one additional signal-vocabulary rule (hyphenated vs. spaced
> "fourth generated button") blocks one defect in the iterative run. A separate,
> unrelated bug mislabels missing threshold configuration as `MISSING_DENOMINATOR`.

The headline result is **Root cause RC-01** below. RC-02/RC-03 are second-order
contributors, RC-04 explains the one genuinely-subtle miss, RC-05 is an unrelated
state-labeling bug, and the rest of the candidate causes I considered were **disproven**.

---

## 1.5 Transcript verification from the Codex SQLite store (checked after first draft)

> **Plain English — summary.** I pulled the two **authoritative** Reviewer B sessions out of
> the Codex SQLite state store (`~/.codex/state_5.sqlite` + per-session rollout JSONL in
> `~/.codex/sessions/…`) and read their surfaced assistant messages and the exact
> `plan-description.md` bytes they were shown. This **confirms RC-01 as effectively the
> whole story**, **refines RC-03** (the operative failing grader predicate is location/path
> plus paraphrase — hyphenation is only latent), and **upgrades RC-04 from "hypothesis"
> to "confirmed observation miss"** for the fresh run.

**Threads located** (authoritative = the harness Reviewer B that wrote the graded
approval; worker-internal reviewers reviewed the *fixed* worker plan and are irrelevant):

| Role | Codex thread | Notes |
|---|---|---|
| Iterative worker | `019ff287-…` | cwd `/tmp/20260811T203343Z-clean-current-seeded-iterative/current/workspace` |
| Iterative Reviewer A | `019ff296-…` (title "Review the plan at …capsule…-A-…") | |
| **Iterative Reviewer B (authoritative)** | `019ff298-e8fe-7de3-936a-11bf326ebe99` | rollout `rollout-2026-08-11T22-52-15-019ff298…jsonl` |
| Fresh worker | `019ff29e-…` | |
| Fresh worker-internal reviewers | `019ff2a5‑…`/`019ff2a8‑…` (Aquinas/Descartes) | reviewed fixed plan — not decisive |
| **Fresh Reviewer B (authoritative)** | `019ff2ab-b992-7ef3-9359-779e774d196f` | rollout `rollout-2026-08-11T23-12-48-019ff2ab…jsonl` |

Chain-of-thought is stored **encrypted** (`reasoning → encrypted_content`; `summary: []`),
so I evaluated the **surfaced** assistant messages + the file contents the tool calls read
(`sed -n '1,220p' …/plan/plan-description.md`, tool outputs themselves are not persisted
in the rollout; the archived capsule file is byte-identical authority for what was shown).

**What Iterative Reviewer B said (surfaced):**
> "…its desired outcome says two initial buttons, **third generated completion**, and a
> black border, while the rest of the plan consistently specifies one initial button,
> **fourth generated completion**, and a white border."

It consolidated **all three** seeded defects into AR-01 (path `plan-description.md`,
location `line 14, contrasted with lines 24 and 45…`) and a second AR-02 about the
unresolved adversarial-review gate. The replay shows AR-01 passes **signal + correction
for SD-01 and SD-03** and only fails on LOCATION; SD-02 additionally fails SIGNAL (it
writes "the initial button", not the literal "one initial button"). **So the sole blocker
for a true-positive grading of a finding that names all three defects is the
LOCATION/PATH + paraphrase predicates.**

**What Fresh Reviewer B said (surfaced):**
> "There are two stable defects now. The plan simultaneously claims 'exactly two initial
> buttons' … and it alternates between black and white border requirements…"
> AR-02: "contradiction between exactly two initial buttons vs one initial button."
> AR-03: "contradiction between black final border vs white final border."

Fresh Reviewer B surfaced **no** finding about the third/fourth generated-button wording.
Its only third/fourth-adjacent finding (AR-01) is scoped to `ui-story-runs/US-01.md`
(click sequence), not `plan-description.md § 3.1`.

**Why AG-01 is a genuine fresh-mode miss (not pure formatting).** The two driver plans
were worded differently, so the seed's first-occurrence replacement produced two
differently-visible versions of the same defect:
- Iterative plan-description (line 14): "*pressing the **third** generated button must
  clear the document…*" — blatant (says the third, not fourth, must clear). Caught.
- Fresh plan-description (line 11): "*after the **third** generated button exists,
  pressing **that fourth** generated button clears the document…*" — subtle; the
  predicate still reads "press fourth", so a fast reader resolves it as a referential
  quirk. Not caught.
So the fresh miss is a blend of **mutation-subtlety** (seeded first occurrence under that
driver's wording) and reviewer attention, and it is independent of the grader.

**Theories added / changed vs. the first draft**

- RC-01 **strengthened to "effectively the whole grader story".**
- RC-03 **reframed:** the operative brittleness that actually blocked a true positive is
  (a) the **location/path** exact-match gate, then (b) **paraphrase/substring-only
  signal** ("the initial button" ≠ "one initial button"). **Hyphenation
  ("fourth-generated-button") is latent** — it did not decide any of these six
  defect-row outcomes here because the evidence field also contained the spaced
  "fourth generated button" (line 45 quote). RC-03's headline should be "substring-only
  signal with no token/word/number equivalence"; hyphenation is a member, not the cause.
- RC-04 **confirmed:** fresh Reviewer B observably did not flag SD-01.

This does not change the adoption conclusion: fail-closed was correct; the 0/3 is
dominated by the grader contract (RC-01), with a real-but-secondary reviewer miss for
SD-01 in fresh (RC-04) and a paraphrase-sensitivity gap (RC-03).

---

## 2. Method — how the evidence was traced, and the sources used

I followed one endpoint-to-endpoint path: seeded defect → defective plan the reviewer
actually read → Reviewer B approval artifact → adapter-formed grading evidence →
grader predicates. For each step I read the archived artifacts, not just the narrative.

### 2.1 Primary evidence read

| What | Path |
|---|---|
| Problem statement + recommended next questions | `.plans/reviewer-optimization-1-4-2/analysis.md` |
| Fresh run narrative | `benchmark/results/20260811T205844Z-clean-current-seeded-fresh/comparison.md` |
| Fresh Reviewer B approval (authoritative findings) | `benchmark/results/20260811T205844Z-clean-current-seeded-fresh/current/reviewers/20260811T205844Z-clean-current-seeded-fresh-current-B-1786482768027444165/plan/approval.json` |
| Fresh oracle result | `benchmark/results/20260811T205844Z-clean-current-seeded-fresh/current/oracle.json` |
| Fresh **defective** plan the reviewer read | same reviewer capsule → `plan/plan-description.md` (line 11 carries all three mutations) |
| Fresh reviewer prompt (blinded instruction) | same reviewer capsule → `reviewer-prompt.md` |
| Iterative run narrative | `benchmark/results/20260811T203343Z-clean-current-seeded-iterative/comparison.md` |
| Iterative Reviewer B approval | `benchmark/results/20260811T203343Z-clean-current-seeded-iterative/current/reviewers/…B…1786481534995424311/plan/approval.json` |
| Iterative defective plan | same capsule → `plan/plan-description.md` |
| Seeded mutation spec (the private map's public twin) | `benchmark/planning/pilot-blinded-defects.json` |
| Semantic grader (predicates replayed verbatim) | `benchmark/planning/grade-blinded-run.sh` |
| Direct-oracle fixture that "proved 3/3" | `benchmark/planning/tests/fixtures/pilot-consolidated-finding.json` |
| Approval schema validator + reviewer prompt template | `benchmark/planning/setup-benchmark.sh` (lines 495–570, 771–790) |
| Evidence adapter (approval → oracle terminal evidence) | `benchmark/planning/setup-benchmark.sh` (lines 879–936) |
| Threshold state synthesis | `benchmark/planning/setup-benchmark.sh` (lines 179–180, 1031–1136) |
| Batch runner (threshold wiring) | `benchmark/planning/run-benchmark.sh` (line 208) |
| Oracle wiring | `benchmark/planning/review-oracle.sh` (blinded → `grade-blinded-run.sh`, line 10) |

### 2.2 How the pipeline is wired (verified by reading the code)

1. `seed-blinded-defects.sh` copies the source workspace and performs the three
   string replacements in `plan-description.md`; it also snapshots the **defective**
   workspace as `target-snapshot` and encrypts the defect map. `pilot-blinded-defects.json`
   is the public twin of that map: same ids, paths, locations, signals, corrections.
2. The reviewer capsule's `plan/` directory contains the **defective** workspace, so
   Reviewer B reads the mutated `plan-description.md`.
3. Reviewer B writes `approval.json`. The adapter (`setup-benchmark.sh` 895–936)
   copies each `approved_findings` dict **verbatim** into `oracle-terminal-evidence.json`
   (`findings`), adding only `finding_id`/`independent` defaults. No field is dropped
   or reshaped.
4. `review-oracle.sh blinded` execs `grade-blinded-run.sh` with that evidence and the
   target snapshot. The grader verifies target-file hashes against the encrypted map
   (so the mutations were present and unchanged), then classifies each defect against
   the findings.
5. Separately, `reviewer-state.json`/`protocol-metadata.json`/`telemetry.json` are
   synthesized for the adoption contract (lines 1031–1136).

### 2.3 The grader's matching chain (verbatim logic at `grade-blinded-run.sh`)

```python
# line 142-146
def candidate_path(finding):                       # exact string equality downstream
    path = finding.get("path", "")
    if not path and " § " in str(finding.get("location", "")):
        path = str(finding["location"]).split(" § ", 1)[0]
    return norm(path).lstrip("./")

# line 148-151
def location_matches(finding, defect):             # "§ 3.1" convention required
    candidate = norm(finding.get("location", ""))
    expected  = norm(defect["location"])           # "plan-description.md § 3.1"
    return candidate == expected or candidate in expected or expected in candidate

# line 153-158  (substring-only; no token overlap fallback here)
def signal_matches(finding, defect):
    observed = norm(" ".join(finding.get(k,"") for k in ("summary","observed_contradiction","impact","evidence")))
    return norm(defect["expected_signal"]) in observed

# line 160-166  (has a 50% token-overlap fallback)
def correction_matches(finding, defect):
    ...
    if expected in correction: return True
    return len(expected_words & tokens(correction)) * 2 >= len(expected_words)

# line 171-192  THE GATE THAT FIRED FIRST
# for each defect:
#   a finding is only considered AT ALL if:
#       candidate_path(finding) == norm(defect["path"]) AND location_matches(finding, defect)
#   otherwise it is skipped -> the defect gets classified "false_positive"
#   and later "false_negative".
```

> **Plain English — the gate.** Before the grader checks whether a finding is about the
> right problem or proposes the right fix, it first demands that the finding's `path`
> field be *exactly* `plan-description.md` and that its `location` field literally
> match `plan-description.md § 3.1`. Any finding that names more than one file, or
> describes its location in words, never reaches the semantic checks.

### 2.4 The replay result (my script, same predicates, real artifacts)

Script: `/tmp/opencode/replay_grader.py` (predicates copied from `grade-blinded-run.sh`).

FRESH —

| finding | envelope OK | SD-01 (third→fourth) | SD-02 (one→two buttons) | SD-03 (white→black border) |
|---|---|---|---|---|
| AR-01 | yes | **blocked by PATH,LOCATION** (signal+correction pass) | blocked by PATH,LOCATION,SIGNAL | blocked by PATH,LOCATION,SIGNAL |
| AR-02 | yes | blocked by PATH,LOCATION | **blocked by PATH,LOCATION** (signal+correction pass) | blocked by PATH,LOCATION |
| AR-03 | yes | blocked by PATH,LOCATION | blocked by PATH,LOCATION | **blocked by PATH,LOCATION** (signal+correction pass) |

ITERATIVE —

| finding | SD-01 | SD-02 | SD-03 |
|---|---|---|---|
| AR-01 (path `plan-description.md`, single file) | **blocked by LOCATION** (signal+correction pass) | **blocked by LOCATION,SIGNAL** | **blocked by LOCATION** (signal+correction pass) |
| AR-02 (path `adversarial-review.md`) | blocked by PATH,LOCATION | blocked by PATH,LOCATION | blocked by PATH,LOCATION |

Oracle counts reconcile exactly: every defect falls to `false_positive` (no finding
got past the gate) → 3 `false_negatives`; the 3 findings that existed but were never
matched contribute `false_positives + max(0, 3 − 0) = 6`, matching `oracle.json`
exactly (`false_positives: 6`, `false_negatives: 3`).

> **Plain English — the replay.** In the fresh run, each seeded defect has a
> find-the-right-problem + propose-the-right-fix verdict that *passes*. The only thing
> stopping every one of them from being a true positive is the file/location rule.
> So the 0/3 is a grader-recognition result, not a reviewer-capability result.

---

## 3. Root causes that withstand scrutiny

Each section: plain-English summary → evidence → mechanics → attempted disproofs →
verdict.

---

### RC-01 — The grader's path/location pre-filter rejects natural consolidated findings (PRIMARY)

> **Plain English — summary.** A reviewer checking "the plan contradicts itself between
> the description file and five other files" naturally names all the files and gives
> the location as a subject heading. The grader instead demands one exact file name and
> one exact `§ 3.1` string that the reviewer was never told about, and it drops any
> finding that doesn't comply before reading its content. That single rule alone
> explains 0/3 in the fresh run and is the universal blocker in both runs.

**Evidence**

- Fresh `approval.json` path values: `AR-02` = `"plan-description.md; work-unit-inventory.md; 01-button-chain-html/steps/01-step-initial-markup.md; 02-proof-and-handoff/steps/01-step-dom-test.md; ui-story-runs/US-01.md"`, `AR-03` = a similarly long multi-file list. `AR-01` = `"ui-story-runs/US-01.md"`. Locations are prose: `"Initial button count requirements"`, `"Final completion border color requirements"`.
- The grader's `candidate_path()` (`grade-blinded-run.sh:142-146`) does an **exact-equality** comparison against `norm(defect["path"]).lstrip("./")` = `"plan-description.md"` and never splits on `;`. `location_matches()` (`148-151`) requires equality/containment with `"plan-description.md § 3.1"`.
- The fresh reviewer quotes the **seeded** text (proof it read the defective file): `AR-02` evidence: *"plan-description.md section 3.1 states 'exactly two initial buttons'…"* and `AR-03`: *"…requires the final lowercase finished text to have a visible black border, while the implementation, test, browser-story, inventory, and UI-story artifacts require a visible white border."* The filenames in those quotes are semantically the *same location* the grader wants, just expressed naturally.
- The direct-oracle fixture that "proved" consolidation works is written to the grader's taste, not to reviewer behaviour: `tests/fixtures/pilot-consolidated-finding.json` uses `"path":"plan.md"` and `"location":"plan.md § 3.1"` — the exact shapes the grader wants. It never exercises a multi-file `path` or a prose `location`, so it cannot catch the failure mode observed here.

**Mechanics (why the reviewer writes "wrong" findings)**

- The reviewer prompt (`setup-benchmark.sh:771-790`, identical in the archived `reviewer-prompt.md`) mandates the eight fields and says *"Consolidated findings may cover multiple defects"* — which for a cross-file contradiction encourages multi-file `path` values. It never states a canonical file/location notation, never mentions `§`, and never says the grader expects the `§ 3.1` style (which would also be absurd to require of a blinded reviewer who must not know the seed).
- The approval schema validator (`setup-benchmark.sh` 495-570) checks only that the eight fields are non-empty strings and `independent` is a boolean; it hands back `approval_schema_status=valid` for the multi-file `path` values. **Schema-valid ≠ gradeable** — the review contract and the grading contract are different contracts, and only the grader knows the second one.
- The adapter (`setup-benchmark.sh:895-936`) preserves the envelope faithfully; the file/location content is exactly what the reviewer wrote. The failure is therefore not archival or transport—it is predicate-vs-content.

**Attempted disproofs (and why each fails)**

1. "The schema requires a single-file path." — **Disproved.** `approval_schema_status=valid` for both multi-file findings; validator only checks non-empty strings.
2. "The grader handles `;`-separated paths via the location fallback." — **Disproved.** The fallback at `142-146` only fires when `path` is *empty* (`if not path`); the multi-file string is non-empty, so it is compared literally to `plan-description.md` and fails.
3. "The reviewer was told the `§` convention." — **Disproved.** `reviewer-prompt.md` contains no location-format instruction; the only constraint is "non-empty string".
4. "0/3 is correct because the reviewer's findings are genuinely different defects." — **Disproved by replay.** For SD-02/SD-03 in fresh, signal and correction predicates pass when path/location are set correctly; these *are* the seeded contradictions, named with the seeded text.
5. "Maybe the reviewer simply should have targeted `plan-description.md` only." — **Weakened but not decisive.** Argument FOR the grader: each defect *originates* in `plan-description.md`, so a finding citing only that file would pass. Argument AGAINST: the same prompt explicitly authorizes consolidated findings; a reviewer describing a cross-file contradiction has no way to know the grader scopes findings to the seed's home file; and the location field would still need the hidden `§ 3.1` string. This is a contract-design question, and the contract is currently only implicit in the grader.

**Verdict: SURVIVES — primary root cause of the semantic 0/3.**

---

### RC-02 — Reviewer-facing contract and grading contract are two different contracts

> **Plain English — summary.** The reviewer is held to a list of field names; the
> grader silently enforces a second, stricter set of rules (exact single file, `§`
> location, phrase-for-phrase wording) that was never communicated and never validated
> by the schema check. The system announces "schema-valid" and "provenance-passed" for
> evidence the grader will never accept.

**Evidence**

- Schema validator scope (confirmed above): non-empty strings + `independent` bool + duplicate-ID check. No `path` cardinality, no location format.
- Grader scope: exact `path` equality, `§`-style `location`, substring `expected_signal`, token-overlap `required_correction` (`grade-blinded-run.sh:142-166`).
- The two hardening rounds' guarantee list (`analysis.md` lines 86-95) says "Consolidated findings are supported by the semantic contract" — true only for the fixture-shaped envelope, as shown by the replay.

**Attempted disproofs**

1. "Every run validates against one contract." — The schema passes while the grader fails, so the run is validating against a *different* contract than the one that decides the score. Not disprovable — it is what the evidence shows.
2. "The reviewer-prompt is generated from the same source as the grader, so they must agree." — They are separate code paths (one `cat` heredoc at 771-790, one Python in `grade-blinded-run.sh`); nothing joins them. Confirmed by reading both.

**Verdict: SURVIVES — it is the design-level restatement of RC-01.**

---

### RC-03 — The signal matcher is brittle: hyphenation, paraphrase and ordinal/number forms fail substring matching

> **Plain English — summary.** Once a finding *does* reach grading, the grader requires
> the *exact phrase* of the defect's expected signal to appear verbatim in the finding
> text (no synonyms, no hyphen/space tolerance, no word-overlap fallback — unlike the
> "required correction" check, which has a word-overlap fallback). Reviewer vocabulary
> that is perfectly natural but phrased differently fails the check.

**Evidence**

- `signal_matches` (`grade-blinded-run.sh:153-158`) is a bare substring test over
  `summary + observed_contradiction + impact + evidence`. `norm()` (`:64-66`) only
  collapses whitespace and lowercases; it does **not** strip hyphens or expand
  "4"/"fourth".
- Iterative `AR-01` describes SD-01/SD-02 as *"fourth-generated-button
  completion"* (hyphenated) and *"the initial button"* — neither contains the
  literal `fourth generated button` nor `one initial button`. Replay shows SD-02 in
  iterative fails **SIGNAL** even after excluding PATH/LOCATION.
- Fresh `AR-01` writes `generated button 4` (ordinal-as-digit) but *also* includes the
  literal *"pressing the fourth generated button"* in `observed_contradiction`, which is
  why fresh SD-01 passes signal — the pass is accidental, not designed.
- Contrast: `correction_matches` (`160-166`) has a `tokens() ∩` ≥50% leniency, so the
  correction accepts paraphrases; the signal check gets no such leniency. The two
  checks treat natural language symmetrically-ish on purpose — except they do not.

**Attempted disproofs**

1. "This never actually mattered because path/location always block first." — For the
   fresh run that is true for scoring, but the iterative run's SD-02 block list is
   `LOCATION,SIGNAL`: fixing the location gate alone would *still* leave a false
   negative. So RC-03 is an independent, live failure mode, not a theoretical one.
2. "The reviewer should quote exact strings." — Blinded reviewers are not given the
   expected-signal vocabulary; demanding verbatim phrasing makes detection depend on a
   spelling coincidence. Weakened — the guardrail exists to stop unrelated findings
   counting, but the cost is false negatives on paraphrases.

**Verdict: SURVIVES (secondary but real; will matter after RC-01 is fixed).**
> *Refinement from §1.5 transcript check:* the predicate that *actually* flipped a
> true-positive to a miss here was mostly the **location/path gate (RC-01)** followed by
> **paraphrase** ("the initial button" vs the literal "one initial button" blocked the
> iterative SD-02 signal). **Hyphenation did not decide any graded row** in these six
> defect outcomes (the evidence field also contained a spaced "fourth generated button").
> Keep RC-03 framed as "substring-only signal with no token/word/number equivalence";
> hyphenation is one member, not the cause.

---

### RC-04 — SD-01 ("third" vs "fourth generated button") was not converted into a plan-description-scoped finding in the fresh run

> **Plain English — summary.** Of the three mutations, two (button count, border color)
> were turned into crisp findings. The third — the word-level
> "after the *third* generated button exists, pressing that *fourth* generated button
> clears…" contradiction — was not serialized as a finding about `plan-description.md`.
> The only related finding (AR-01) is scoped to the UI story file and its click-count
> wording. Either the reviewer did not perceive the in-sentence mismatch, or it saw it
> and folded it into the US-01 off-by-one story; the transcript does not let us pick
> between the two, and the grader gate would have masked a *correct* catch anyway.

**Evidence**

- Defective paragraph (fresh capsule `plan/plan-description.md` line 11): *"…loads
  with exactly two initial buttons; … after the third generated button exists, pressing
  that fourth generated button clears the document and renders exact lowercase text
  finished with a visible black border."* The SD-01 mutation (seed: `fourth`→`third`)
  leaves an **internal** chain "third exists / press that fourth" which a fast reader
  can resolve by reference to § 8.1's correct "fifth click" narrative — i.e., it is
  perceivable but subtle, unlike the count/color contradictions which are gross
  cross-file conflicts.
- Fresh findings never quote "third generated button"; AR-01 (US-01 off-by-one) cites
  `ui-story-runs/US-01.md`, not `plan-description.md`.
- Iterative mode *did* observe it: iterative AR-01's `observed_contradiction` says
  *"…completion when pressing the third generated button … while the same plan later
  requires … fourth-generated-button completion…"* — and the replay shows that finding
  passes signal+correction for SD-01 and is stuck on LOCATION only. So the capability
  to see SD-01 exists; the fresh-mode output simply did not serialize it for the right
  file.

**Attempted disproofs**

1. "The mutation was not in the file the reviewer read." — **Disproved.** The reviewer
   capsule's file contains it, and the grader's defective-sha256 check passed.
2. "The reviewer never reads `plan-description.md`." — **Disproved.** AR-02/AR-03 quote
   its § 3.1 text verbatim.
3. "This is really just RC-01 in disguise." — Partially. The *grading* outcome for
   SD-01 in fresh is determined by RC-01 (AR-01 would have been a TP if path/location
   matched). But *why* no plan-description-scoped finding exists is a separate
   observation/writing question that RC-01 cannot answer, so it is kept distinct.

**Verdict: SURVIVES as a contributing cause for SD-01 in fresh mode — now CONFIRMED as
an observation miss by the authoritative fresh Reviewer B (see §1.5).** The iterative
reviewer *did* catch SD-01 (its mutation variant was blatant: "pressing the *third*
generated button must clear"); the fresh reviewer did not (its variant was subtle:
"after the *third* … pressing *that fourth* …"). IR-observable evidence (surfaced
messages + the plan bytes it read) shows no SD-01 finding was produced or serialized.
So this is a real, separate phenomenon — partly seed-subtlety, partly reviewer
attention — not merely grader masking. Directly supports the analysis recommendation 3
(attention allocation) and the new-cohort checklist idea of reconciling *every*
top-level desired-outcome statement.

---

### RC-05 — `MISSING_DENOMINATOR` is mislabeled, wrongly conflating missing *denominator* with missing *thresholds* (state synthesis bug)

> **Plain English — summary.** The archive says "denominator missing" while the oracle
> actually reports denominator 3. The real situation: the batch runner never passes the
> two acceptance thresholds, so the state synthesizer reads an empty string,
> `float("")` fails, thresholds become `null`, and the synthesizer blames the
> denominator. It is a separate bug from the 0/3 — it does not change any score — but
> it corrupts a fail-closed reason and must be fixed, not normalized away.

**Evidence**

- `run-benchmark.sh:208` invokes `setup-benchmark.sh` without setting
  `SEMANTIC_THRESHOLD`/`INDEPENDENT_THRESHOLD`; `setup-benchmark.sh:179-180` then
  exports `"${SEMANTIC_THRESHOLD:-}"` → empty string.
- State synthesis (`1031-1136`): `float(os.environ["SEMANTIC_THRESHOLD"])` on an empty
  value raises `ValueError`, caught at `1094-1095`, setting both thresholds to `None`;
  the guard at `1096-1097` (`… or semantic_threshold is None or …` → `reasons.add("MISSING_DENOMINATOR")`)
  fires even though `denominators.seeded = 3` is present and valid.
- Confirmed in output: `reviewer-state.json` (`seeded_denominator: 3`,
  `semantic_threshold: null`, reason `MISSING_DENOMINATOR`), mirrored in
  `protocol-metadata.json` and `telemetry.json`.
- Grader independence: `grade-blinded-run.sh` never reads thresholds; rates and counts
  are computed purely from findings+defects. So thresholds are *not* a cause of 0/3.
- The `SEMANTIC_THRESHOLD_FAILED`/`INDEPENDENT_THRESHOLD_FAILED` branches exist but only
  in the test harness (`tests/test-review-lifecycle.sh:334-335` sets `1.0`), never in
  the real runner.

**Attempted disproofs**

1. "Threshold absence caused the 0/3." — **Disproved.** The oracle score is
   threshold-independent by code inspection; only fail-closed labeling changes.
2. "The reason name is accurate; the denominator genuinely is missing." — **Disproved.**
   `oracle.json` reports `denominators.seeded: 3` and a terminal report; the state
   synthesizer reads 3 but still adds `MISSING_DENOMINATOR` because a *different*
   variable (threshold) is None. The reason string is simply wrong.

**Verdict: SURVIVES — a real, isolated state-synthesis/coding defect; not the cause
of the semantic zero, but part of why the archive is not clean.**

---

## 4. Hypotheses considered and disproven outright

These were real candidate causes and were eliminated with evidence:

1. **"The adapter dropped or reshaped the finding envelope."** — **Disproven.**
   `setup-benchmark.sh:895-936` copies each finding dict verbatim; the oracle count
   `false_positives: 6` reconciles *exactly* with "3 findings present but none
   targeted" (`3 defect-rows + 1×3 unmatched findings`). Had findings been dropped, the
   count would be 3, not 6.
2. **"The reviewer was shown the fixed plan, not the defective one."** — **Disproven.**
   The reviewer capsule's `plan/plan-description.md` carries the mutated text, the
   findings quote it verbatim, and the grader verified the defective-file hash against
   the encrypted map before grading.
3. **"The seeded mutations never applied / were reverted before grading."** —
   **Disproven.** Same evidence as (2); the seed verification exit-path would have aborted
   with "target defect hash mismatch" (`grade-blinded-run.sh:59-60`) if it had.
4. **"The reviewer was not blind / saw the defect map, biasing output."** — **Not
   supported, and irrelevant.** The capsule excludes the encrypted map (created with
   `chmod 700`/`600`, `seed-blinded-defects.sh:20,78-83`); and even if it were debated,
   grading consumes only the findings, whose content proves the reviewer reasoned from
   the plan text.
5. **"Reviewer B's *rejection* is itself a false signal."** — Not a cause. The
   rejections were **legitimate**: the defective plan really was internally broken in
   multiple ways, so `overall_plan_approval=false` was correct defensive behaviour. The
   0/3 is purely the seeded-defect *grading* contract; conflating "rejected the plan"
   with "failed the review" obscures that the reviewer did high-quality work that the
   grader refuses to see.
6. **"Missing thresholds caused the fail-closed reasons to fire."** — Partially true
   for `MISSING_DENOMINATOR` (see RC-05) but the semantic result and other reasons
   (`APPROVAL_REJECTED`, `PLAN_NOT_APPROVED`) are driven by the real rejection and are
   correct.

---

## 5. Interaction with the adoption gate and what it means

- **Fail-closed is working.** Both runs refuse adoption; Reviewer B rejected, binding,
  schema, provenance, and oracle terminality all passed; the archive stays
  non-adoptable. Nothing here weakens the gate.
- **But the detection evidence is currently un-informative.** A grader that filters out
  every semantically-correct finding before grading cannot distinguish "reviewer never
  saw the mutation" from "reviewer saw and fixed it" — so the 0-rate cannot yet be read
  as a reviewer capability statement. Every recommended next experiment in `analysis.md`
  (§ "Recommended next analysis questions", items 1, 2, 4, 7; § "Recommended next
  experiments") should be run **after** the grader contract is fixed and the fixture set
  is extended, or a repeat of this cohort will likely reproduce a grader-driven 0 again
  and be misread as a model/prompt weakness.

---

## 6. Recommended fixes (small, evidence-bound)

Order does not matter for correctness, but 6.1/6.2 are the ones that change the 0/3.

1. **Grader: make `path` matching tolerant.** Accept the finding if *any* `;`-separated
   `path` segment equals the defect path, or if the defect path is a segment/token
   substring of the finding path. Add regression fixtures with multi-file `path`
   values.
2. **Grader: make `location` matching tolerant.** Accept `§`-style (status quo),
   "section 3.1 / sec 3.1 / § 3.1" equivalents after normalizing `§`↔`section`↔`sec`,
   and a fallback that a location/evidence mention of the defect's filename is enough
   when no section notation appears. Add prose-location fixtures.
3. **Grader: give `signal_matches` the same leniency as `correction_matches`** — a
   token-overlap fallback plus hyphen/number normalization
   (`fourth-generated-button` ≡ `fourth generated button`, `4` ≡ `fourth`), or scope
   signal matching to also read `required_correction`.
4. **State/difid:** decouple threshold from denominator: produce a distinct
   `MISSING_THRESHOLDS` reason (or record thresholds as a first-class config field in
   the batch), and have `run-benchmark.sh` export the configured thresholds so the
   live cohort stops shipping `null`s. Add a test asserting the state reason labels the
   actual missing component.
5. **Fixture set:** extend `tests/fixtures/` with (a) multi-file `path`, (b) prose
   `location`, (c) hyphenated signal, (d) `;`-path consolidation — the shapes the live
   reviewer actually produces — so the oracle's own guarantees are tested against
   editorial realism, not self-fulfilling fixtures.
6. **Reviewer prompt (optional, worst contract):** if findings must name a canonical
   file, say so explicitly and give a location *style* example; this keeps reviewer
   freedom while aligning output with the grader. (A blinded reviewer still must not
   be told the seed, only the citation convention.)

---

## 7. What is still open / needs one more evidence pass

- **SD-01 fresh-mode observation vs. re-scoping** (RC-04): the Reviewer B transcript
  (`reviewers/…/workspace/reviewer.jsonl`) was not retained in the published archive,
  so I could not separate "did not perceive" from "perceived but wrote under AR-01's
  US-01 scope". Re-run with transcript retention, or grep the two worker `worker.jsonl`
  files for "third" near reviewer session boundaries.
- **Encrypted private map**: my replay used `pilot-blinded-defects.json` (the public
  twin used by both the seed and the fixtures). The two archived runs' encrypted maps
  are gone after publication; if the private seed roots are recoverable, re-verify the
  replay against each run's own manifest (locations/signals are expected to be
  identical — the seed source is the same JSON).
- **Repeat-count**: this pair (one iterative + one fresh) is enough to *fail adoption*
  but not to estimate a stable reviewer detection rate — consistent with `analysis.md`
  recommendation 8.
- **IR run taint (`VALIDATION_FAILED`)**: confirmed as the worker's own final
  validation (adversarial-review approval not mirrored), orthogonal to grading; no
  further trace needed unless the process-audit angle is pursued.

---

## 8. One-paragraph bottom line

> **Plain English — bottom line.** The current 0/3 is overwhelmingly an adjudicator
> contract problem, not a reviewer blindness problem: the grader drops every finding
> that does not cite the exact file `plan-description.md` and the exact location string
> `plan-description.md § 3.1`, rules the blinded reviewer was never told and the schema
> check never enforces, and that gate alone accounts for 0/3 in the fresh run and the
> location block in the iterative run. A brittle phrase matcher (hyphenation) is the
> one extra blocker for one defect in the iterative run, SD-01 was genuinely the
> subtlest mutation in the fresh run, and the `MISSING_DENOMINATOR` reason is an
> unrelated thresholds-labeling bug. The system is correctly fail-closed; the next
> cohort should fix the grader's realism and the fixture set before any conclusion is
> drawn about reviewer capability, modes, models, or task wording.