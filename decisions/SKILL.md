---
name: decisions
description: Use when an agent needs to raise a non-blocking, multiple-choice question mid-work -- stub a reasonable solution, keep working, and let the user answer later from the CLI or the decision-board mod, or let any agent glean open, decided, implemented, closed, or urgent questions over the stdio MCP tool. A decided question is outstanding work for the agent, not just the user's to answer: implement it, then mark it so. Do not use for a blocking question that must be answered before the current turn can continue -- use the harness's own question/confirmation mechanism for that.
---
<!-- MODE: PROD -->

# Decisions

A register of non-blocking questions, held in `DECISIONS.json` and managed
with the `decisions` command. Each question carries lettered answer options,
a priority, the branch it was raised on (as context, not a filter), and
whatever the agent stubbed while it stayed open.

This is not a replacement for a harness's own blocking question or
confirmation mechanism. It is for the question that does not need an answer
right now: stub a reasonable assumption, record the question as open, and
keep working. The user answers later, whenever they are back.

## The file

`DECISIONS.json` at the root of whatever holds the questions. If it does not
exist yet, write it yourself first, as the skeleton below with an empty
`questions` array and `skill_version` copied exactly as shown.

```json
{
  "skill": "decisions",
  "skill_version": "2.0.0-alpha.5",
  "comment": "Non-blocking questions raised during work.",
  "questions": [
    {
      "id": "Q1",
      "title": "Cache the parsed config, or re-read it every call?",
      "status": "decided",
      "priority": "normal",
      "branch": "feature/config-reload",
      "options": [
        {"letter": "a", "label": "Cache"},
        {"letter": "b", "label": "Re-read every call"}
      ],
      "context": "Stubbed with option b (re-read) while continuing.",
      "chosen": "b",
      "resolution": null,
      "created_at": "2026-01-01T00:00:00Z",
      "updated_at": "2026-01-01T00:00:00Z"
    }
  ]
}
```

## The `decisions` command

```
decisions add --title T --option a:LABEL --option b:LABEL [--option c:LABEL ...]
              [--priority normal] [--context C] [--file PATH]
decisions list [--status open|decided|implemented|closed|dropped|obsolete]
               [--priority urgent|high|normal|low|someday] [--branch B] [--file PATH]
decisions answer <ID> <LETTER> [--file PATH]
decisions stub <ID> <ASSUMPTION> [--file PATH]
decisions implement <ID> [NOTE] [--file PATH]
decisions apply <ID> <RESOLUTION> [--file PATH]
decisions close <ID> <RESOLUTION> [--file PATH]   (an alias for apply)
```

Any command takes `--file PATH`, which wins over everything else. Failing
that, the register is `DECISIONS_JSON`, else `./DECISIONS.json`.

`add` records the current git branch automatically, as context on the
question -- it travels with the question, it is not a separate filter
mechanism a later `list` call keys on by itself (though `--branch` can
narrow by it when that is useful).

A question raised while stubbing a solution should carry that stub in
`--context`, so a later reader of an open question knows what was actually
done in the meantime, not just that a question exists.

A question's lifecycle is `open -> decided -> implemented`. `answer` moves
it from open to decided once the user picks an option; that is still
outstanding work -- the agent's, not the user's -- until `implement` records
that the pick was actually carried out and moves it to implemented. A
decided question stays visible (in `decisions list --status decided`, in the
decision-board pane, and in the session-start reminder) for exactly that
reason: it does not vanish just because someone answered it. `close`/`apply`
can withdraw a question from any status, with or without ever implementing
it.

## Other ways to reach the register

- **The MCP adapter** (`decisions-mcp`, stdio only, no daemon, no port):
  exposes `list_open`, `list_decided`, `list_implemented`, `list_closed`,
  `list_urgent`, `answer`, `add`, `stub`, and `implement` as typed tool
  calls, for an agent that wants to glean or act on questions without
  shelling out.
- **The `decision-board` mod**: shows open and decided questions in a pane
  (toggle to "Implemented" to see what is already done), and lets a person
  press a button to choose an answer option directly, instead of running
  `decisions answer` by hand.
- **The `decision-reminder-plugin`**: at session start, reads the register
  and reminds the agent of open and urgent questions to answer, and
  separately names every decided question still awaiting implementation --
  the agent's own outstanding work, not forgotten just because the session
  changed.

## Migration

A register this version did not write is migrated transparently on read:
the original is backed up to a versioned `.back.json` beside it, and every
entry that still converts against the current shape is carried forward.
Whatever did not convert is reported, naming the id and the parse error.
