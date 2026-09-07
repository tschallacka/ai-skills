<!-- MODE: DEV -->
# Debugging a test

Every helper here lives in `planning/tests/lib-test.sh`, which every suite
sources. Nothing below needs a change to a test file to start working, and
nothing below is active unless it is asked for.

Two separate jobs, deliberately not one switch:

- **Evidence** is for CI. A red leg has to be diagnosable from its log alone.
- **Breakpoints** are for a local run, and are inert anywhere else.

## Evidence: what a failing test leaves behind

**Automatic. No calls, no flags.** When a test exits non-zero its temp root is
printed to stderr, and `run-tests.sh` already prints a failing test's output in
full.

Retention differs by where it ran, which is the whole point:

| | failed test's root |
|---|---|
| CI | **kept** — there is no re-run and no machine to come back to |
| local | **cleaned** — re-run it, or keep it with the flag below |

### Why this exists

`lib-test.sh` used to `rm -rf` the root on *any* exit, failure included. The
reasoning was sound in isolation — a test that leaves its root behind turns a
debugging session into a disk-space problem — but it destroyed the evidence
before anything could print it.

Measured: `test-chat-broadcast-stall` failing on the macOS bash 3.2 leg
reported

```
FAIL: the subscriber never received a message, so it was not a connection the server was servicing:
```

and nothing else. The message interpolates `sub.out`, which is empty in exactly
that failure mode. `server.err`, `server.out` and the send's own output were
all written and then deleted here — so a **required**, reproducibly red check
carried no diagnostic at all, on a platform no maintainer has locally.

A test's own failure message names one or two files at most, and the
interesting one is routinely a file nobody thought to interpolate. That is why
the dump is not selective.

### Bounds

A benchmark root reaches gigabytes, so the dump caps rather than emitting a
tree nobody will read. Files are listed in sorted order; a binary is named but
not printed, so a core dump cannot bury the text files that matter.

| variable | default | meaning |
|---|---|---|
| `AI_SKILLS_TEST_EVIDENCE` | `1` | `0` disables the dump |
| `AI_SKILLS_TEST_EVIDENCE_FILES` | `40` | files dumped before it stops |
| `AI_SKILLS_TEST_EVIDENCE_BYTES` | `16384` | bytes printed per file |
| `AI_SKILLS_TEST_KEEP` | unset | `1` keeps a failed root, `0` cleans it — beats the CI/local default either way |

## Breakpoints

### By line — the usual case

An agent debugging a test has just read the file, so the line it picks is the
line as it stands at that moment. Pass a bare line, a `FILE:LINE`, or a list:

```sh
AI_SKILLS_TEST_BREAK_LINE=120 planning/tests/test-something.sh
AI_SKILLS_TEST_BREAK_LINE=120,204 planning/tests/test-something.sh
AI_SKILLS_TEST_BREAK_LINE=lib-test.sh:466 planning/tests/test-something.sh
```

A bare line means **the running test**. The `FILE:LINE` form stops inside a
library the test calls.

That distinction is load-bearing, not decoration. `set -T` makes the DEBUG trap
fire inside every sourced file, so matching the line alone stopped at line 466
of `lib-test.sh` — inside `t_end` — when asked for line 466 of a test file five
lines long, and labelled the dump with the test's name. A breakpoint that stops
somewhere other than where it was set, and then misreports where it is, is
worse than no breakpoint.

| variable | meaning |
|---|---|
| `AI_SKILLS_TEST_BREAK_LINE` | where to stop: `N`, `FILE:N`, or a comma/space list |
| `AI_SKILLS_TEST_BREAK_EXIT` | `1` stops at the first hit, exits 70, and keeps the root |

`AI_SKILLS_TEST_BREAK_EXIT=1` is the shape to reach for when the plan is to go
and read the tree: stopping to inspect and then deleting what you stopped for
is the one combination nobody wants, so a deliberate stop implies retention.

It is opt-in only because a DEBUG trap fires before every command.

### By name — when the point outlives the edit

```sh
AI_SKILLS_TEST_BREAK="after-install" planning/tests/test-something.sh
AI_SKILLS_TEST_BREAK="all"           planning/tests/test-something.sh
```

Triggered by `t_bp <label>` calls placed in the test. Worth adding when a spot
is interesting more than once, since a label survives edits above it and says
what the spot *is*. For a one-off, a line number is less ceremony.

### Halting is refused where it would hang

`t_bp` and a line breakpoint halt only with a terminal on stdin and only
outside CI. Anywhere else they dump, say they did not stop, and carry on — a
halt on a runner would hang the leg until its timeout and report as something
else entirely.

At a halt: `enter` continues, `s` opens a shell, `q` aborts with 70.

## `t_dump` — state, not commands

```sh
t_dump [label] [file] [line]
```

Prints the variables **the test itself created** — diffed against what existed
when the library loaded, so it is not a dump of the whole environment, which is
a dump nobody reads — and then the file tree.

This is the part `bash -x` cannot give you: xtrace shows the commands, never
the resulting state.

## `t_trace_on` / `t_trace_off` — xtrace, bounded

```sh
t_trace_on
...the suspect part...
t_trace_off
```

`bash -x` over a whole test prints every line of setup, teardown and library
plumbing, and the ten lines actually in question arrive buried in four hundred.
`PS4` carries `file:line`, which bare `set -x` does not, so a trace of a helper
called from three places says which call it is.

## Related

- `docs/TESTING-PROTOCOL.md` — the interactive-shell testing protocol.
- `.agents/MAINTAINER.md` §1.8 — one EXIT trap, process-wide. The evidence dump
  extends the single cleanup rather than installing a second handler.
- `BUGS.json` B268 — a skipped test is still reported as `PASS`, so a green
  summary does not yet prove every test ran. Read a suite's raw output, not
  only its summary, when a leg's result is surprising.
