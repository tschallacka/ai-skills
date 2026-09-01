---
name: todo
description: Use when work is queued that outlives the conversation and needs to survive a restart, a handoff, or a compaction, so tasks and their sub-tasks live in one JSON file managed with the `todo` command. Also, when juggling three or more separate work items in sequence, ask the user whether to file them here before starting; never auto-file without asking. Do not use for a short in-chat checklist, for the steps of a task already in progress, for defects (use the bug-report skill), or where a planning skill's plan documents are the right home.
---
<!-- MODE: PROD -->

# Todo

A queue of work that outlives the conversation, held in `TODO.json` and managed
with the `todo` command. Tasks nest: a large task carries sub-tasks, and a
sub-task is a task in its own right that can be finished while its siblings wait.

**JSON, not prose.** A queue is structured data. Statuses have to be derived
rather than remembered, and a title is free text that will eventually contain the
characters a reader treats as syntax. Query it, never scrape it.

`todo` is a single prebuilt binary that ships with this skill. It needs no shell
and no other tool — not `rjq`, not `date` — so it behaves the same under bash,
zsh or anything else, and the queue can be written on a machine that has none of
them.

## The file

`TODO.json` at the root of whatever the work belongs to. One file per queue.
Every command takes `--file PATH`, which wins over everything else; failing that
the path comes from `TODO_JSON`, then `./TODO.json`.

```json
{
  "skill": "todo",
  "skill_version": "2.0.0-alpha.1",
  "comment": "What this queue is for, in one line.",
  "tasks": [
    {
      "id": "T1",
      "title": "Split the oversized library",
      "status": "partly",
      "priority": "high",
      "parent": null,
      "detail": "One file holds 47 functions against a 500-line cap. Split by concern, not by line count.",
      "blocked_on": null,
      "refs": ["planning/scripts/plan-document-lib.sh"],
      "note": "Two of four groups done.",
      "created_at": "2026-08-19T21:04:11Z",
      "updated_at": "2026-08-20T09:12:40Z"
    },
    {
      "id": "T1a",
      "title": "Extract the progress group",
      "status": "done",
      "priority": "normal",
      "parent": "T1",
      "detail": "Six functions: percent, bar, icon, status label, objective, reminder.",
      "blocked_on": null,
      "refs": [],
      "note": "a284423",
      "created_at": "2026-08-19T21:04:11Z",
      "updated_at": "2026-08-20T08:31:02Z"
    }
  ]
}
```

| field | meaning |
|---|---|
| `id` | short stable handle. A sub-task conventionally extends its parent's id (`T1` → `T1a`), but nesting comes from `parent`, not the spelling |
| `title` | one line, no trailing full stop, readable on its own |
| `status` | `open`, `partly`, `blocked`, `decided`, `done`, `dropped`, `obsolete` |
| `priority` | `urgent`, `high`, `normal`, `low`, `someday`. What order the work happens in, which is not the same as how big it is |
| `parent` | the id this nests under, or `null` for a top-level task |
| `detail` | what the task is and why it is worth doing |
| `blocked_on` | who or what must move first. Set it and set `status` to `blocked` |
| `refs` | files or documents whoever picks it up will need. Paths, never task ids |
| `note` | where it got to: a commit, a decision, a measurement, a reason for dropping it |
| `created_at` | when it was first recorded, ISO 8601 UTC |
| `updated_at` | when anything on it last changed. Every write sets it |

The field order above is the order on disk, and the tools reproduce it exactly:
a read-modify-write of an unchanged queue is byte-identical, so a diff shows only
what actually changed.

The two header fields sit outside the list, because they describe the file rather
than any row:

| field | meaning |
|---|---|
| `skill` | which skill's schema this file follows |
| `skill_version` | the schema version of the tools that last wrote it |

**There is no severity here.** A queued task is not more or less broken; that is
a defect's field, and defects belong in the `bug-report` skill. The two registers
were once read by one shared library that had to pretend they agreed, and every
place it pretended became a defect.

**Priority is not size and not severity.** It answers "what next", so it is
allowed to change without the task changing. `someday` is honest and useful: it
says recorded, not forgotten, and not now.

**Flat list, `parent` pointer.** Not nested arrays: appending a sub-task is one
object at the end of the list, with no walk to find its place and no rewrite of a
parent. The tree is reconstructed on read.

## Adding a task

Order carries no meaning, so append and stop thinking about it — every write
re-sorts the file anyway.

```sh
todo add --title "Move the artifact map into the coupling registry" \
         --detail "Some couplings live only in prose, where nothing can read them." \
         --priority normal --refs MAINTAINER.md
```

It prints the id it allocated. `--title` and `--detail` are required and the
command refuses without them: a task nobody else could pick up is a note to self,
and it will read as one in three weeks.

A sub-task is the same call with `--parent`:

```sh
todo add --title "Move the four generated-artifact rows" \
         --detail "The generated files and their builders, which change together." \
         --priority high --parent T9
```

A sub-task carries its own priority. A high-priority sub-task under a
low-priority parent is normal: it is how "one part of this matters now" gets
recorded.

Pass `--id` to choose the id yourself, which is how the suffixed sub-task ids
(`T41a`) get written — the allocator only ever suggests the next number.

The vocabulary is fixed and the binary will not accept anything outside it:

| Field | Accepted |
|---|---|
| `--status` | `open`, `partly`, `blocked`, `decided`, `done`, `dropped`, `obsolete` |
| `--priority` | `urgent`, `high`, `normal`, `low`, `someday` |

An out-of-vocabulary value is refused when the queue is *read*, not by a separate
check that a writer could skip.

## Closing a task

Set the status and say where it got to, in one call. A closure with nothing in
`--note` is refused: `done`, `dropped` and `obsolete` all owe an account of what
happened.

```sh
todo update T9a --status done --note "a1b2c3d, four rows moved"
```

**Dropping a task is a legitimate close.** Set `dropped` with the reason in
`--note`. A task that quietly disappears gets proposed again six weeks later.

`--append-note` adds to the note instead of replacing it, so the reason a task
was opened survives the record of how it ended.

Blocking one names who is blocking it:

```sh
todo update T7 --status blocked --blocked-on "the macOS CI run"
```

A parent is not closed by its children. Check first:

```sh
todo list --parent T9 --status open
```

`updated_at` is set on every write, including a priority change — a timestamp
maintained only sometimes is worse than none, because a reader cannot tell a
quiet task from a stale field.

A refused update changes nothing. The queue is left exactly as it was, rather
than written and then reported as broken.

## Reading it

These commands print what the user reads. Run one and pass its output through
unchanged: reformatting a rendered tree in a reply spends tokens rewriting text
that is already in its final shape, and every rewrite can alter a status or drop
a row.

The whole queue as an indented tree, children under their parent:

```sh
todo tree
```

```
⛔ T7  [urgent]  Push the branch and read the macOS legs
🔨 T1  [high]  Split the oversized library
  ⬜ T1b  [high]  Extract the table group
  ✅ T1a  [normal]  Extract the progress group
📌 T8  [normal]  Keep the current gate rather than promoting it
🚫 T9  [someday]  Move the artifact map into the coupling registry
```

Sorted by status, then priority, then id at every level, so the order is stable
between runs. A high-priority sub-task rises above its siblings without moving
out from under its parent.

| glyph | status | means |
|---|---|---|
| ⬜ | `open` | queued, nothing started |
| 🔨 | `partly` | started, or some sub-tasks closed |
| ⛔ | `blocked` | waiting on someone else; `blocked_on` names them |
| 📌 | `decided` | a choice, not work; `note` says what was chosen and why |
| ✅ | `done` | finished |
| 🚫 | `dropped` | deliberately not doing it |
| 🚫 | `obsolete` | overtaken by events: superseded, disabled, or no longer applies |

What is open, which is what a check-in asks for:

```sh
todo report
```

A blocked task with nothing recorded in `blocked_on` is called out there by name
rather than shown as a blank, because it is the one that stalls a working
session.

One task in full, as stored:

```sh
todo show T7
```

A filtered table, one tab-separated row per task, for feeding to something else:

```sh
todo list --status blocked
todo list --priority urgent
todo list --parent T1
todo list --touching installer/       # matches within the refs list
todo list --since 2026-08-01T00:00:00Z
todo count --status open
todo next-id
```

An absent filter matches everything rather than matching the empty string, so
`list` with no flags is the whole queue and not an empty one.

## Keeping the file in priority order

Nothing to do: every write re-sorts. The order is status, then priority, then the
numeric part of the id, so `T10` follows `T9` rather than preceding it, and a
suffixed sub-task id sorts with its number.

Status leads here where severity leads in the defect register, because the
queue's question is "what is still to do": a done task never sits above an open
one. `todo fmt` rewrites the file in canonical form without changing anything
else, for a file that was hand-edited.

## Pruning: keeping a queue a queue

A queue nobody reads is a list, and a hundred closed tasks is how it gets there.
Closed tasks move **out** — never away:

```sh
todo prune                                  # what would leave, and what is kept back
CONFIRM=<the token it printed> todo prune   # do it
```

The removed tasks are written to a dated `TODO.<date>.archive.json` beside the
queue, which is itself a queue: `todo --file TODO.2026-09-01.archive.json report`
reads it back unchanged. Two prunes on one day merge into that day's archive
rather than overwriting it.

A closed task is **kept back** while anything live still depends on it — while an
open task is its child, or names it exactly in `blocked_on`. Pruning it would
leave that open task pointing at an id no longer in the file. `--older-than
<ISO8601>` keeps recent closures too, for an archive of only the long-settled.

Nothing is written until the token it printed comes back, so the content approved
is the content written.

## Rules that keep it usable

**Re-read before reporting status.** The file is the state. A summary in the
conversation is a copy, and it is stale the moment anything lands.

**Split a task when it grows sub-tasks; do not widen its title.** A title needing
"and" is usually two tasks, and a sub-task can be closed while its siblings wait.

**Record a decision as a task.** When the answer is a choice rather than work,
set `decided` and put what was chosen and why in `--note`. Otherwise the same
choice gets relitigated. `decided` counts as open work: a decision recorded but
not acted on is still something to do.

**Name who is blocking, not that it is blocked.** `--blocked-on "the maintainer"`
or `--blocked-on "the macOS CI run"` is actionable; `blocked` alone is not. When
the blocker is another task, give the id and nothing else — the tools follow an
exact id through a rename and a prune, and cannot follow one buried in a
sentence.

**Priority is a judgement you are allowed to revise.** Re-prioritising is a write
like any other. A queue whose priorities never move is a queue nobody is reading.

**Do not delete a task.** Close it as `done`, `dropped` or `obsolete` with a
reason, and let `prune` move it out. A deleted task takes its reasoning with it.

## Validating the file

```sh
todo check
```

Prints `<n> tasks, sound`, or every rule the file breaks — and exits non-zero in
that case, so it fits a hook or a CI job.

A finding here means more than "the file is malformed". Every writer refuses what
`check` reports, so a task that breaks a rule **did not come from the tools**:
the file was edited by hand, or by something imitating their output.

Most of the rules cannot be broken any more. The status and priority vocabularies
are types, so an out-of-vocabulary value fails when the file is *read*, naming
the field and the line:

```
unknown variant `wip`, expected one of `open`, `partly`, `blocked`, `decided`, `done`, `dropped`, `obsolete` at line 10 column 28
```

What `check` still reports is the set of rules that need more than one field to
decide: unique ids, a `parent` that resolves, timestamps present, and the
evidence a closure owes.

### A queue written by an older version

`todo` refuses to read one in place and says so. Convert it:

```sh
todo migrate
```

That copies the file to a versioned `TODO.<version>.back.json` **before parsing
anything**, carries the tasks that still fit the current shape and are still
open, leaves the closed ones in the backup, and reports anything it could not
convert with the commands to move it by hand.

There is no compatibility layer and there will not be one. A task the converter
does not understand is handed back to you — with the backup to read it from and
`todo add` to re-file it — rather than guessed at. Nothing is lost by leaving one
unconverted: the backup keeps it.

### A merge conflict in the queue

Two branches that both queue a task both take the same next id, so the conflict
is semantic: one id, two unrelated tasks, and neither side wrong.

```sh
todo resolve                                  # what has to be decided
todo resolve theirs:T97:T125 > preview.json   # decide it; nothing is written
CONFIRM=<the token it printed> todo resolve theirs:T97:T125
```

It reads both clean sides out of the git index, so it works mid-conflict with
nothing checked out. It follows a rename into `parent` and into a `blocked_on`
that is exactly that id, leaves `refs` alone because those are paths, reports ids
left in prose rather than rewriting them, refuses a queue whose own side is
already unsound, and writes nothing until the token it printed comes back.

Pass every decision in one run. A partial set is refused with the ones still
missing, rather than applied halfway.

## When not to use this

A defect belongs in the `bug-report` skill, which asks for a reproduction and a
verification this schema has no place for. A checklist inside one task, finished
before the conversation ends, belongs in the conversation. Work needing ordered
goals, verification instructions and progress tracking belongs in a planning
skill's documents.
