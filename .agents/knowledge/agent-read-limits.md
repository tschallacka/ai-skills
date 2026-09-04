<!-- MODE: DEV -->
# How much of a file an agent actually reads

Measured 2026-09-03. **Claude Code truncates a file read silently.** The other
two harnesses either announce it or do not truncate at all.

| harness | version / model | boundary | announced? |
|---|---|---|---|
| Claude Code | 2.1.259, sonnet + opus | **~25,000 tokens** | **no — no notice anywhere** |
| opencode | 1.18.27, big-pickle + openrouter sonnet 4.5 | 50 KB on an `-f` attachment | yes, in-band |
| codex | 0.153.0, gpt-5.6-luna | none found | — |

**There is no line-count boundary anywhere.** Every real limit is a token or
byte budget. The belief that agents "read the first ~300 lines" is false, and
it is false about codex in particular, which read 2,600 of 2,600 lines with all
twelve markers present in both replicates.

Claude Code's `Read` has three behaviours, and only the third is dangerous:

1. over 256 KB — refuses loudly;
2. an explicit range over budget — refuses loudly;
3. **under 256 KB with no range — returns a prefix and stops, with no notice.**

The third is the one a skill hits, because an agent told to read a skill reads
it with no range.

## What it means here

A shipped document over roughly 25,000 tokens is **partly read, silently**, on
the harness we use most. At the time of measuring, `planning/SKILL.md` was
1,502 lines / 87.8 KB — the only surveyed file over either cap — and 2 of 3
real runs received 977 lines, losing about 35% of the file.

Two consequences worth holding on to:

- **An agent cannot tell you it was truncated, and may invent a reason.** One
  asked why content was missing replied that it had "declined to load the rest,
  since it's repetitive filler". The transcript showed a single unbounded
  `Read` had handed it 1,078 lines. It rationalised a truncation it could not
  see, confidently. So "the agent said it loaded the file" is not evidence.
- **A split is only safe if the load is verified.** Fault-injecting one
  oversized part, a self-verifying layout reported `part-3.md MISSING` in 3 of
  3 runs, while the same fault under a plain imperative layout produced "All 5
  part files loaded … Ready" with the truncation buried in a parenthetical. The
  verification token has to sit on the part's **last** line; at the top it
  proves nothing.

## How it was measured

Generator, drivers and raw results are in
[`../skill-loading-experiment/`](../skill-loading-experiment/), and the full
write-up with per-run numbers is
[`../skill-loading-findings.md`](../skill-loading-findings.md).

The controls are the part worth copying:

- **Random 12-hex tokens at known lines**, so a correct answer cannot come from
  the model's prior.
- **The file moved away before questioning**, so an answer cannot come from
  re-reading. That control earned its keep: opencode reached for the file on
  disk and the move caught it.
- **Bisection by line *width*, not line count.** The same 2,600 lines arrived
  whole at 20 characters wide and were cut at line 1,078 at 65 characters wide.
  That is what proved the boundary is not lines.

## Two instruments that lied

Recorded because both are easy to repeat:

- A **notice detector matching a bare `truncat`** fired on `planning/SKILL.md`'s
  own prose, which discusses truncation. A detector has to match the harness's
  actual notice text, not the topic.
- A **7.4 GB copy of `opencode.db` into `/tmp`** was the source of a memory
  spike, because `/tmp` is tmpfs and therefore RAM. Inspect that database in
  place, read-only. See the `/tmp is tmpfs` section in the global instructions.

## Re-checking it

These are version-pinned claims, not laws. Re-measure when a harness updates:
`probe-claude-cap.sh` is the cheap single-phase probe, `reps.sh` drives
replicates, and `summarise-read-calls.py` reduces a transcript to what was
actually returned.
