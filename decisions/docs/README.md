<!-- MODE: PROD -->
# Decisions

**A question register for work that does not stop to wait.**

`DECISIONS.json` — one file, read with the `decisions` command, holding every
non-blocking question an agent raised mid-work: lettered options, a
priority, the branch it came from, and whatever was stubbed while it stayed
open.

## What you get

- **A stub, not a stall.** An agent facing an unresolved choice stubs a
  reasonable answer, records the question as open, and keeps working. The
  user answers later, whenever they are back — this never replaces a
  harness's own blocking question or confirmation mechanism.
- **Lettered options, not a guess.** Every question offers concrete choices
  (`a`, `b`, `c`, ...), so answering is a pick, not free-form prose to parse.
- **Priority is a fact.** `urgent`/`high`/`normal`/`low`/`someday`, so "what
  needs an answer first" is a filter, not an inference.
- **The branch travels as context.** `add` records the current git branch
  automatically on the question itself, alongside whatever else grounds it —
  not as a separate mechanism to merge or reconcile across branches.
- **Three ways in.** The `decisions` CLI, the `decisions-mcp` stdio adapter
  (no daemon, no port) for an agent that wants typed tool calls, and the
  `decision-board` mod for a person to see and answer questions in a pane.
- **Decided is not done.** A question's lifecycle is `open -> decided ->
  implemented`. The user answering it does not make it vanish: a decided
  question stays visible everywhere (the pane, the CLI, the session-start
  reminder) until an agent actually carries out the pick and marks it
  `implemented` -- it is the agent's outstanding work, not just a record of
  the user's choice.

## Quick start

```
decisions add --title "Cache the parsed config, or re-read it every call?" \
  --option a:Cache --option b:"Re-read every call" --priority normal \
  --context "Stubbed with option b (re-read) while continuing; caching needs an invalidation story this change does not need yet."
Q1

decisions list --status open
Q1 [Normal/Open] Cache the parsed config, or re-read it every call? (whatever branch you were on)

decisions answer Q1 b

decisions implement Q1 "Confirmed: re-read every call, parsing is cheap here"
```

A question can also be left open while recording what was assumed, rather
than decided and implemented in one step:

```
decisions stub Q1 "Assumed option b for now; revisit once load testing exists"
decisions close Q1 "Withdrawn: load testing landed, caching is not worth it"
```

`close`/`apply` can withdraw a question from any status, with or without
ever implementing it — a decided question that turns out not to be worth
doing is `close`d, not `implement`ed.

## Good to know

A question answered from the `decision-board` mod shells out to this same
binary — there is exactly one writer of `DECISIONS.json`, whichever front
end the person or agent used.

A register this version did not write is migrated transparently on the next
read: the original bytes are backed up to a versioned `.back.json` beside
it, and every entry that still converts against the current shape is
carried forward. What does not convert is reported by id and parse error,
never silently dropped.
