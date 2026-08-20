---
name: bug-report
description: Use when a defect is found that will not be fixed in the same breath, so it is recorded with its reproduction, the measurement that proves it real, its mechanism, and later its fix and verification, in one JSON file read with jq. Do not use for work that is merely queued (use the todo skill), for a design preference, or for a defect being fixed immediately in the current change.
---

# Bug report

A register of defects, held in `BUGS.json` and read with `jq`. One entry per
defect, carrying the four things that decide whether it can be acted on: how to
reproduce it, the measurement proving it real, the mechanism, and what closed it.

**A report without a reproduction is a rumour.** The most expensive thing in a
defect register is an entry nobody can confirm: the next reader either re-derives
it from scratch or quietly ignores it. Record the command and the output, not the
impression.

## The file

`BUGS.json` at the root of whatever holds the defect.

```json
{
  "comment": "Defects found in <subject>, with reproduction and verification.",
  "bugs": [
    {
      "id": "B7",
      "title": "The first finding of every review cycle becomes the table header",
      "status": "fixed",
      "severity": "blocking",
      "parent": null,
      "reproduce": "printf 'AR-41,First,Do it,resolved,W01\\nAR-42,Second,Do it,resolved,W02\\n' | update-adversarial-review.sh <plan>",
      "observed": "AR-41 rendered as the header row with |---| beneath it; AR-42 the only data row.",
      "expected": "A header row of column names, then both findings as data rows.",
      "mechanism": "plan_render_csv_table emits the delimiter after row 1, which is correct for its --table-paragraph caller. This caller passed findings straight in, so row 1 was a finding.",
      "surfaces": ["planning/scripts/update-adversarial-review.sh"],
      "fix": "ea29673 — the caller prepends the header row; the library contract is unchanged.",
      "verification": "Two findings in, both present as data rows, header restored. Mutation-tested by removing the prepend.",
      "found_by": "reviewer subagent, cycle 12",
      "notes": "Five related tests passed while this was broken: none asserted the header exists."
    }
  ]
}
```

| field | why it is there |
|---|---|
| `id` | short stable handle, referenced from commits and conversations |
| `title` | the defect in one line, stated as what goes wrong |
| `status` | `reported`, `confirmed`, `fixed`, `not-a-defect`, `wont-fix`, `obsolete` |
| `severity` | `blocking`, `major`, `minor`, `cosmetic` |
| `parent` | a defect this is a facet of, or `null`. Sub-entries let one root cause carry several observed failures |
| `reproduce` | the command or steps. Runnable, not described |
| `observed` | what actually happened, quoted from the output |
| `expected` | what should have happened, so a reader can judge without guessing |
| `mechanism` | the root cause once known. Absent until `confirmed` |
| `surfaces` | every file that has to change. A defect in one behaviour often lives in several |
| `fix` | the commit and, in a clause, what it changed |
| `verification` | how the fix was proven, including the mutation that fails without it |
| `found_by` | who or what found it: a test, a reviewer, a user report |
| `notes` | anything a future reader needs, especially why it went unnoticed |

`reproduce`, `observed` and `expected` are required from the moment an entry
exists. `mechanism`, `fix` and `verification` fill in as it progresses.

## Recording one

```sh
jq '.bugs += [{
      "id": "B12", "title": "A colonless heading silently ignores a rename",
      "status": "confirmed", "severity": "minor", "parent": null,
      "reproduce": "printf \\"# NoColon\\\\n\\" > d.md; plan_replace_title d.md New; head -1 d.md",
      "observed": "# NoColon, and exit 0.",
      "expected": "Either the heading is rewritten, or the call refuses.",
      "mechanism": "It rewrites with sub(/:.*/, ...), so a heading with no colon matches nothing.",
      "surfaces": ["planning/scripts/lib/document/plan_replace_title.sh"],
      "fix": null, "verification": null,
      "found_by": "unit test written against the function alone",
      "notes": null
    }]' BUGS.json > BUGS.json.tmp && mv BUGS.json.tmp BUGS.json
```

Write to a temp file and rename. `jq ... BUGS.json > BUGS.json` truncates the
file before `jq` reads it.

## Closing one

A fix and its verification land together. A `fixed` entry with no `verification`
is a claim that the defect is gone.

```sh
jq '(.bugs[] | select(.id == "B12") | .status) = "fixed"
  | (.bugs[] | select(.id == "B12") | .fix) = "a1b2c3d — refuses a heading it cannot rewrite"
  | (.bugs[] | select(.id == "B12") | .verification) = "Reproduction now exits 65; mutation removing the guard fails the test"' \
  BUGS.json > BUGS.json.tmp && mv BUGS.json.tmp BUGS.json
```

**`not-a-defect` is a real outcome, and worth as much as a fix.** Set it with the
reasoning in `verification`, so the next person who trips over the same behaviour
finds out it was investigated.

## Reading it

These commands print what the user reads. Pass the output through unchanged.

The register, worst first, grouped by status:

```sh
jq -r '
  def glyph: {reported:"💤", confirmed:"⛔", fixed:"✅",
              "not-a-defect":"✔️", "wont-fix":"🚫", obsolete:"🚫"}[.status] // "❔";
  def rank: {blocking:0, major:1, minor:2, cosmetic:3}[.severity] // 4;
  def render($parent; $depth):
    ([.bugs[] | select(.parent == $parent)] | sort_by(rank))[] as $bug
    | ("  " * $depth) + ($bug | glyph) + " " + $bug.id
      + "  [" + ($bug.severity // "?") + "]  " + $bug.title,
      render($bug.id; $depth + 1);
  render(null; 0)' BUGS.json
```

```
⛔ B12  [blocking]  A colonless heading silently ignores a rename
  ✅ B12a  [minor]  The same shape in the goal writer
✅ B7   [blocking]  The first finding of every review cycle becomes the table header
✔️ B3   [minor]  --in review uses a retired vocabulary
```

What is open and confirmed, which is the work queue:

```sh
jq -r '[.bugs[] | select(.status == "reported" or .status == "confirmed")]
       | sort_by({blocking:0, major:1, minor:2, cosmetic:3}[.severity] // 4)[]
       | "\(.severity | ascii_upcase)\t\(.id)\t\(.title)"' BUGS.json \
  | column -t -s "$(printf '\t')"
```

One entry as a report a person can act on:

```sh
jq -r --arg id B7 '.bugs[] | select(.id == $id) | "
\(.id)  \(.title)
Status:   \(.status)   Severity: \(.severity)   Found by: \(.found_by // "unrecorded")

Reproduce:
  \(.reproduce)

Observed:  \(.observed)
Expected:  \(.expected)
Mechanism: \(.mechanism // "not yet established")
Surfaces:  \(.surfaces | join(", "))

Fix:          \(.fix // "none yet")
Verification: \(.verification // "none yet")
\(if .notes then "\nNotes: " + .notes else "" end)"' BUGS.json
```

Entries nobody can act on, which is the register's own health check:

```sh
jq -r '.bugs[] | select(.reproduce == null or .reproduce == "")
       | "\(.id) has no reproduction"' BUGS.json
```

Fixed without verification, and confirmed without a mechanism:

```sh
jq -r '.bugs[] | select(.status == "fixed" and (.verification == null or .verification == ""))
                 | "\(.id) is fixed with nothing proving it",
       .bugs[] | select(.status == "confirmed" and (.mechanism == null or .mechanism == ""))
                 | "\(.id) is confirmed with no mechanism"' BUGS.json
```

A severity roll-up of what is still open:

```sh
jq -r '[.bugs[] | select(.status == "reported" or .status == "confirmed")]
       | group_by(.severity)[] | "\(.[0].severity): \(length)  \(map(.id) | join(" "))"' BUGS.json
```

## Rules that keep it honest

**Reproduce before recording.** An entry written from an impression sends the
next reader chasing something that may not exist. If it cannot be reproduced, say
so in `observed` and set `status` to `reported` rather than `confirmed`.

**Record the surfaces, plural.** One wrong behaviour usually lives in more than
one file: the writer and the validator, the instruction and the acceptance
criterion. An entry naming one file invites a half-fix.

**Say why it went unnoticed.** In `notes`. "Five related tests passed while this
was broken, none asserted the header exists" is worth more than the fix, because
it names the missing test rather than the missing line.

**A fix names its mutation.** `verification` should say what fails when the fix is
removed. A fix nothing can fail is indistinguishable from a coincidence.

**Do not delete an entry.** Set `not-a-defect`, `wont-fix` or `obsolete` with the
reasoning. A deleted entry takes its investigation with it, and the same
behaviour will be reported again.

## Validating the file

```sh
findings="$(jq -r '
  ([.bugs[].id]) as $ids
  | if ([.bugs[].id] | length) != ([.bugs[].id] | unique | length) then "duplicate ids" else empty end,
    (.bugs[] | select(.parent != null and (.parent | IN($ids[]) | not))
             | "\(.id) names a parent that does not exist: \(.parent)"),
    (.bugs[] | select(.status | IN("reported","confirmed","fixed","not-a-defect","wont-fix","obsolete") | not)
             | "\(.id) has an unknown status: \(.status)"),
    (.bugs[] | select(.severity | IN("blocking","major","minor","cosmetic") | not)
             | "\(.id) has an unknown severity: \(.severity)"),
    (.bugs[] | select(.reproduce == null or .reproduce == "") | "\(.id) has no reproduction"),
    (.bugs[] | select(.status == "fixed" and (.verification == null or .verification == ""))
             | "\(.id) is fixed with no verification")
' BUGS.json)"
[ -z "$findings" ] && echo 'BUGS.json is sound' || printf '%s\n' "$findings"
```

`jq -e` is wrong here: it exits 4 on empty output, which is the sound case.

## When not to use this

Work that is queued rather than broken — a refactor, a migration, a decision —
belongs in the `todo` skill. A defect you are fixing in the current change needs
a test, not a register entry. And a design disagreement is not a defect: record
it as a decision where the decision lives.
