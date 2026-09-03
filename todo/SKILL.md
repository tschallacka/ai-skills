---
name: todo
description: Use when work is queued that outlives the conversation and needs to survive a restart, a handoff, or a compaction, so tasks and their sub-tasks live in one JSON file read with rjq. Also, when juggling three or more separate work items in sequence, ask the user whether to file them here before starting; never auto-file without asking. Do not use for a short in-chat checklist, for the steps of a task already in progress, for defects (use the bug-report skill), or where a planning skill's plan documents are the right home.
---
<!-- MODE: PROD -->

# Todo

A queue of work that outlives the conversation, held in `TODO.json` and read with
`rjq`. Tasks nest: a large task carries sub-tasks, and a sub-task is a task in its
own right that can be finished while its siblings wait.

**JSON, not prose.** A queue is structured data. Statuses have to be derived
rather than remembered, and a title is free text that will eventually contain the
characters a reader treats as syntax. Query it, never scrape it.

## The file

`TODO.json` at the root of whatever the work belongs to. One file per queue.

```json
{
  "skill": "todo",
  "skill_version": "1.4.2",
  "comment": "What this queue is for, in one line.",
  "tasks": [
    {
      "id": "T1",
      "title": "Split the oversized library",
      "status": "partly",
      "priority": "high",
      "parent": null,
      "detail": "One file holds 47 functions against a 500-line cap. Split by concern, not by line count.",
      "note": "Two of four groups done.",
      "blocked_on": null,
      "refs": ["planning/scripts/plan-document-lib.sh"],
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
      "note": "a284423",
      "blocked_on": null,
      "refs": [],
      "created_at": "2026-08-19T21:04:11Z",
      "updated_at": "2026-08-20T08:31:02Z"
    }
  ]
}
```

| field | meaning |
|---|---|
| `id` | short stable handle. A sub-task conventionally extends its parent's id (`T1` → `T1a`), but nesting comes from `parent`, not the spelling. The renders sort it through `idkey`, which compares the number numerically, so `T10` follows `T9`; without that def `sort_by` compares strings and item 10 lands between 1 and 2 |
| `title` | one line, no trailing full stop, readable on its own |
| `status` | `open`, `partly`, `blocked`, `decided`, `done`, `dropped`, `obsolete` |
| `parent` | the id this nests under, or `null` for a top-level task |
| `detail` | what the task is and why it is worth doing |
| `note` | where it got to: a commit, a decision, a measurement, a reason for dropping it |
| `blocked_on` | who or what must move first. Set it and set `status` to `blocked` |
| `refs` | files or documents whoever picks it up will need |
| `priority` | `urgent`, `high`, `normal`, `low`, `someday`. What order the work happens in, which is not the same as how big it is |
| `created_at` | when it was first recorded, ISO 8601 UTC |
| `updated_at` | when anything on it last changed. Set it on every write, or the field lies |

The two header fields sit outside the list, because they describe the file rather
than any row:

| field | meaning |
|---|---|
| `skill` | which skill's schema this file follows |
| `skill_version` | the `package.json` version of the skill that last wrote it |

`skill_version` is how an upgrade becomes visible. An installed skill carries its
version in `.version` beside `SKILL.md`, so the two can be compared, and a file
written by an older skill can be recognised before its schema is assumed:

```sh
skill_dir="$(dirname "$(command -v true)")"   # replace with the installed skill's directory
installed="$(sed -n 's/^package_version=//p' "$skill_dir/.version" 2>/dev/null)"
recorded="$(rjq -r '.skill_version // "unrecorded"' TODO.json)"
if [ -z "$installed" ]; then
    printf 'cannot read the installed version; leaving %s alone\n' TODO.json
elif [ "$installed" = "$recorded" ]; then
    printf '%s was written by the installed skill (%s)\n' TODO.json "$installed"
else
    printf '%s was written by %s, the installed skill is %s: re-read the schema before writing\n' \
        TODO.json "$recorded" "$installed"
fi
```

Stamp it on every write, next to `updated_at`:

```sh
rjq --arg v "$installed" '.skill_version = $v' TODO.json > TODO.json.tmp && mv TODO.json.tmp TODO.json
```

### Upgrading a file an older skill wrote

**There is no backwards compatibility.** A reader does not accept an older shape;
the agent holding the file brings it up to the installed version, or rewrites it.

Every version ships its own `schema.<version>.json` beside `SKILL.md`. It states
the required and optional fields, the enums, the invariants, and — under
`upgrade_from` — the recipe that turns each named predecessor into it. So when the
recorded version differs from the installed one:

```sh
rjq -r '"required: \(.item.required | join(", "))",
       "enums:    \(.item.fields.status.enum | join("/"))",
       "upgrades from: \(.upgrade_from | keys | join(", ") | if . == "" then "nothing" else . end)"' \
   "$skill_dir/schema.$installed.json"
rjq -r --arg from "$recorded" '.upgrade_from[$from].steps[]? // empty' \
   "$skill_dir/schema.$installed.json"
```

If `upgrade_from` names the recorded version, run its steps: they are written to be
run, not read. If it does not — or no `schema.<recorded>.json` exists to diff
against — rewrite the file from the current schema rather than guessing at the
difference. A register is cheap to rebuild from its own contents and expensive to
half-migrate. Either way the last step is the same: stamp the installed version, so
the next reader sees a current file rather than repeating this.

Only `id`, `title`, `status` and `priority` are required. A task with no priority
cannot be placed in the queue, which is the queue's only job.

**Priority is not size and not severity.** It answers "what next", so it is
allowed to change without the task changing. `someday` is honest and useful: it
says recorded, not forgotten, and not now.

**Flat list, `parent` pointer.** Not nested arrays: appending a sub-task is one
object at the end of the list, with no walk to find its place and no rewrite of a
parent. The tree is reconstructed on read, which is `rjq`'s job.

## Adding a task

Order carries no meaning, so append and stop thinking about it.

```sh
now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
rjq --arg now "$now" '.tasks += [{
      "id": "T9", "title": "Move the artifact map into the coupling registry",
      "status": "open", "priority": "normal", "parent": null,
      "detail": "Some couplings live only in prose, where nothing can read them.",
      "note": null, "blocked_on": null, "refs": ["MAINTAINER.md"],
      "created_at": $now, "updated_at": $now
    }]' TODO.json > TODO.json.tmp && mv TODO.json.tmp TODO.json
```

`date -u +%Y-%m-%dT%H:%M:%SZ` is the one spelling that works on both GNU and BSD
date. `date -Iseconds` is GNU-only and `date -j` is BSD-only.

A sub-task is the same call with `parent` set:

```sh
now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
rjq --arg now "$now" '.tasks += [{
      "id": "T9a", "title": "Move the four generated-artifact rows",
      "status": "open", "priority": "high", "parent": "T9",
      "detail": "The generated files and their builders, which change together.",
      "note": null, "blocked_on": null, "refs": [],
      "created_at": $now, "updated_at": $now
    }]' TODO.json > TODO.json.tmp && mv TODO.json.tmp TODO.json
```

A sub-task carries its own priority. A high-priority sub-task under a low-priority
parent is normal: it is how "one part of this matters now" gets recorded.

**Write to a temp file and rename.** `rjq ... TODO.json > TODO.json` truncates the
file before `rjq` reads it, and the queue is gone.

## Closing a task

Set the status and say where it got to, in one call.

```sh
now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
rjq --arg id T9a --arg now "$now" '
  (.tasks[] | select(.id == $id) | .status) = "done"
  | (.tasks[] | select(.id == $id) | .note) = "a1b2c3d, four rows moved"
  | (.tasks[] | select(.id == $id) | .updated_at) = $now' \
  TODO.json > TODO.json.tmp && mv TODO.json.tmp TODO.json
```

**Every write sets `updated_at`.** A timestamp that is only sometimes maintained
is worse than none, because a reader cannot tell a quiet task from a stale field.
Changing a priority counts as a write:

```sh
now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
rjq --arg id T9 --arg now "$now" '
  (.tasks[] | select(.id == $id) | .priority) = "urgent"
  | (.tasks[] | select(.id == $id) | .updated_at) = $now' \
  TODO.json > TODO.json.tmp && mv TODO.json.tmp TODO.json
```

A parent is not closed by its children. Check, then close it deliberately:

```sh
rjq -r --arg id T9 '[.tasks[] | select(.parent == $id and .status != "done")] as $left
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
rjq -r '
  def glyph: {open:"💤", partly:"⏳", blocked:"⛔", decided:"📌",
              done:"✅", dropped:"✔️", obsolete:"🚫"}[.status] // "❔";
  def rank: {urgent:0, high:1, normal:2, low:3, someday:4}[.priority // ""] // 5;
  def idkey: [(. | scan("[0-9]+") | tonumber)?, .];
  def render($parent; $depth):
    ([.tasks[] | select(.parent == $parent)] | sort_by(rank, (.id | idkey)))[] as $task
    | ("  " * $depth) + ($task | glyph) + " " + $task.id
      + "  [" + ($task.priority // "?") + "]  " + $task.title,
      render($task.id; $depth + 1);
  render(null; 0)' TODO.json
```

```
⛔ T7  [urgent]   Push the branch and read the macOS legs
⏳ T1  [high]     Split the oversized library
  💤 T1b  [high]     Extract the table group
  ✅ T1a  [normal]   Extract the progress group
📌 T8  [normal]   Keep the current gate rather than promoting it
🚫 T9  [someday]  Move the artifact map into the coupling registry
```

Sorted by priority at every level, siblings included, then by id so the order is
stable between runs. A high-priority sub-task rises above its siblings without
moving out from under its parent.

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
rjq -r 'def rank: {urgent:0, high:1, normal:2, low:3, someday:4}[.priority // ""] // 5;
  def idkey: [(. | scan("[0-9]+") | tonumber)?, .];
  [.tasks[] | select(.status == "open" or .status == "partly" or .status == "blocked")]
  | sort_by(rank, (.id | idkey))[]
  | "\(.priority)\t\(.id)\t\(.status)\t\(.title)"' TODO.json \
  | column -t -s "$(printf '\t')"
```

Where things stand, for "what is left":

```sh
rjq -r '.tasks | group_by(.status)[]
       | "\(.[0].status): \(length)  \(map(.id) | join(" "))"' TODO.json
```

Ready to start — open, with nothing open beneath it:

```sh
rjq -r 'def rank: {urgent:0, high:1, normal:2, low:3, someday:4}[.priority // ""] // 5;
  def idkey: [(. | scan("[0-9]+") | tonumber)?, .];
  .tasks as $all
  | [ $all[] | . as $task
      | select(.status == "open")
      | select([$all[] | select(.parent == $task.id and .status != "done")] | length == 0) ]
  | sort_by(rank, (.id | idkey))[]
  | "\(.priority)\t\(.id)\t\(.title)"' TODO.json | column -t -s "$(printf '\t')"
```

Sorted by priority, so the first line is what to pick up.

Two details that are load-bearing. `. as $task`, because inside the inner
`select` a bare `.id` is the inner task's id, so `.parent == .id` is never true
and the filter passes everything. And `.priority // ""` inside the rank map,
because `{...}[null]` is a rjq error rather than a miss: one row without a
priority would otherwise kill the whole render.

One task in full:

```sh
rjq -r --arg id T7 '.tasks[] | select(.id == $id) | "
\(.id)  \(.title)
Status:  \(.status)\(if .blocked_on then "   Waiting on: " + .blocked_on else "" end)

\(.detail // "no detail recorded")

Where it got to: \(.note // "nothing recorded")
\(if (.refs | length) > 0 then "See: " + (.refs | join(", ")) else "" end)"' TODO.json
```

Blocked work and who owes it, which is what a check-in asks for:

```sh
rjq -r '.tasks[] | select(.status == "blocked")
       | "\(.id)  \(.title)\n    waiting on: \(.blocked_on // "unrecorded")"' TODO.json
```

## Keeping the file in priority order

The renders sort, so the file's order never changes an answer. Sorting the file
itself is for the human reading a diff: a new urgent task appearing at the top is
visible, appended at the bottom it is not.

```sh
rjq 'def rank: {urgent:0, high:1, normal:2, low:3, someday:4}[.priority // ""] // 5;
  def idkey: [(. | scan("[0-9]+") | tonumber)?, .];
    .tasks |= sort_by(rank, (.id | idkey))' TODO.json > TODO.json.tmp && mv TODO.json.tmp TODO.json
```

Run it after adding tasks. A pure reordering: only the sequence of the array
changes, so a diff shows movement and nothing else.

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

**Priority is a judgement you are allowed to revise.** Re-prioritising is a
write like any other: set the new value and stamp `updated_at`. A queue whose
priorities never move is a queue nobody is reading.

**Do not delete a task.** Close it as `done`, `dropped` or `obsolete` with a
reason. A deleted task takes its reasoning with it.

## Validating the file

Membership is tested with `index(...)`, not jq's `IN/1`. `rjq` — the runtime
this register mandates, since every writer refuses with 69 without it — does
not implement `IN/1`: it exits 5 having printed nothing, and an empty findings
string is the sound case, so the whole check reported any register sound.

Note the `as $e` binding on each clause. `A | index(B)` evaluates `B` with `A`
as its input, so a bare `index(.status)` looks `.status` up on the *array* and
dies with "cannot index". Binding the entry first is what makes the field
reference resolve against the entry — the same shape `reg_findings` uses.

```sh
findings="$(rjq -r '
  (if (.skill_version // "") == "" then "the file does not record which skill version wrote it" else empty end),
  (if (.skill // "") == "" then "the file does not name its schema" else empty end),
  ([.tasks[].id]) as $ids
  | if ([.tasks[].id] | length) != ([.tasks[].id] | unique | length) then "duplicate ids" else empty end,
    (.tasks[] as $e | select($e.parent != null and (($ids | index($e.parent)) == null))
                    | "\($e.id) names a parent that does not exist: \($e.parent)"),
    (.tasks[] as $e | select((["open","partly","blocked","decided","done","dropped","obsolete"] | index($e.status)) == null)
                    | "\($e.id) has an unknown status: \($e.status // "missing")"),
    (.tasks[] | select(.status == "blocked" and (.blocked_on == null or .blocked_on == ""))
              | "\(.id) is blocked with nothing named"),
    (.tasks[] | select(.parent == .id) | "\(.id) is its own parent")
' TODO.json)" || { printf 'the soundness check could not run\n' >&2; exit 70; }
[ -z "$findings" ] && echo 'TODO.json is sound' || printf '%s\n' "$findings"
```

`rjq -e` is wrong here: it exits 4 on empty output, which is the sound case, so a
clean file would look like a failure. Test the command's own status, as above,
and then the output — empty output only means sound when the command ran.

## When not to use this

A defect belongs in the `bug-report` skill, which asks for a reproduction and a
verification this schema has no place for. A checklist inside one task, finished
before the conversation ends, belongs in the conversation. Work needing ordered
goals, verification instructions and progress tracking belongs in a planning
skill's documents.
