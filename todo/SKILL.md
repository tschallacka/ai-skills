---
name: todo
description: Use when work is queued that outlives the conversation and needs to survive a restart, a handoff, or a compaction, so tasks and their sub-tasks live in one JSON file read with jq. Do not use for a short in-chat checklist, for the steps of a task already in progress, for defects (use the bug-report skill), or where a planning skill's plan documents are the right home.
---

# Todo

A queue of work that outlives the conversation, held in `TODO.json` and read with
`jq`. Tasks nest: a large task carries sub-tasks, and a sub-task is a task in its
own right that can be finished while its siblings wait.

**JSON, not prose.** A queue is structured data. Statuses have to be derived
rather than remembered, and a title is free text that will eventually contain the
characters a reader treats as syntax. Query it, never scrape it.

## The file

`TODO.json` at the root of whatever the work belongs to. One file per queue.

```json
{
  "comment": "What this queue is for, in one line.",
  "tasks": [
    {
      "id": "T1",
      "title": "Split the oversized library",
      "status": "partly",
      "parent": null,
      "detail": "One file holds 47 functions against a 500-line cap. Split by concern, not by line count.",
      "note": "Two of four groups done.",
      "blocked_on": null,
      "refs": ["planning/scripts/plan-document-lib.sh"]
    },
    {
      "id": "T1a",
      "title": "Extract the progress group",
      "status": "done",
      "parent": "T1",
      "detail": "Six functions: percent, bar, icon, status label, objective, reminder.",
      "note": "a284423",
      "blocked_on": null,
      "refs": []
    }
  ]
}
```

| field | meaning |
|---|---|
| `id` | short stable handle. A sub-task conventionally extends its parent's id (`T1` → `T1a`), but nesting comes from `parent`, not the spelling |
| `title` | one line, no trailing full stop, readable on its own |
| `status` | `open`, `partly`, `blocked`, `decided`, `done`, `dropped`, `obsolete` |
| `parent` | the id this nests under, or `null` for a top-level task |
| `detail` | what the task is and why it is worth doing |
| `note` | where it got to: a commit, a decision, a measurement, a reason for dropping it |
| `blocked_on` | who or what must move first. Set it and set `status` to `blocked` |
| `refs` | files or documents whoever picks it up will need |

Only `id`, `title` and `status` are required.

**Flat list, `parent` pointer.** Not nested arrays: appending a sub-task is one
object at the end of the list, with no walk to find its place and no rewrite of a
parent. The tree is reconstructed on read, which is `jq`'s job.

## Adding a task

Order carries no meaning, so append and stop thinking about it.

```sh
jq '.tasks += [{
      "id": "T9", "title": "Move the artifact map into the coupling registry",
      "status": "open", "parent": null,
      "detail": "Some couplings live only in prose, where nothing can read them.",
      "note": null, "blocked_on": null, "refs": ["MAINTAINER.md"]
    }]' TODO.json > TODO.json.tmp && mv TODO.json.tmp TODO.json
```

A sub-task is the same call with `parent` set:

```sh
jq '.tasks += [{
      "id": "T9a", "title": "Move the four generated-artifact rows",
      "status": "open", "parent": "T9",
      "detail": "The generated files and their builders, which change together.",
      "note": null, "blocked_on": null, "refs": []
    }]' TODO.json > TODO.json.tmp && mv TODO.json.tmp TODO.json
```

**Write to a temp file and rename.** `jq ... TODO.json > TODO.json` truncates the
file before `jq` reads it, and the queue is gone.

## Closing a task

Set the status and say where it got to, in one call.

```sh
jq '(.tasks[] | select(.id == "T9a") | .status) = "done"
  | (.tasks[] | select(.id == "T9a") | .note) = "a1b2c3d, four rows moved"' \
  TODO.json > TODO.json.tmp && mv TODO.json.tmp TODO.json
```

A parent is not closed by its children. Check, then close it deliberately:

```sh
jq -r --arg id T9 '[.tasks[] | select(.parent == $id and .status != "done")] as $left
  | if ($left | length) == 0 then $id + " has nothing open under it"
    else $id + " still has: " + ($left | map(.id) | join(", ")) end' TODO.json
```

**Dropping a task is a legitimate close.** Set `dropped` with the reason in
`note`. A task that quietly disappears gets proposed again six weeks later.

## Reading it

These commands print what the user reads. Run one and pass its output through
unchanged: reformatting a rendered tree in a reply spends tokens rewriting text
that is already in its final shape, and every rewrite can alter a status or drop
a row.

The whole queue as an indented tree:

```sh
jq -r '
  def glyph: {open:"💤", partly:"⏳", blocked:"⛔", decided:"📌",
              done:"✅", dropped:"✔️", obsolete:"🚫"}[.status] // "❔";
  def render($parent; $depth):
    (.tasks[] | select(.parent == $parent)) as $task
    | ("  " * $depth) + ($task | glyph) + " " + $task.id + "  " + $task.title,
      render($task.id; $depth + 1);
  render(null; 0)' TODO.json
```

```
⏳ T1  Split the oversized library
  ✅ T1a  Extract the progress group
  💤 T1b  Extract the table group
⛔ T7  Push the branch and read the macOS legs
📌 T8  Keep the current gate rather than promoting it
🚫 T9  Move the artifact map into the coupling registry
```

| glyph | status | means |
|---|---|---|
| 💤 | `open` | queued, nothing started |
| ⏳ | `partly` | started, or some sub-tasks closed |
| ⛔ | `blocked` | waiting on someone else; `blocked_on` names them |
| 📌 | `decided` | a choice, not work; `note` says what was chosen and why |
| ✅ | `done` | finished |
| ✔️ | `dropped` | deliberately not doing it; the grey tick is not an achievement |
| 🚫 | `obsolete` | overtaken by events: superseded, disabled, or no longer applies |
| ❔ | — | a status this table does not know, which is itself worth looking at |

What is live, one line each:

```sh
jq -r '.tasks[] | select(.status == "open" or .status == "partly" or .status == "blocked")
       | "\(.id)\t\(.status)\t\(.title)"' TODO.json | column -t -s "$(printf '\t')"
```

Where things stand, for "what is left":

```sh
jq -r '.tasks | group_by(.status)[]
       | "\(.[0].status): \(length)  \(map(.id) | join(" "))"' TODO.json
```

Ready to start — open, with nothing open beneath it:

```sh
jq -r '.tasks as $all | $all[] | . as $task
  | select(.status == "open")
  | select([$all[] | select(.parent == $task.id and .status != "done")] | length == 0)
  | "\(.id)  \(.title)"' TODO.json
```

`. as $task` is load-bearing. Inside the inner `select`, `.id` is the inner
task's id, so `.parent == .id` compares each task's parent to its own id, is
never true, and the filter silently passes everything.

One task in full:

```sh
jq -r --arg id T7 '.tasks[] | select(.id == $id) | "
\(.id)  \(.title)
Status:  \(.status)\(if .blocked_on then "   Waiting on: " + .blocked_on else "" end)

\(.detail // "no detail recorded")

Where it got to: \(.note // "nothing recorded")
\(if (.refs | length) > 0 then "See: " + (.refs | join(", ")) else "" end)"' TODO.json
```

Blocked work and who owes it, which is what a check-in asks for:

```sh
jq -r '.tasks[] | select(.status == "blocked")
       | "\(.id)  \(.title)\n    waiting on: \(.blocked_on // "unrecorded")"' TODO.json
```

## Rules that keep it usable

**Re-read before reporting status.** The file is the state. A summary in the
conversation is a copy, and it is stale the moment anything lands.

**Split a task when it grows sub-tasks; do not widen its title.** A title needing
"and" is usually two tasks, and a sub-task can be closed while its siblings wait.

**Record a decision as a task.** When the answer is a choice rather than work,
set `decided` and put what was chosen and why in `note`. Otherwise the same
choice gets relitigated.

**Name who is blocking, not that it is blocked.** `blocked_on: "the maintainer"`
or `blocked_on: "the macOS CI run"` is actionable; `blocked` alone is not.

**Do not delete a task.** Close it as `done`, `dropped` or `obsolete` with a
reason. A deleted task takes its reasoning with it.

## Validating the file

```sh
findings="$(jq -r '
  ([.tasks[].id]) as $ids
  | if ([.tasks[].id] | length) != ([.tasks[].id] | unique | length) then "duplicate ids" else empty end,
    (.tasks[] | select(.parent != null and (.parent | IN($ids[]) | not))
              | "\(.id) names a parent that does not exist: \(.parent)"),
    (.tasks[] | select(.status | IN("open","partly","blocked","decided","done","dropped","obsolete") | not)
              | "\(.id) has an unknown status: \(.status)"),
    (.tasks[] | select(.status == "blocked" and (.blocked_on == null or .blocked_on == ""))
              | "\(.id) is blocked with nothing named"),
    (.tasks[] | select(.parent == .id) | "\(.id) is its own parent")
' TODO.json)"
[ -z "$findings" ] && echo 'TODO.json is sound' || printf '%s\n' "$findings"
```

`jq -e` is wrong here: it exits 4 on empty output, which is the sound case, so a
clean file would look like a failure. Test the output instead.

## When not to use this

A defect belongs in the `bug-report` skill, which asks for a reproduction and a
verification this schema has no place for. A checklist inside one task, finished
before the conversation ends, belongs in the conversation. Work needing ordered
goals, verification instructions and progress tracking belongs in a planning
skill's documents.
