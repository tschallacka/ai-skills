# Testing story: post-implementation-review

## Task given to the agent (verbatim)

I just finished writing rate_limiter/rate_limiter.py a few minutes ago. I
only tested the happy path (see rate_limiter/test_happy_path.py) and I'm not
confident it's correct under concurrent use or unusual inputs. Please give it
a hard, thorough review, tell me what's actually wrong with it, and fix
whatever needs fixing.

## What "done" looks like

- The agent recognizes this as exactly the scenario this skill exists for
  (residual risk after an implementation, only partially verified) and runs
  something resembling the three-phase review, not just a quick read-through.
- It finds at least the two real, concrete bugs in the fixture: `reset()`
  raises `KeyError` for a user who was never rate-limited yet, and the
  limiter has no thread/process-safety despite `allow()` reading then writing
  shared state (a classic check-then-act race) — a real concurrency defect
  given the task explicitly asks about "concurrent use."
- Findings are grounded in `file:line`, not vague impressions (per the
  skill's own "no vibes" rule).
- It proposes and (per the task's explicit request) applies concrete, minimal
  fixes, then re-runs `test_happy_path.py` to confirm nothing regressed.
- **The critical part to watch**: the skill's Phase 2/3 instructs spawning
  subagents under specific personas (`ROLE_ID=alex`, `ROLE_ID=christoph`) via
  `<PLANNING_SKILL_DIR>/scripts/role-context.sh` — a script that does not
  exist in this container, since only `post-implementation-review` is
  installed, not `planning`. What does the agent actually do when it hits
  that instruction? (See "Known risk areas.")

## Why this story

This skill's core value is the three-phase structure (self-review, an
independent second opinion, an adversarial third pass) — a single-phase
"looks fine to me" pass defeats the point. This story hands the agent a
realistic "I just built this, not fully confident" scenario (its literal
entry-gate condition) with genuine, non-obvious bugs, so whether it reaches
for the full documented process — and copes with a hard cross-skill
dependency it cannot satisfy — is directly observable.

## Known risk areas to watch for in the transcript

- **Undocumented hard dependency on the `planning` skill.** `SKILL.md`
  Phases 2 and 3 both say to spawn a subagent with `ROLE_ID=alex` /
  `ROLE_ID=christoph` and load its role docs via
  `<PLANNING_SKILL_DIR>/scripts/role-context.sh` — but `PLANNING_SKILL_DIR`
  and that script only exist if the `planning` skill is *also* installed,
  which nothing in `post-implementation-review/SKILL.md` states as a
  prerequisite. In this container it genuinely isn't installed. Does the
  agent: (a) silently skip the persona step and just run two generic
  subagents, (b) get stuck trying to resolve a path that doesn't exist,
  (c) notice and say so explicitly, or (d) something else? Any of (a)/(b) is
  a real, fixable documentation gap (the dependency should be stated, or the
  skill should degrade explicitly when it's absent).
- The skill also references `planning/references/comment-discipline-contract.md`
  for comment-hygiene review — same missing-dependency question applies.
- The final report path is `.plans/<initiative>/post-implementation-review.md`
  — another `planning`-skill convention (the `.plans/` directory). Does the
  agent invent a substitute location, ask, or just write it there anyway
  (creating `.plans/` itself, which may or may not be reasonable)?
- Whether "Do not load any skill on your own…" instructions to subagents are
  followed even though there are no other skills installed to accidentally
  load in this container (a weaker test of that clause here than in a
  multi-skill environment — note if the story should be re-run later with
  `planning` also installed via `--skills "post-implementation-review
  planning"` to test the intended dependency path for real).
