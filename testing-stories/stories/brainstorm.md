# Testing story: brainstorm

## Task given to the agent (verbatim)

Let's brainstorm something. I want to add a "favorites" feature to our
product so people can bookmark items and get back to them later. I don't
know yet whether that should be a simple star toggle, a full collections
system with folders, or something in between, and I haven't thought through
how it should behave across devices or whether other people should ever be
able to see what someone has favorited.

## What "done" looks like

- The agent recognizes the explicit "let's brainstorm" invocation and enters
  the skill's collaborative back-and-forth rather than jumping straight to
  a plan or an implementation.
- A living document is written to disk under a per-initiative path (not
  just held in the conversation), and it is updated as the conversation
  progresses rather than written once at the end.
- The agent asks real, focused questions about the genuinely open
  dimensions the prompt names (star vs. collections vs. hybrid,
  cross-device behavior, visibility to others) rather than silently
  deciding all three itself.
- At some point a fresh, separate agent (not the main agent reusing its own
  context) runs an adversarial completeness pass over the captured idea.
- Any question raised in that pass that would change scope is taken back to
  the user in a batch, with multiple-choice options plus a free-type
  option — not a bare open-ended re-ask.
- The session ends at (or the transcript shows it reaching) the decision
  gate: plan vs. implement now — it does not silently start writing
  application code, and it does not silently start producing
  goals/steps/work-unit-inventory structure itself (that's the planning
  skill's job, handed off, not done inline).
- The document records explicit non-goals and open/undecided items, not
  just the happy path.

## Why this story

This is the skill's own headline trigger phrase ("let's brainstorm") paired
with a genuinely under-specified feature request with real, materially
different directions (a toggle vs. a whole collections system) and a
real cross-cutting unknown (visibility to other people) — exactly the shape
the skill exists for, and rich enough that a shallow "just ask one question
and move on" response would visibly under-deliver against the skill's own
five-phase process.

## Known risk areas to watch for in the transcript

- **Cross-skill dependency gap**: Phase 2 explicitly instructs spawning the
  adversarial-pass subagent with `ROLE_ID=eve` and loading its persona via
  `<PLANNING_SKILL_DIR>/scripts/role-context.sh eve` — but this container
  has only the `brainstorm` skill installed, not `planning`, so that path
  does not exist and `PLANNING_SKILL_DIR` is never set. Does the agent
  notice this dependency is missing and adapt (e.g., run a plain adversarial
  pass without the persona machinery, or say plainly that a documented step
  can't be completed here), or does it silently skip the entire adversarial
  phase without flagging that anything was skipped? This is the single most
  likely real finding this story can produce.
- Does it actually spawn a separate subagent for Phase 2, or does it "play
  adversary" in its own context (which the skill explicitly forbids, since
  the same context already holds and will defend its own decisions)?
- Does it correctly batch open questions (at most 5 at a time, each with
  options plus a free-type choice) rather than asking them one at a time or
  as a single unstructured paragraph?
- Does it stop at the decision gate and actually ask "plan or implement," or
  does it assume one answer and proceed without asking?
- Does the brainstorm document end up under `.plans/<initiative>/brainstorm.md`
  with a real derived slug, or does the agent invent a different location?
