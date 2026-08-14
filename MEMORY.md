# MEMORY — ai-skills session handoff (2026-08-13)

Continue here tomorrow. Everything below is the current, verified state.

## What this session did

Implemented the **worker-personas** system (a full plan: 5 goals, 17 work units),
then ran multiple post-implementation-review passes, and finally did a general
cleanup. All code changes are **UNCOMMITTED**.

### Implemented (shipped/working, verified 23/23 suite green)
- **Persona identity + scope**: `planning/scripts/role-context.sh` is the machine
  registry (`ROLES=()` 11 personas) + scope (`role_docs()`), injects per-role
  voice preamble from `roles/VOICES.md`, FAILS CLOSED on unset/unknown `ROLE_ID`,
  has a sourcing guard (sourced ≠ executed) so `resolve_id` is reusable.
- **Per-role reader composition**: `plan-context.sh` + `plan-context-lib.sh`
  gate ALL subcommands (init/read/check/refresh/checkpoint) per role;
  `installer|oracle|eve` are refused plan content; budgets capped via
  `printf -v` (no eval); no-ROLE_ID identity-free path preserved (adversary-probe
  fixture relies on it).
- **Monitor supervision**: `planning/scripts/supervision-frame.sh` (bounded ~2048B
  frame emitter, footer-overwrite, grant log = case+command never reasoning) +
  `planning/scripts/monitor-read.sh` (maintainer-only reader, pull-on-exception,
  symlink-safe `readlink -f`).
- **Benchmark wiring**: `benchmark/planning/runtime/lib-agent.sh`
  `persona_id_for` routes worker→benny, **reviewer-a→christian,
  reviewer-b→christoph** (re-mapped in a late review round to match the
  contract), analyzer→alex, oracle→oracle, post-run→frank. `persona_bootstrap` +
  `persona_bootstrap_prompt` inject ROLE_ID env + voice into worker/analyzer/
  reviewer prompts. `chris` stays the *planning* adversarial-review persona via
  `ROLE_ID=chris` (SKILL.md + run-adversary-probe.sh), independent of persona_id_for.
- **Installer/manifest**: `install.sh skill_files()`, `V27-PACKAGE-MANIFEST.txt`,
  `V27-PACKAGE-MAP.tsv` all at **74 installable rows** and byte-consistent;
  `test-installer-manifest.sh` derives the count from the map (no literal) and
  reconciles skill_files()↔manifest↔map. Removed a dangling `source_only` map row.
- **New tests** (all in suite): test-persona-drift.sh, test-voice-artifact-drift.sh,
  test-supervision-frame.sh, test-progress-bar-shape.sh (+ fixtures/progress-shape
  and progress-shape-bad). Persona + voice drift tests derive the registry from
  `role-context.sh --list` (identity-free), not a hardcoded copy.
- **Docs**: `MAINTAINER.md` (artifact map incl. VOICES.md/monitor-read/supervision-frame/
  plan-context, §2.5 fail-closed, §2.9 reader composition, §2.10 supervision, checklist),
  `MAINTAINER-STYLE-CONTRACT.md` (persona & reader system section, roles-master rows
  for maintainer=supervision-monitor and dana=NOT supervision-frame monitor,
  progress-bar shape contract, drift-test checklist), `planning/ROLES.md`
  (persona matrix is a **mirror** of role_docs(), not the source), and the
  `role-context.sh` header (FAIL-CLOSED + voice + sourcing-guard).

## REVIEW-HANDED-DOWN DECISION (important, do not revert)
- Reviewer mapping is **reviewer-a→christian, reviewer-b→christoph** (matches
  contract: christian = handoff-only/never approves, christoph = sole approval).
  This was a user-chosen "rewrite the mapping" after reviewers flagged the old
  (chris/christian) wiring. `chris` remains the planning adversarial-reviewer only.
- F1 (dead `! grep -q` assertions) removed from test-installer-manifest.
- Everything else reviewed SOUND across multiple fresh alex+christoph passes.

## Cleanup done
- **Deleted ALL `.plans/`** (gitignored, user-confirmed). All plan dirs gone:
  worker-personas, maintainer-doc-sync, reviewer-*, benchmark-agent-runtime,
  probe-adversary. Brainstorm scratch, plan docs, review reports all removed.
- **RECOVERY**: `benchmark/planning/tests/test-review-lifecycle.sh` (committed
  version) depends on `.plans/reviewer-oracle-evidence-hardening/` as a test
  fixture. That plan was restored from git history:
  `git archive 8dbd04f .plans/reviewer-oracle-evidence-hardening | tar -x -C .`
  (60 files). It is gitignored again (never commit it). WITHOUT it,
  test-review-lifecycle.sh fails (test-data dependency on gitignored path).
- Do NOT "make the test self-contained" — I tried that and it over-rotated
  (digging a hole in a hole). The committed test + restored gitignored fixture
  is the working state; `test-review-lifecycle.sh` passes as-is (verified).
- Cleaned stray `benchmark/results/*adapter-test` archives from my debugging.

## Current uncommitted changes
Modified: README, benchmark/planning/{README,benchmark-test,run-benchmark,
session-id-from-jsonl,setup-benchmark,telemetry,worker-prompt,tests/test-capsule-access,
tests/test-safeguards}, install.sh, package.json, planning/{MAINTAINER,
MAINTAINER-STYLE-CONTRACT,REVIEWER,ROLES,SKILL,V27-PACKAGE-MANIFEST,V27-PACKAGE-MAP},
planning/scripts/{plan-context-lib,plan-context,role-context,run-adversary-probe},
planning/tests/test-installer-manifest.
Untracked: benchmark/planning/runtime/, benchmark/planning/tests/test-runtime-agent.sh,
brainstorm/, post-implementation-review/, planning/roles/VOICES.md,
planning/scripts/{monitor-read,supervision-frame}.sh,
planning/tests/fixtures/progress-shape(+-bad)/,
planning/tests/{test-persona-drift,test-progress-bar-shape,test-supervision-frame,test-voice-artifact-drift}.sh.

## Verification status (last run)
- Full suite: **23 passed / 0 failed / 2 UNCONFIGURED** (PLANNING_CONTEXT_CACHE-gated, expected).
- `git diff --check` clean; `bash -n` clean on all edited scripts.
- test-review-lifecycle.sh passes with restored fixture.

## What to do next (tomorrow)
1. Run `./run-tests.sh` first (confirm still green).
2. `git diff --stat` + review, then decide on a single coordinated commit
   (user has been asked several times about committing; not yet done).
3. Nothing is committed. Suggested commit message covers: worker-personas system,
   installer/manifest sync, maintainer-doc sync, progress-bar fix + tests.
