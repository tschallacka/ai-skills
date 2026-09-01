---
name: bug-report
description: Use when a defect is found that will not be fixed in the same breath, so it is recorded with its reproduction, the measurement that proves it real, its mechanism, and later its fix and verification, in one JSON file read with rjq. Do not use for work that is merely queued (use the todo skill), for a design preference, or for a defect being fixed immediately in the current change.
---
<!-- MODE: PROD -->

# Bug report

A register of defects, held in `BUGS.json` and read with `rjq`. One entry per
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
| `id` | short stable handle, referenced from commits and conversations. The renders sort it through `idkey`, which compares the number numerically, so `B10` follows `B9`; without that def `sort_by` compares strings and item 10 lands between 1 and 2 |
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
recorded="$(rjq -r '.skill_version // "unrecorded"' BUGS.json)"
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
rjq --arg v "$installed" '.skill_version = $v' BUGS.json > BUGS.json.tmp && mv BUGS.json.tmp BUGS.json
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

`reproduce`, `observed`, `expected`, `severity` and `priority` are required from
the moment an entry exists. `mechanism`, `fix` and `verification` fill in as it progresses.

## Recording one

```sh
bugs add --title "A colonless heading silently ignores a rename" \
         --reproduce 'printf "# NoColon\n" > d.md; plan_replace_title d.md New; head -1 d.md' \
         --observed "# NoColon, and exit 0." \
         --expected "Either the heading is rewritten, or the call refuses." \
         --severity minor --priority normal --status confirmed \
         --mechanism "It rewrites with sub(/:.*/, ...), so a heading with no colon matches nothing." \
         --surfaces planning/scripts/lib/document/plan_replace_title.sh \
         --found-by "unit test written against the function alone"
```

It prints the id it allocated. `--title`, `--reproduce`, `--observed` and
`--expected` are required and the command refuses without them: a defect nobody
can reproduce is a rumour, and one with no stated expectation is an opinion.

`bugs` is a single prebuilt binary that ships with this skill. It needs no shell
and no other tool — not `rjq`, not `date` — so it behaves the same under bash,
zsh or anything else, and the register can be written on a machine that has
none of them.

The vocabulary is fixed and the binary will not accept anything outside it:

| Field | Accepted |
|---|---|
| `--severity` | `blocking`, `major`, `minor`, `cosmetic` |
| `--priority` | `urgent`, `high`, `normal`, `low`, `someday` |
| `--status` | `reported`, `confirmed`, `fixed`, `not-a-defect`, `wont-fix`, `obsolete` |

An out-of-vocabulary value is refused when the register is *read*, not by a
separate check that a writer could skip. That is deliberate: a register once
reached a merge carrying `"severity": "critical"` because validation was its own
step and the writing path had gone around it.

## Closing one

A fix and its verification land together. A `fixed` entry with no `verification`
is a claim that the defect is gone, so the command refuses it.

```sh
bugs update B12 --status fixed \
    --fix "a1b2c3d — refuses a heading it cannot rewrite" \
    --verification "Reproduction now exits 65; mutation removing the guard fails the test"
```

Dismissing one needs its reasoning instead:

```sh
bugs update B12 --status not-a-defect --reason "documented behaviour; the caller was wrong"
```

**`not-a-defect` is a real outcome, and worth as much as a fix.** The reason is
appended to `notes`, so the next person who trips over the same behaviour finds
out it was investigated.

`updated_at` is set on every write, including a priority change — a timestamp
maintained only sometimes is worse than none, because a reader cannot tell a
quiet entry from a stale field.

A refused update changes nothing. The register is left exactly as it was, rather
than written and then reported as broken.

## Reading it

These commands print what the user reads. Pass the output through unchanged.

The register, worst first, children under their parent:

```sh
bugs tree
```

```
⛔ B12   [high/minor]       A colonless heading silently ignores a rename
  ✅ B12a  [normal/minor]     The same shape in the goal writer
✔️ B3    [someday/minor]    --in review uses a retired vocabulary
```

Priority first, then severity, then id. The pairing is deliberate: a reader needs
both to judge an entry, and seeing them together is what stops a blocking defect
nobody can reach outranking a cosmetic one on every screen.

What is open, which is the work queue:

```sh
bugs report
```

One entry in full, as stored:

```sh
bugs show B12
```

A filtered table, one tab-separated row per entry, for feeding to something else:

```sh
bugs list --status confirmed
bugs list --severity blocking
bugs list --surface planning/scripts     # matches within the surfaces list
bugs list --since 2026-08-01T00:00:00Z
bugs count --status reported
bugs next-id
```

An absent filter matches everything rather than matching the empty string, so
`list` with no flags is the whole register and not an empty one.

## Keeping the file in priority order

Nothing to do: every write re-sorts. The order is priority, then severity, then
the numeric part of the id, so `B10` follows `B9` rather than preceding it.

The stored order **is** the reading order. No command re-sorts on output, so a
report never imposes a second opinion on urgency over the one the file records.

## Rules that keep it honest

**One defect per entry.** If an entry needs the word "and" to state what is
wrong, it is two entries. A bundled report cannot be closed: fixing one half
leaves the id open with no way to say which half, and the verification then covers
less than the title claims. Split it, and have the halves name each other in
`notes` — "B47 is the rung answering nothing; B48 is CI being unable to see it;
fixing either leaves the other open". Use `parent` only when one genuinely
contains the other, not for two peers.

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
bugs check
```

Prints `<n> entries, sound`, or every rule the file breaks — and exits non-zero
in that case, so it fits a hook or a CI job.

A finding here means more than "the file is malformed". Every writer refuses
what `check` reports, so an entry that breaks a rule **did not come from the
tools**: the file was edited by hand, or by something imitating their output.
That is worth knowing, because it is how a register once reached a merge with
three duplicated entries and a severity outside the vocabulary.

Most of the rules are not checked at all any more — they cannot be broken. The
status, severity and priority vocabularies are types, so an out-of-vocabulary
value fails when the file is *read*, naming the field and the line:

```
unknown variant `critical`, expected one of `blocking`, `major`, `minor`, `cosmetic` at line 10 column 28
```

What `check` still reports is the set of rules that need more than one field to
decide: unique ids, a `parent` that resolves, a reproduction present, and the
evidence a closure owes.

### A register written by an older version

`bugs` refuses to read one in place and says so. Convert it:

```sh
bugs migrate
```

That copies the file to a versioned `BUGS.<version>.back.json` **before parsing
anything**, carries the entries that still fit the current shape and are still
open, leaves the closed ones in the backup, and reports anything it could not
convert with the commands to move it by hand.

There is no compatibility layer and there will not be one. An entry the
converter does not understand is handed back to you — with the backup to read it
from and `bugs add` to re-file it — rather than guessed at. Nothing is lost by
leaving one unconverted: the backup keeps it.

### A merge conflict in the register

Two branches that both file a defect both take the same next id, so the conflict
is semantic: one id, two unrelated defects, and neither side wrong.

```sh
bugs resolve                                  # what has to be decided
bugs resolve theirs:B97:B125 > preview.json   # decide it; nothing is written
CONFIRM=<the token it printed> bugs resolve theirs:B97:B125
```

It reads both clean sides out of the git index, so it works mid-conflict with
nothing checked out. It follows a rename into `parent`, reports ids left in prose
rather than rewriting them, refuses a register whose own side is already unsound,
and writes nothing until the token it printed comes back.

## When not to use this

Work that is queued rather than broken — a refactor, a migration, a decision —
belongs in the `todo` skill. A defect you are fixing in the current change needs
a test, not a register entry. And a design disagreement is not a defect: record
it as a decision where the decision lives.
