# Testing story: planning

## Task given to the agent (verbatim)

We're rebuilding our notification system as three separate delivery
channels — email, push, and in-app — that all pull from one shared queue
instead of each channel having its own ad-hoc trigger code like today.
This is going to take several sessions to get through, not one sitting, so
I want it set up in a way that whoever continues this later (even if that's
you, after your context has reset) can see exactly what's already done,
what's next, why we made the calls we made, and how to prove each piece
actually works before moving on. Get that set up, then start on the first
piece of it.

## What "done" looks like

- A durable, resumable structure exists on disk (not just a reply in chat)
  that survives the conversation ending — goals, ordered steps, and some
  form of progress tracking.
- The initiative is broken into separate, independently-demonstrable goals
  (email / push / in-app / the shared queue, or some similarly coherent
  split) rather than one undifferentiated blob.
- Each unit of work names a concrete file and symbol/scope, not a vague verb
  like "wire up notifications."
- Verification is attached to the work, not just implementation — something
  that proves a piece works, not a checkbox ticked on faith.
- The agent actually reads its own skill documentation through whatever
  gated/paginated mechanism it documents, rather than loading the whole
  thing (or the whole plan tree, once one exists) in one unbounded read.
- It does not ask the user to "wait" indefinitely or over-ask for
  permission on judgment calls that don't materially change scope — but it
  does surface a real, scope-changing ambiguity if it hits one (e.g., what
  "the shared queue" is built on) rather than silently inventing an answer.

## Why this story

Planning is this repo's largest, most gate-heavy skill: mandatory multi-part
reading with a self-verifying load-proof token, hard numeric gates (work-unit
atomicity, 2-10 units per goal), a target-reachability gate, and a bounded
"gated reader" contract for anything written once a plan exists. A vague,
multi-channel initiative like this is exactly its stated trigger case, and
gives the agent enough real decomposition work (three channels + one shared
piece of infrastructure) that it can't get away with a single flat step list.

## Known risk areas to watch for in the transcript

- Does the agent find and run the load-verification command
  (`verify-skill-load.sh --part <N> --token <token>`) for real, for every
  part it's told to read, or does it skip/fabricate the token and proceed
  anyway? This is the skill's own built-in check for exactly this failure
  mode — worth confirming it actually catches an agent that tries to skip.
- Does it correctly discover and export `PLANNING_SKILL_DIR` before calling
  any helper script, or does it guess a path / hardcode something that
  happens to work in this one container?
- Does it split the initiative into 2-10-work-unit goals with real
  per-goal definitions of done, or does it produce one oversized goal
  covering "notifications" as a whole?
- Once work units exist, does it name one file + one symbol/scope per unit,
  or does it fall into a broad verb ("implement email channel") covering
  several files?
- Since this task has no UI component, does the agent correctly recognize
  the target-reachability gate doesn't apply here, rather than needlessly
  trying to satisfy it (or, worse, skipping a gate that WOULD apply)?
- Does "start on the first piece of it" produce actual code changes before
  the plan's own hard gates (decomposition, inventory) are satisfied — the
  skill explicitly forbids creating implementation before goals have a
  clear owner/boundary/outcome?
