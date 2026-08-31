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
- **An overview worth reading.** `plan-overview --plan-dir DIR --out FILE` produces a single
  file (open it, print it, archive it) — identity, step drill-downs, the
  dependency graph, test and coverage panels, findings.
- **A board across every plan.** `render-plans-board.sh` answers the question
  asked before you open any one plan: of the twenty directories under your
  plans root, which are being worked, which wait on a review, which are done,
  and which are not plans at all. One card per plan, linking into that plan's
  own overview.
- **A live version too.** `plan-overview --plan-dir DIR --serve --port PORT` serves
  the artifact itself on loopback. If no matching prebuilt artifact exists, the
  installer reports that the plan overview is unavailable on that platform.
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
