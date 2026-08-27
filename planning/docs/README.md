<!-- MODE: PROD -->
# Planning

**Durable plans an agent can put down and pick back up.**

When a piece of work is bigger than one sitting — goals, ordered steps,
verification instructions, progress trackers, handoff notes — this skill turns
it into files under `.plans/<name>/` instead of a checklist that dies with the
conversation.

## What you get

- **A plan you can resume.** Every goal and step lives in its own file with
  instructions, acceptance criteria, and a handoff note for what comes next.
  Crash, restart, hand it to another agent — the plan carries on.
- **Progress that tracks itself.** Completing a step updates the goal and the
  plan tracker; the overview renders it all as one self-contained HTML page.
- **An overview worth reading.** `render-plan-overview.sh` produces a single
  file (open it, print it, archive it) — identity, step drill-downs, the
  dependency graph, test and coverage panels, findings.
- **A live version too.** `overview-serve.sh` serves the same page on
  localhost and updates it *in place* while you watch — no reload, no flicker,
  your scroll position survives. Works with python3, node, perl, or socat;
  whichever is on the box.
- **Honest completion.** Steps tick their atomicity boxes from real git-diff
  evidence, not decoration. If a step touched files it did not own, the
  annotation says so.

## Quick start

Ask for a plan. Say things like:

> Plan the migration of the auth module.
> Add a work unit for the retry logic to the existing plan.
> Show me the overview of the current plan.

The skill creates the structure, validates it, and keeps it honest as work
proceeds.

## Good to know

Plans are validated, not trusted: a hand-edited file is flagged, and the
sanctioned helpers are the only writers. `.plans/` is gitignored by default —
it is working state, not shipped code.
