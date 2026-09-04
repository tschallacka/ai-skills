# MODE: DEV

# Are long skill files silently truncated when an agent loads them?

Yes. Measured, not assumed. On Claude Code the loss is **silent**, the agent
proceeds as though it has the whole skill, and `planning/SKILL.md` is affected
today: 2 of 3 runs received 977 of its 1502 lines.

The believed boundary — "codex reads about the first 300 lines" — is **wrong
twice over.** There is no line-count boundary anywhere: the limits that exist
are token or byte budgets, working out at roughly 800–1080 lines of this
repo's prose, about three times the believed figure. And codex, the agent the
belief was about, turns out not to truncate at all — it read 2600 lines of
2600.

What is true, and worse than "the rest only if it thinks it needs it", belongs
to the other two agents: with a realistic prompt Claude Code silently dropped
the tail in 5 of 6 runs and never told anyone.

A design built on 300 lines would have been wrong about the number, wrong about
the unit, and aimed at the wrong agent.

Codex was down for most of this investigation (404 on
`https://chatgpt.com/backend-api/codex/responses`) and came back near the end;
it was then measured, and it is the one agent that does **not** truncate.

## What was measured, and in which version

| | version | model(s) |
|---|---|---|
| Claude Code | 2.1.259 | `sonnet`, `opus` |
| opencode | 1.18.27 | `opencode/big-pickle`, `openrouter/anthropic/claude-sonnet-4.5` |
| codex | 0.153.0 | `gpt-5.6-luna` |

## The boundary

### Claude Code 2.1.259 — the `Read` tool has three behaviours

Only one of them is safe, and the dangerous one is the one a skill actually
hits, because an agent told to read a skill reads it with no explicit range.

| condition | behaviour | notice |
|---|---|---|
| file over 256 KB, no range | refuses outright | **loud**: `File content (509.7KB) exceeds maximum allowed size (256KB)` |
| explicit `offset`/`limit` whose range exceeds the token budget | refuses outright | **loud**: `File content (49433 tokens) exceeds maximum allowed tokens (25000)` |
| file under 256 KB, **no range** | delivers a prefix and stops | **none at all** |

The budget is **~25,000 tokens**, not a line count. Three 2600-line files, one
unbounded `Read` each:

| file | bytes | lines delivered | notice |
|---|---|---|---|
| 20-char lines | 54,644 | **2601 of 2600** (whole file) | none |
| ~65-char lines | 165,245 | **1078 of 2600** | none |
| 200-char lines | 521,924 | refused, then chunked | loud |

A 2600-line file arriving whole rules out the documented 2000-line default
being the operative limit in this version.

The cut is **perfectly deterministic**: exactly 1078 lines / 68,536 source
bytes / 72,818 delivered characters, identical across every replicate, on both
`sonnet` and `opus`. What is *not* deterministic is whether the agent notices
and issues further ranged reads to recover.

### opencode 1.18.27 — the `-f` attachment path caps at 50 KB, and says so

opencode inlines an attachment as a text message part. The database stores what
was actually sent, so this is direct evidence rather than a quiz. For the
165 KB file it stored 55,290 characters ending in:

```
(Output capped at 50 KB. Showing lines 1-807. Use offset=808 to continue.)
```

The cap is **50 KB**, and unlike Claude Code it is announced **in-band**, inside
the content the model sees. The quiz agreed exactly: markers up to line 800
recalled, 1200 and beyond reported `ABSENT` — and reported honestly, because the
model could see the footer. Identical boundary on `opencode/big-pickle` and on
`openrouter/anthropic/claude-sonnet-4.5`, so the cap is **harness-level, not
model-level**.

### codex 0.153.0 — reads the whole file, loses it across a resume

Codex is the counter-example, and it settles the premise it came from. Told to
read the same 2600-line / 165 KB file with the weak, realistic prompt, its
phase-1 output contained **all twelve markers**, at lines 50 through 2400,
28,088 tokens used. No truncation at any tested depth, in either replicate.

**The "codex reads only about the first 300 lines" belief is false**, at least
at 0.153.0 on `gpt-5.6-luna`. It read 2600.

Codex fails somewhere else instead. `codex exec` is one-shot, so phase 2 uses
`codex exec resume --last`, and across that boundary the content is gone: 1 of
12 markers recalled in the first replicate, 0 of 12 in the second, everything
else honestly reported `ABSENT`. This is failure mode 2, not truncation.

One honest limitation: that only shows codex loses the content **across a
resume**. Within the single turn that read the file the content was plainly
present — it echoed it. Whether codex retains a large skill deep inside one
long session is not measured here, because `codex exec` gave no way to ask
without crossing the resume boundary. Claude Code, by contrast, *did* retain
its markers across `--resume`, so the difference is real rather than an
artefact of the method.

### The stricter of the three decides the convention

opencode's 50 KB is stricter than Claude Code's ~25,000 tokens (~68.5 KB of
this prose), and codex imposes no read limit at all. A file safe for all three
must clear 50 KB — so opencode sets the number, and the fact that codex is
generous does not relax it.

## The real skills, measured

| file | lines | bytes | verdict |
|---|---|---|---|
| `planning/SKILL.md` | 1502 | 89,860 | **over both caps** — 75% over opencode's 50 KB |
| `CODE-STYLE.md` | 864 | 41,561 | under, little headroom |
| `planning/MAINTAINER.md` | 449 | 38,246 | under |
| `planning/ARCHITECTURE.md` | 631 | 32,323 | under |
| `MEMORY.md` | 530 | 31,922 | under |
| `planning/REVIEWER.md` | 397 | 22,954 | under |
| `todo/SKILL.md` | 395 | 17,425 | under |
| `chat/SKILL.md` | 345 | 18,612 | under |
| `bug-report/SKILL.md` | 359 | 17,520 | under |

Exactly one file is over, and it is the one that matters most. Note also that
`planning/SKILL.md` points at `ARCHITECTURE.md`, `MAINTAINER.md` and
`REVIEWER.md` — 1477 further lines and 93 KB. Each is individually under the
cap, so the dependency chain is safe *per file*; it is the monolith that is not.

Measured directly on the real file, weak prompt, Claude Code / sonnet:

| replicate | Read calls | lines delivered of 1502 |
|---|---|---|
| 1 | 2 | 1503 (recovered) |
| 2 | 1 | **977** |
| 3 | 1 | **977** |

525 lines — 35% of the skill — silently absent in two runs of three, with no
notice anywhere in the transcript.

## Three different failures, which need three different fixes

The brief asked these be kept apart, and all three actually occurred.

1. **Never sent.** Claude Code's silent prefix. The tail was never in the tool
   result; no amount of prompting about retention helps. Fix: keep files under
   the cap.
2. **Sent, then dropped.** opencode delivered every part in phase 1 and echoed
   all five load tokens, then compacted the session between turns; by phase 2
   the content was gone and it answered `ABSENT` for all five. Fix: nothing
   structural — the echo proves load-time completeness, not persistence.
3. **Sent whole, not recalled.** Not observed. Every miss traced to 1 or 2.

## The control that made this measurable

Each replicate loads the file, then the file is **moved away** before any
question is asked. A correct answer can only have come from context. Telling an
agent not to re-read and trusting it would not have distinguished these cases —
and in one run opencode did reach for the file on disk, which the move caught.

Tokens are random 12-hex strings at known line numbers. A guessable
`MARKER_400` would let a model answer without ever having seen the file.

## Recovery is stochastic, and prompting only shifts the odds

Whether the agent notices the short read and issues ranged follow-ups depends
on how hard the prompt pushes. Claude Code / sonnet, same 2600-line file:

| prompt | recovered fully | lost the tail silently |
|---|---|---|
| "in full, first line to last, do not stop early" | 6 of 7 | 1 of 7 |
| "read the skill and follow it" (realistic) | 1 of 6 | **5 of 6** |

So an imperative "read it all" directive is worth having and is **not** a
mechanism. One run in seven still failed even under maximum pressure, and under
a realistic instruction it failed five times in six.

## The four structures compared

Five parts of 520 lines / 34 KB each, comfortably inside both caps.

| structure | content lands | failure detectable |
|---|---|---|
| monolithic (status quo) | no — silently truncated | **no** |
| split + imperative read-now | yes | no |
| split + short index | yes | no |
| **split + self-verifying load** | yes | **yes** |

With every part under the cap, all three split structures loaded and retained
everything. **Splitting is what prevents the loss**; the echo does not prevent
anything.

### The fault injection, which is the whole point

`part-3` inflated to 2600 lines so its final-line `LOAD TOKEN` cannot survive
the cap. Both structures lost it. Only one said so.

Self-verifying:

> `part-3.md` was truncated — I only received lines 1-1049 of 2601, so its LOAD
> TOKEN is not in my context. Per the instructions, I must report it as MISSING
> rather than guess.
>
> `part-3.md MISSING`

Imperative, same injected fault:

> All 5 part files loaded (part-3 was truncated by the read cap at line
> 1049/2601, but the pattern was already clear and consistent) … **Ready.**

Same loss, opposite report. The imperative structure led with "all 5 loaded",
buried the truncation in a parenthetical, and signed off "Ready" — a success
claim nobody downstream can falsify. The self-verifying structure produced one
greppable word, `MISSING`, on its own line.

Detection fired in 3 of 3 self-verifying replicates. The imperative structure
recovered in 1 of 2 and produced the false success above in the other.

One confound, recorded rather than hidden: the part files label their tokens
`VERIFICATION TOKEN` and `LOAD TOKEN`, and the model volunteered both even under
the structures that never asked for them. So "was a token echoed" does not by
itself separate the structures on a healthy load — only the fault-injected run
does.

## Recommended convention

1. **No file an agent loads exceeds 40 KB.** Under opencode's 50 KB with
   headroom, and well under Claude Code's ~25,000 tokens. For this repo's prose
   that is roughly 600 lines. Measure bytes, never lines — the budget is not a
   line count.
2. **Split `planning/SKILL.md`.** It is the only file over the line, and it is
   demonstrably truncated today. *Not done here — that is Tschallacka's call.*
3. **Every part ends with a `LOAD TOKEN:` on its last line**, and the main
   `SKILL.md` requires all of them echoed before the skill may be used, with
   `MISSING` as the mandated word for one that did not arrive.
4. **The main `SKILL.md` stays small enough to be an index** — it must be the
   one file guaranteed to arrive whole.

### Why the token goes on the last line

A token at the top of a part is echoed by an agent that read only the opening,
so the mechanism would prove nothing. On the last line, echoing it means the
part was delivered to its end. This is the single detail the mechanism depends
on.

### What the mechanism does and does not buy

It **detects** an incomplete load. It does not prevent one, and it does not
guarantee the content is still there later — failure mode 2 above. Prevention is
the size cap; the echo is the net underneath it.

An honest limit: the echo is an instruction, and instructions are followed
probabilistically. It fired 3 of 3 times here, on one fault, on one agent. It
should be read as a smoke detector, not an interlock. A gate that *parses* the
echo and refuses to proceed without every token would be an interlock; that
would be a separate change.

## What did not work, and what nearly produced a wrong answer

* **Trusting the agent's account of why content is missing.** Asked which tokens
  it had, Claude Code answered `ABSENT` honestly but explained it had "declined
  to load the rest, since it's repetitive filler with no real skill content".
  The transcript shows a single unbounded `Read` handed back 1078 lines. It
  rationalised a harness truncation it could not see. **An agent's self-report is
  not evidence.**
* **A `truncat` substring as a notice detector.** It reported a harness notice on
  every read of `planning/SKILL.md`, because that file's own prose uses the word
  three times. It now matches `exceeds maximum allowed`. A check that fires on
  the content it is inspecting is exactly the kind of gate that reports success
  without checking anything.
* **Testing 300 and stopping.** Bisecting by line width is what showed the limit
  is not lines: the same 2600 lines arrived whole at 20 chars wide and were cut
  at 1078 at 65 chars wide.
* **Copying `opencode.db` before inspecting it.** It is 7.4 GB and `/tmp` here is
  tmpfs, so the copy went into RAM and Tschallacka noticed the machine
  straining. `inspect-opencode-db.py` opens the live file `mode=ro` instead —
  read-only without the copy.
* **`opencode/claude-sonnet-4-5`** returns `401 Insufficient balance` on the zen
  provider. `opencode/big-pickle` on the same provider works, so the block is
  per-model, not per-provider.

## Reproducing this

Everything is in `.agents/skill-loading-experiment/`:

| script | what it answers |
|---|---|
| `gen-marker-file.py` | builds a monolith with random tokens at known lines (`--width` for the line-cap-vs-byte-cap test) |
| `gen-split-skill.py` | builds the three split structures (`--fat-part` injects the fault) |
| `probe-claude-cap.sh` | what was *delivered*, from the transcript, one turn per point |
| `run-claude.sh` / `run-opencode.sh` | load, move the file away, quiz |
| `run-structure.sh` | scores completeness and retention per part, either agent |
| `inspect-claude-transcript.py` | per-`Read` offsets, result sizes, markers present |
| `inspect-opencode-db.py` | what opencode stored for a session, live file, read-only |
| `find-notice.py` | searches a whole transcript, not just tool results, before calling a truncation silent |
| `summarise-read-calls.py` | one TSV row per probe |
| `reps.sh` | N replicates of one condition |

## Raw results

Claude Code cap probes, one unbounded read each, `lines delivered / source lines`:

```
label          model   src_lines  src_bytes  reads  delivered_lines  delivered_chars
w20            sonnet  2600       54644      1      2601             66542
mono-r1        sonnet  2600       165245     1      1078             72818
mono-r2        sonnet  2600       165245     3      2601             177141
mono-r3        sonnet  2600       165245     3      2601             177141
mono-r4        sonnet  2600       165245     3      2601             177141
mono-r5        sonnet  2600       165245     5      2602             177344
mono-opus-r1   opus    2600       165245     3      2601             177141
mono-opus-r2   opus    2600       165245     3      2601             177141
w200           sonnet  2600       521924     3      403              81997
l1000          sonnet  1000       63545      1      1001             67443
weakA-r1       sonnet  2600       165245     1      1078             72818
weakA-r2       sonnet  2600       165245     1      1078             72818
weakA-r3       sonnet  2600       165245     1      1078             72818
weakB-r1       sonnet  2600       165245     1      1078             72818
weakB-r2       sonnet  2600       165245     1      1078             72818
weakB-r3       sonnet  2600       165245     6      2634             179175
realplan-r1    sonnet  1502       89860      2      1503             96037
realplan-r2    sonnet  1502       89860      1      977              59602
realplan-r3    sonnet  1502       89860      1      977              59602
```

Load-and-quiz replicates, markers by line (file moved away before questioning):

```
agent     mode/model                              50  250 290 310 400 800 1200+
claude    read / sonnet                           HIT HIT HIT HIT HIT HIT MISS(all)
opencode  attach / openrouter sonnet 4.5          HIT HIT HIT HIT HIT HIT MISS(all)
opencode  attach / opencode-big-pickle            HIT HIT HIT HIT HIT HIT MISS(all)
```

Split structures, 5 parts of 520 lines, `echoed / recalled` per part:

```
rep agent     structure   model                part-1..5
1   claude    verify      sonnet               all ECHOED / all HIT
1   claude    imperative  sonnet               all ECHOED / all HIT
1   claude    index       sonnet               all ECHOED / all HIT
```

Fault injection, part-3 inflated to 2600 lines:

```
rep agent     structure   model                part-3 result
1   claude    verify      sonnet               NOT_ECHOED, reported MISSING
2   claude    verify      sonnet               NOT_ECHOED, reported MISSING
3   claude    verify      sonnet               NOT_ECHOED, reported MISSING
1   claude    imperative  sonnet               NOT_ECHOED, claimed "All 5 loaded ... Ready"
2   claude    imperative  sonnet               ECHOED (recovered by chunked reads)
1   opencode  verify      big-pickle           all 5 ECHOED, then all 5 ABSENT in phase 2
```

codex 0.153.0 / gpt-5.6-luna, 2600-line file, weak prompt, markers recalled
after the file was moved away and the session resumed:

```
rep  markers in phase-1 output  markers recalled in phase 2 (after resume)
1    12 of 12 (all)             1 of 12  (line 250 only)
2    12 of 12 (all)             0 of 12
```

The left column is the read boundary and shows none. The right column is the
resume boundary and shows a near-total loss.

## Open questions for Tschallacka

**Q1.** Split `planning/SKILL.md`? It is the only file over the cap and is
measurably losing 35% of itself today. The investigation deliberately did not
touch it.

**Q2.** Should the load check become an **interlock** rather than a smoke
detector — a script that parses the echoed tokens and fails the run when one is
`MISSING`? That is enforceable where an instruction is not, and this repo has
been bitten twice by gates that reported success without checking.

**Q3.** Codex is now measured and does not truncate, which means the premise it
came from was about something else — most likely the resume/compaction loss
above, which looks identical from the outside. Worth a follow-up measurement of
retention *within* one long codex session, which `codex exec` could not reach?
