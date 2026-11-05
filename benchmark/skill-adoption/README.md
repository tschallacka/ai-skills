<!-- MODE: DEV -->
# skill-adoption benchmark

Does an agent reach for this repo's own skills and MCP tools unprompted,
and when it does (or doesn't), does the task actually come out right?
Twenty scenarios (`SCENARIOS.md`), one per skill or skill pair, each
phrased so the right tool is the obviously good choice without ever naming
it — adoption and correctness are scored and reported separately, never
conflated (see `DATA-SPEC.md`'s Metrics section for exactly how).

## Layout

- `SCENARIOS.md` — the scenario set, prose rationale for what each one
  measures and why.
- `scenarios/<id>/` — the executable twin: `scenario.json` (machine-readable
  manifest) + `scenario.sh` (setup/verify).
- `PROTOCOL.md` — the exact procedure one cell runs under: the subject/
  experimenter/orchestrator role split, the launch sequence, the proven
  `claude` env-stripping fix and its unverified `opencode` analogue.
- `DATA-SPEC.md` — storage and reproducibility specification: directory
  layout, JSON Schemas (`schema/`), versioning, and the metrics
  `aggregate.py` computes (with Wilson confidence intervals, not the normal
  approximation — this benchmark's rep counts are small).
- `run-cell.sh`, `make-experimenter-brief.py`, `record-run.py`,
  `aggregate.py` — the orchestration: one cell end to end, brief generation
  for the Haiku experimenter, bundle validation/recording, and rollup.
- `results/` — gitignored. Raw evidence from live interactive sessions
  (`runs.jsonl`, `registry.jsonl`, `transcripts/`) plus the regenerable
  `aggregate.json`/`AGGREGATE.md`. Not committed until a specific pass is
  deliberately kept, same convention as `benchmark/ai-text-editor-usage/`.

## Running one cell

```bash
./run-cell.sh S04-decisions claude mcp 1
```

Builds a disposable workspace, installs the full skill collection into a
disposable `HOME` at the requested mode, launches a real `claude -p
--model haiku` experimenter that drives a real interactive `claude` (or
`opencode`) subject through `interactive-shell`, and appends one record to
`results/runs.jsonl` on completion. **This is a real, billed, live-model
benchmark run, not a unit test** — it costs real time and tokens for both
the subject and the experimenter. See `PROTOCOL.md` step 6 for the 10-minute
default timeout and which scenarios are known to need a different one.

`S<NN>-<name>` scenario ids, `cli` is `claude` or `opencode`, `mode` is
`skill`, `mcp`, or `n/a` (for the convention-only scenarios, which have no
binary and only one real mode — `run-cell.sh` still installs a real skill
collection underneath, defaulting to `mcp`).

## Running a full pass

No multi-cell driver script exists yet (nothing has needed one — the first
passes have been run cell by cell to validate the harness itself). The
straightforward loop, once that validation is done:

```bash
for scenario_dir in scenarios/S*; do
    id="$(basename "$scenario_dir")"
    modes="$(python3 -c "import json; print(' '.join(json.load(open('$scenario_dir/scenario.json'))['modes']))")"
    for cli in claude opencode; do
        for mode in $modes; do
            for rep in 1 2; do
                ./run-cell.sh "$id" "$cli" "$mode" "$rep"
            done
        done
    done
done
python3 aggregate.py results/runs.jsonl
```

Run cells in parallel across separate workspaces where your orchestration
allows it — nothing here shares state between cells except the append-only
`results/` files, which is exactly what `jsonl` is for.

## First things to verify once this actually runs live

`PROTOCOL.md`'s "Known limitations and unverified assumptions" section is
the authoritative list; the two things most likely to need a fix before
any number here means anything:

1. Does `installer install --source <repo> --package dev --agent <cli>
   --all --yes --integration ...` actually produce a working, agent-usable
   skill set under a bare disposable `HOME`, with no further setup?
2. Does the `claude` subject's env-stripping fix (confirmed in
   `benchmark/ai-text-editor-usage/LAB-PROTOCOL.md`) transfer unchanged to
   this harness, and does `opencode` need an analogous fix at all?

A cell where the subject never adopts anything across several reps, or
where the experimenter's bundle never parses, is worth checking against
these two before it is scored as a genuine finding.
