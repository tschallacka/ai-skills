---
name: rjq
description: Use when a JSON file has to be parsed, searched or filtered and `jq` is not installed or not on PATH -- it runs the shipped `rjq` binary, a jq-compatible query tool with no other dependencies. Prefer `jq` when it is present. Do not use to write JSON that another tool owns (use that tool), or for free-text search (use grep).
---
<!-- MODE: PROD -->

# rjq

`rjq` is a jq-compatible query tool shipped with this skill as one prebuilt
static binary. It reads JSON, applies a filter, and prints the result. It needs
no shell, no other tool, and no package manager, so it works on a machine where
`jq` was never installed.

## Which one to run

1. If `jq` is on PATH, use it. It is the reference implementation.
2. Otherwise run the shipped binary by its full path, because the shared bin
   directory is often not on PATH in an agent's shell:

   ```bash
   RJQ="${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills/bin/rjq"
   "$RJQ" '.tasks | length' TODO.json
   ```

   On Windows the file is `rjq.exe` in the same directory.

3. If that path does not exist, the skill is not installed for this machine;
   say so rather than guessing at the file.

## Supported

These are verified against the shipped binary. Anything outside this list is
not promised, so check it on a sample before relying on it.

| You want | Filter |
|---|---|
| one field | `.name` |
| length of an array | `.tasks \| length` |
| every element's field | `.tasks[].id` |
| filter rows | `.tasks[] \| select(.status == "open") \| .id` |
| sum a field | `[.tasks[].n] \| add` |
| transform each element | `.tasks \| map(.id)` |
| object keys | `keys` |
| one element | `.tasks[0]` |
| match a regex | `.tasks[] \| select(.id \| test("T2"))` |
| pass a value in safely | `--arg id T2 '.tasks[] \| select(.id == $id)'` |

Options: `-r` prints strings raw (no quotes), `-c` prints compact output, `-e`
exits 4 when the query outputs nothing (0 otherwise), `--arg NAME VALUE` binds a
string variable, `--argjson NAME JSON` binds a JSON value, `--slurpfile NAME FILE`
binds a file's JSON values as `$NAME`.

## Rules

- Pass the filter as one quoted argument. Interpolate values with `--arg`, never
  by pasting them into the filter text: a value with a quote or a backtick in it
  changes the filter.
- Read with `rjq`; do not hand-edit JSON that a register or a skill owns. Those
  tools keep the file sound and stamp its version.
- A query that finds nothing prints nothing or `null`. Check the output rather
  than assuming there was a match.
