---
name: bug-report
description: Use when a defect is found that will not be fixed in the same breath, so it is recorded with its reproduction, the measurement that proves it real, its mechanism, and later its fix and verification, in one JSON file read with jq. Do not use for work that is merely queued (use the todo skill), for a design preference, or for a defect being fixed immediately in the current change.
---
<!-- MODE: PROD -->

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
  "skill": "bug-report",
  "skill_version": "1.4.2",
  "comment": "Defects found in <subject>, with reproduction and verification.",
  "bugs": [
    {
      "id": "B7",
      "title": "The first finding of every review cycle becomes the table header",
      "status": "fixed",
      "severity": "blocking",
      "priority": "urgent",
      "parent": null,
      "reproduce": "printf 'AR-41,First,Do it,resolved,W01\\nAR-42,Second,Do it,resolved,W02\\n' | update-adversarial-review.sh <plan>",
      "observed": "AR-41 rendered as the header row with |---| beneath it; AR-42 the only data row.",
      "expected": "A header row of column names, then both findings as data rows.",
      "mechanism": "plan_render_csv_table emits the delimiter after row 1, which is correct for its --table-paragraph caller. This caller passed findings straight in, so row 1 was a finding.",
      "surfaces": ["planning/scripts/update-adversarial-review.sh"],
      "fix": "ea29673 — the caller prepends the header row; the library contract is unchanged.",
      "verification": "Two findings in, both present as data rows, header restored. Mutation-tested by removing the prepend.",
      "found_by": "reviewer subagent, cycle 12",
      "notes": "Five related tests passed while this was broken: none asserted the header exists.",
      "created_at": "2026-08-20T07:41:03Z",
      "updated_at": "2026-08-20T08:02:55Z"
    }
  ]
}
```

| field | why it is there |
|---|---|
| `id` | short stable handle, referenced from commits and conversations |
| `title` | the defect in one line, stated as what goes wrong |
| `status` | `reported`, `confirmed`, `fixed`, `not-a-defect`, `wont-fix`, `obsolete` |
| `severity` | `blocking`, `major`, `minor`, `cosmetic`. How bad it is when it happens |
| `priority` | `urgent`, `high`, `normal`, `low`, `someday`. When it gets fixed, which is a separate judgement: a cosmetic defect on the page every user sees can outrank a blocking one behind a flag nobody has enabled |
| `created_at` | when it was first recorded, ISO 8601 UTC |
| `updated_at` | when anything on it last changed. Set it on every write, or the field lies |
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
recorded="$(jq -r '.skill_version // "unrecorded"' BUGS.json)"
if [ -z "$installed" ]; then
    printf 'cannot read the installed version; leaving %s alone\n' BUGS.json
elif [ "$installed" = "$recorded" ]; then
    printf '%s was written by the installed skill (%s)\n' BUGS.json "$installed"
else
    printf '%s was written by %s, the installed skill is %s: re-read the schema before writing\n' \
        BUGS.json "$recorded" "$installed"
fi
```

Stamp it on every write, next to `updated_at`:

```sh
jq --arg v "$installed" '.skill_version = $v' BUGS.json > BUGS.json.tmp && mv BUGS.json.tmp BUGS.json
```

### Upgrading a file an older skill wrote

**There is no backwards compatibility.** A reader does not accept an older shape;
the agent holding the file brings it up to the installed version, or rewrites it.

Every version ships its own `schema.<version>.json` beside `SKILL.md`. It states
the required and optional fields, the enums, the invariants, and — under
`upgrade_from` — the recipe that turns each named predecessor into it. So when the
recorded version differs from the installed one:

```sh
jq -r '"required: \(.item.required | join(", "))",
       "enums:    \(.item.fields.status.enum | join("/"))",
       "upgrades from: \(.upgrade_from | keys | join(", ") | if . == "" then "nothing" else . end)"' \
   "$skill_dir/schema.$installed.json"
jq -r --arg from "$recorded" '.upgrade_from[$from].steps[]? // empty' \
   "$skill_dir/schema.$installed.json"
```

If `upgrade_from` names the recorded version, run its steps: they are written to be
run, not read. If it does not — or no `schema.<recorded>.json` exists to diff
against — rewrite the file from the current schema rather than guessing at the
difference. A register is cheap to rebuild from its own contents and expensive to
half-migrate. Either way the last step is the same: stamp the installed version, so
the next reader sees a current file rather than repeating this.

`reproduce`, `observed`, `expected`, `severity` and `priority` are required from
the moment an entry exists. `mechanism`, `fix` and `verification` fill in as it progresses.

## Recording one

```sh
now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
jq --arg now "$now" '.bugs += [{
      "id": "B12", "title": "A colonless heading silently ignores a rename",
      "status": "confirmed", "severity": "minor", "priority": "normal", "parent": null,
      "reproduce": "printf \\"# NoColon\\\\n\\" > d.md; plan_replace_title d.md New; head -1 d.md",
      "observed": "# NoColon, and exit 0.",
      "expected": "Either the heading is rewritten, or the call refuses.",
      "mechanism": "It rewrites with sub(/:.*/, ...), so a heading with no colon matches nothing.",
      "surfaces": ["planning/scripts/lib/document/plan_replace_title.sh"],
      "fix": null, "verification": null,
      "found_by": "unit test written against the function alone",
      "notes": null,
      "created_at": $now, "updated_at": $now
    }]' BUGS.json > BUGS.json.tmp && mv BUGS.json.tmp BUGS.json
```

Write to a temp file and rename. `jq ... BUGS.json > BUGS.json` truncates the
file before `jq` reads it. `date -u +%Y-%m-%dT%H:%M:%SZ` is the one spelling that
works on both GNU and BSD date.

## Closing one

A fix and its verification land together. A `fixed` entry with no `verification`
is a claim that the defect is gone.

```sh
now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
jq --arg id B12 --arg now "$now" '
    (.bugs[] | select(.id == $id) | .status) = "fixed"
  | (.bugs[] | select(.id == $id) | .fix) = "a1b2c3d — refuses a heading it cannot rewrite"
  | (.bugs[] | select(.id == $id) | .verification) = "Reproduction now exits 65; mutation removing the guard fails the test"
  | (.bugs[] | select(.id == $id) | .updated_at) = $now' \
  BUGS.json > BUGS.json.tmp && mv BUGS.json.tmp BUGS.json
```

**Every write sets `updated_at`**, including a priority change. A timestamp
maintained only sometimes is worse than none: a reader cannot tell a quiet entry
from a stale field.

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
  def prank: {urgent:0, high:1, normal:2, low:3, someday:4}[.priority // ""] // 5;
  def srank: {blocking:0, major:1, minor:2, cosmetic:3}[.severity // ""] // 4;
  def render($parent; $depth):
    ([.bugs[] | select(.parent == $parent)] | sort_by(prank, srank, .id))[] as $bug
    | ("  " * $depth) + ($bug | glyph) + " " + $bug.id
      + "  [" + ($bug.priority // "?") + "/" + ($bug.severity // "?") + "]  " + $bug.title,
      render($bug.id; $depth + 1);
  render(null; 0)' BUGS.json
```

```
✅ B7    [urgent/blocking]  The first finding of every review cycle becomes the table header
⛔ B12   [high/minor]       A colonless heading silently ignores a rename
  ✅ B12a  [normal/minor]     The same shape in the goal writer
✔️ B3    [someday/minor]    --in review uses a retired vocabulary
```

Priority first, then severity, then id. The pairing is deliberate: a reader needs
both to judge an entry, and seeing them together is what stops a blocking defect
nobody can reach outranking a cosmetic one on every screen.

What is open and confirmed, which is the work queue:

```sh
jq -r 'def prank: {urgent:0, high:1, normal:2, low:3, someday:4}[.priority // ""] // 5;
  def srank: {blocking:0, major:1, minor:2, cosmetic:3}[.severity // ""] // 4;
  [.bugs[] | select(.status == "reported" or .status == "confirmed")]
  | sort_by(prank, srank, .id)[]
  | "\(.priority)\t\(.severity)\t\(.id)\t\(.title)"' BUGS.json \
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
       | group_by(.priority)[] | "\(.[0].priority): \(length)  \(map(.id) | join(" "))"' BUGS.json
```

## Keeping the file in priority order

The renders sort, so the file's order never changes an answer. Sorting the file
itself is for the human reading a diff: a new urgent entry appearing at the top is
visible, and appended at the bottom it is not.

```sh
jq 'def prank: {urgent:0, high:1, normal:2, low:3, someday:4}[.priority // ""] // 5;
    def srank: {blocking:0, major:1, minor:2, cosmetic:3}[.severity // ""] // 4;
    .bugs |= sort_by(prank, srank, .id)' \
  BUGS.json > BUGS.json.tmp && mv BUGS.json.tmp BUGS.json
```

Run it after adding entries. It is a pure reordering: nothing but the sequence of
the array changes, so a diff shows only movement.

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
  (if (.skill_version // "") == "" then "the file does not record which skill version wrote it" else empty end),
  (if (.skill // "") == "" then "the file does not name its schema" else empty end),
  ([.bugs[].id]) as $ids
  | if ([.bugs[].id] | length) != ([.bugs[].id] | unique | length) then "duplicate ids" else empty end,
    (.bugs[] | select(.parent != null and (.parent | IN($ids[]) | not))
             | "\(.id) names a parent that does not exist: \(.parent)"),
    (.bugs[] | select(.status | IN("reported","confirmed","fixed","not-a-defect","wont-fix","obsolete") | not)
             | "\(.id) has an unknown status: \(.status)"),
    (.bugs[] | select(.severity | IN("blocking","major","minor","cosmetic") | not)
             | "\(.id) has an unknown severity: \(.severity)"),
    (.bugs[] | select(.priority | IN("urgent","high","normal","low","someday") | not)
             | "\(.id) has an unknown priority: \(.priority)"),
    (.bugs[] | select(.created_at == null or .updated_at == null)
             | "\(.id) is missing a timestamp"),
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
