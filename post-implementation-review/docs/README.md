<!-- MODE: PROD -->
# Post-implementation review

**The review you want after the code exists.**

Built something and suspect it left something behind — bugs, undecided
choices, paths you never executed? This skill runs the after-the-fact review
in three passes and ends with concrete proposed fixes, not a feelings report.

## What you get

- **Pass 1 — implementer self-analysis.** The one who wrote it re-reads it
  with fresh eyes and classifies every suspicion: confirmed, partial, or
  not-a-bug.
- **Pass 2 — an independent solutions agent.** A second agent with no stake
  verifies each finding against the code and proposes the minimal fix,
  traced to an observed defect — never best-practice boilerplate.
- **Pass 3 — a critical-feedback agent.** A third tears into the proposed
  fixes: do they actually fix it? over-reaching? under-reaching? Each gets a
  verdict: apply, apply-with-changes, reject, or document-only.
- **One report, one decision.** Everything lands in
  `post-implementation-review.md` with an apply-or-decline call per fix.
  Declined fixes stay recorded — nothing silently disappears.

## Quick start

> Review what we just built.

The skill offers the review when an implementation ends with residual risk;
say yes, or invoke it directly after your own work.

## Good to know

This is the *after* pass. The adversarial review *before* implementation is
the planning skill's job — different muscle, different time.
