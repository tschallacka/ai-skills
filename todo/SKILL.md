---
name: todo
description: Use when work is queued that outlives the conversation and needs to survive a restart, a handoff, or a compaction, so items and their sub-items live in one JSON file read with jq. Do not use for a short in-chat checklist, for the steps of a single task already in progress, or where the planning skill's plan documents are the right home.
---

# Todo

A queue of work that outlives the conversation, held in one JSON file and read
with `jq`. Items nest: a large item carries sub-items, and a sub-item is a
first-class row that can be finished on its own.

**JSON, not prose.** A queue is structured data: statuses have to be derived
rather than remembered, and a title is free text that will eventually contain the
characters your reader treats as syntax. Query it, never scrape it.

## The file

`TODO.json` at the root of whatever the work belongs to — a repository, a project
directory, a plans root. One file per queue.

```json
{
  "comment": "What this queue is for, in one line.",
  "items": [
    {
      "id": "T1",
      "title": "One test reporter, not eighteen",
      "status": "partly",
      "parent": null,
      "detail": "Tests that define their own exiting reporter hide every finding after the first.",
      "evidence": "Measured by injecting two failures and counting what was reported.",
      "blocked_on": null,
      "refs": ["planning/tests/lib-test.sh"]
    },
    {
      "id": "T1a",
      "title": "Convert the three regression tests to the shared reporter",
      "status": "done",
      "parent": "T1",
      "detail": "Three files, one shared shape, 47 call sites between them.",
      "evidence": "b1ba471; both injected failures reported where one was hidden before.",
      "blocked_on": null,
      "refs": []
    }
  ]
}
```

Every field except `id`, `title` and `status` may be `null` or omitted.

| field | meaning |
|---|---|
| `id` | short stable handle. A sub-item conventionally extends its parent's id (`T1` → `T1a`), but nesting comes from `parent`, not from the spelling |
| `title` | one line, no trailing full stop, readable on its own |
| `status` | `open`, `partly`, `blocked`, `decided`, `done`, `not-a-defect`, `obsolete` |
| `parent` | the id this nests under, or `null` for a top-level item |
| `detail` | what it is and why it matters, in sentences |
| `evidence` | the measurement, commit, or output that settles it. A `done` item without evidence is a claim |
| `blocked_on` | who or what must move first. Set it and set `status` to `blocked` |
| `refs` | files, tests, or documents a reader will need |

**Flat list, `parent` pointer.** Not nested arrays: appending a sub-item is then
one object added at the end, with no walk to find its place and no rewrite of a
parent. The tree is reconstructed on read, which is `jq`'s job.

## Adding an item

Append an object. Nothing depends on the order, so there is no insertion point to
get wrong.

```sh
jq '.items += [{
      "id": "T24", "title": "Rename the stale fixtures", "status": "open",
      "parent": null, "detail": "Three fixtures name a script that moved.",
      "evidence": null, "blocked_on": null, "refs": []
    }]' TODO.json > TODO.json.tmp && mv TODO.json.tmp TODO.json
```

A sub-item is the same call with `parent` set:

```sh
jq '.items += [{
      "id": "T24a", "title": "Rename the reachability fixture", "status": "open",
      "parent": "T24", "detail": "Five steps numbered 01 in one goal.",
      "evidence": null, "blocked_on": null, "refs": []
    }]' TODO.json > TODO.json.tmp && mv TODO.json.tmp TODO.json
```

**Write to a temp file and rename.** `jq ... TODO.json > TODO.json` truncates the
file before `jq` reads it, and the queue is gone.

## Closing an item

Set the status and record what settles it, in the same call. An item closed
without evidence is indistinguishable from one abandoned.

```sh
jq '(.items[] | select(.id == "T24a") | .status) = "done"
  | (.items[] | select(.id == "T24a") | .evidence) = "3fe77b4; the suite is green on both shells"' \
  TODO.json > TODO.json.tmp && mv TODO.json.tmp TODO.json
```

A parent is not closed by its children. Close it deliberately, when nothing is
left under it:

```sh
jq -r --arg id T24 '
  [.items[] | select(.parent == $id and .status != "done")] as $left
  | if ($left | length) == 0 then "T24 has nothing open under it; safe to close"
    else "T24 still has: " + ($left | map(.id) | join(", ")) end' TODO.json
```

## Reading it

**These commands print what the user reads.** Run one and pass its output
through unchanged. Reformatting it in a reply spends tokens rewriting text that
is already in its final shape, and every rewrite is a chance to alter a status
or drop an item.

The whole queue as an indented tree:

```sh
jq -r '
  def glyph: {open:"💤", partly:"⏳", blocked:"⛔", decided:"📌",
              done:"✅", "not-a-defect":"✔️", obsolete:"🚫"}[.status] // "❔";
  def render($parent; $depth):
    (.items[] | select(.parent == $parent)) as $item
    | ("  " * $depth) + ($item | glyph) + " " + $item.id + "  " + $item.title,
      render($item.id; $depth + 1);
  render(null; 0)' TODO.json
```

```
⏳ T1  One test reporter, not eighteen
  ✅ T1a  Convert the three regression tests
  💤 T1b  Convert flag-form-equivalence and portable-helpers
⛔ T7  Push the branch and read the macOS legs
🚫 T9  Move the artifact map into coupling.tsv
⬜ T24  Rename the stale fixtures
```

| glyph | status | means |
|---|---|---|
| 💤 | `open` | queued, nothing started |
| ⏳ | `partly` | started, or some sub-items closed |
| ⛔ | `blocked` | waiting on someone else; `blocked_on` says who |
| 📌 | `decided` | a choice was made, not work to do; `evidence` says why |
| ✅ | `done` | finished, with evidence |
| ✔️ | `not-a-defect` | looked at, nothing to fix; the grey tick is deliberate, it is not an achievement |
| 🚫 | `obsolete` | overtaken by events: disabled, superseded, or deliberately ignored |
| ❔ | — | a status this table does not know, which is itself the finding |

What is actually open, parents and children together, one line each:

```sh
jq -r '.items[] | select(.status == "open" or .status == "partly")
       | "\(.id)\t\(.title)"' TODO.json | column -t -s "$(printf '\t')"
```

A status roll-up, for "where are we":

```sh
jq -r '.items | group_by(.status)[]
       | "\(.[0].status): \(length)  \(map(.id) | join(" "))"' TODO.json
```

One item in full, when the user asks about it:

```sh
jq -r --arg id T7 '.items[] | select(.id == $id)
  | "\(.id)  \(.title)\n\nStatus:   \(.status)\nBlocked:  \(.blocked_on // "nothing")\n\n\(.detail)\n\nEvidence: \(.evidence // "none recorded")"' TODO.json
```

Only what is blocked on someone else, which is what a status meeting wants:

```sh
jq -r '.items[] | select(.status == "blocked")
       | "\(.id)  \(.title)\n    waiting on: \(.blocked_on)"' TODO.json
```

Ready to start — open, and no open children, so nothing has to happen first:

```sh
jq -r '.items as $all | $all[] | . as $item
  | select(.status == "open")
  | select([$all[] | select(.parent == $item.id and .status != "done")] | length == 0)
  | "\(.id)  \(.title)"' TODO.json
```

`. as $item` is load-bearing. Inside the inner `select`, `.id` is the *inner*
item's id, so `.parent == .id` compares each item's parent to its own id, is
never true, and the filter silently passes everything. Plant a parent with an open
child to see the difference.

## Rules that keep it usable

**Every `done` carries its evidence.** A commit, a measurement, a command
output. Without it, nobody can tell a finished item from a forgotten one, and
re-deriving that is more expensive than recording it was.

**Re-read before reporting status.** The file is the state. A summary in the
conversation is a copy, and it is stale the moment anything lands.

**Split an item when it grows sub-items, do not widen its title.** A title that
needs "and" is usually two items, and a sub-item can be finished and closed while
its siblings wait.

**Record the decision, not just the task.** When an item is a choice rather than
work, set `status` to `decided` and put the reasoning in `evidence`. The next
reader needs to know why, or the choice gets relitigated.

**Do not delete an item.** Set `status` to `done`, `not-a-defect` or `obsolete`
and say why
in `evidence`. A deleted item takes its reasoning with it, and "we looked at this
and it was not real" is worth as much as a fix.

## Validating the file

Structural mistakes are cheap to catch and expensive to trip over.

`jq -e` is wrong here: it exits 4 when the output is empty, which is the sound
case, so a clean file would look like a failure. Test the output instead.

```sh
findings="$(jq -r '
  ([.items[].id] | length) as $count
  | ([.items[].id] | unique | length) as $unique
  | ([.items[].id]) as $ids
  | if $count != $unique then "duplicate ids" else empty end,
    (.items[] | select(.parent != null and (.parent | IN($ids[]) | not))
             | "\(.id) names a parent that does not exist: \(.parent)"),
    (.items[] | select(.status | IN("open","partly","blocked","decided","done","not-a-defect","obsolete") | not)
             | "\(.id) has an unknown status: \(.status)"),
    (.items[] | select(.status == "done" and (.evidence == null or .evidence == ""))
             | "\(.id) is done with no evidence"),
    (.items[] | select(.status == "blocked" and (.blocked_on == null or .blocked_on == ""))
             | "\(.id) is blocked with nothing named")
' TODO.json)"
[ -z "$findings" ] && echo 'TODO.json is sound' || printf '%s\n' "$findings"
```

Run it after any edit: a `jq` write that produced valid JSON can still produce a
queue that reads wrong -- a parent that no longer exists, or a `done` item whose
evidence never got filled in.

## When not to use this

A checklist inside one task, tracked in the conversation and finished before it
ends. The planning skill's plan documents, when the work needs ordered goals,
verification and progress. A bug tracker your team already reads — this file is
for work that would otherwise live only in someone's head or a chat scrollback.
