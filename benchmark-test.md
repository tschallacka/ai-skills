# Planning benchmark test

Reusable isolation procedure for benchmarking a new planning skill revision
after an improvement. The benchmark worker must run outside this repository so
it cannot inspect earlier proof directories, benchmark reports, notes, or
generated artifacts by ordinary discovery.

## Inputs

- `REVISION`: the skill revision being benchmarked, such as `1.4.1`
- `RUN_ID`: a unique run label, such as `20260808T004239Z`
- `BENCH_ROOT`: an isolated directory outside this repository, such as
  `/tmp/ai-skills-benchmark-${REVISION}-${RUN_ID}`
- `PLAN_NAME`: a new unique plan directory inside `BENCH_ROOT`, such as
  `basic-test-proof-${REVISION}-isolated-plan`
- `THREAD_ID`: the `thread.started` identifier emitted by the runner
- `RESULT_DIR`: the repository archive directory,
  `/home/tschallacka/git/ai-skills/benchmark/results/${REVISION}/${RUN_ID}`

The approved task specification is:

```text
/home/tschallacka/git/ai-skills/basic-test-proof-plan.md
```

All planning references must be resolved relative to the repository-local
skill directory:

```text
/home/tschallacka/git/ai-skills/planning/
```

For example, the current UI reference resolves to
`planning/references/ui-user-story-validation.md`.

## Isolation setup

Create a fresh working directory outside this repository. `/tmp` is preferred
because it is easy to audit and discard.

```bash
REVISION="REPLACE_WITH_REVISION"
RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)"
BENCH_ROOT="/tmp/ai-skills-benchmark-${REVISION}-${RUN_ID}"
mkdir -p "$BENCH_ROOT"
```

Copy exactly one benchmark packet into that directory. The packet is the only
document the worker receives as local context. It contains the worker prompt
below plus the absolute paths to the approved task specification and
repository-local planning skill.

```bash
cp benchmark-test.md "$BENCH_ROOT/benchmark-test.md"
```

Before starting the worker, verify that the isolated directory does not contain
prior results or repository artifacts:

```bash
find "$BENCH_ROOT" -maxdepth 2 -type f -print
```

The expected output before the worker starts is only:

```text
/tmp/ai-skills-benchmark-${REVISION}-${RUN_ID}/benchmark-test.md
```

Do not copy prior benchmark results, previous proof directories, repository
notes, generated artifacts, or `.git` data into `BENCH_ROOT`.

## Worker launch

Start a fresh persisted telemetry runner with its current working directory set
to `BENCH_ROOT`. The worker must see only `benchmark-test.md` in its local
directory at startup. Do not use `--ephemeral`; SQLite telemetry is required.

The worker may read these absolute source paths:

- `/home/tschallacka/git/ai-skills/basic-test-proof-plan.md`
- `/home/tschallacka/git/ai-skills/planning/SKILL.md`
- repository-local references named by that skill, resolved under
  `/home/tschallacka/git/ai-skills/planning/`

The worker must not read, list, search, compare, or summarize any other path in
`/home/tschallacka/git/ai-skills`, including previous plan directories,
benchmark reports, brainstorm documents, result directories under
`benchmark/results/`, `.plans/`, git history, or other repository files.

## Benchmark prompt

Paste the following prompt into a fresh runner after replacing `REVISION` and
`PLAN_NAME`. Launch it from `BENCH_ROOT`, not from this repository:

```text
Run a fresh isolated planning-only basic test proof for the repository-local
planning skill revision REVISION.

Your current working directory is an isolated benchmark directory outside the
repository. Treat this directory as your whole benchmark workspace. At startup
it should contain only benchmark-test.md.

Use only this repository-local skill as the benchmark target:

/home/tschallacka/git/ai-skills/basic-test-proof-plan.md
/home/tschallacka/git/ai-skills/planning/SKILL.md

Read benchmark-test.md, the task specification, and the repository-local
planning skill in full. When planning/SKILL.md names another reference, resolve
that path relative to /home/tschallacka/git/ai-skills/planning/ and read the
resolved repository-local file. This includes the UI planning reference when
the skill requires it.

Do not inspect, list, search, read, copy, compare against, summarize, or use
any prior proof directory, benchmark report, plan, brainstorm, result archive,
git history, generated artifact, or repository file outside the exact source
paths named above and the relative planning references named by
planning/SKILL.md. Do not read or use the installed skill at
/home/tschallacka/.codex/skills/planning. Only benchmark-test.md, the approved
task specification, the repository-local planning skill, and its relative
references may be used as benchmark inputs.

Create a fresh plan under:

PLAN_NAME

The future task is:

- Create button-chain.html with one initial button.
- Pressing the current last button appends exactly one button below it.
- Pressing the fourth generated button clears the document.
- The completion state prints the exact lowercase text "finished" with a visible white border.

This is a planning-only proof. Do not create, edit, open, inspect, serve, or
test any HTML. Do not start a browser, server, driver, or other execution
tooling.

Use normal runner behavior. Do not force sequential execution, disable
parallelism, disable subagents, or use ephemeral mode. Preserve the full Codex
SQLite telemetry for this run.

Complete the decomposition, atomic work-unit inventory, UI user story, UI
story run/cache, testing companions, adversarial review, bug register,
progress trackers, context snapshot, validation, and analysis report.

Record exact start/end timestamps, elapsed time, worker result, validation
results, review result, artifact/process audit, thread ID, and token usage.

If a repository-local reference named by planning/SKILL.md is unavailable,
check the resolved path under the repository's planning/ directory and report
that fact. Do not substitute the installed skill or claim that a reference is
missing without checking the repository-local path.

Before completion, audit only the isolated benchmark directory for generated
artifacts and the expected plan output. Do not audit the repository except for
the exact allowed source paths above.

End with a concise execution summary and the exact path to the analysis report.
```

## Telemetry command

Run this after completion. Replace `THREAD_ID` with the `thread.started` ID.
The run must not use `--ephemeral`, or the SQLite record may be unavailable.

```bash
THREAD_ID="REPLACE_WITH_THREAD_ID" python3 - <<'PY' > "$BENCH_ROOT/telemetry.txt"
import json
import os
import re
import sqlite3

thread_id = os.environ["THREAD_ID"]
db = "/home/tschallacka/.codex/logs_2.sqlite"
con = sqlite3.connect(db)

total = 0
records = 0

for row_id, ts, body in con.execute(
    "select id, ts, feedback_log_body from logs "
    "where thread_id=? order by id",
    (thread_id,),
):
    objects = []
    text = body or ""

    try:
        objects.append(json.loads(text))
    except Exception:
        for match in re.findall(r"\{.*?\}", text):
            try:
                objects.append(json.loads(match))
            except Exception:
                pass

    def values(value):
        if isinstance(value, dict):
            for key, item in value.items():
                if key == "total_usage_tokens" and isinstance(item, (int, float)):
                    yield item
                yield from values(item)
        elif isinstance(value, list):
            for item in value:
                yield from values(item)

    for obj in objects:
        for value in values(obj):
            total += value
            records += 1

print(f"thread_id={thread_id}")
print(f"usage_records={records}")
print(f"total_usage_tokens={int(total)}")
PY
```

If SQLite has no matching records, record token usage as unavailable in
`telemetry.txt`. Never infer or invent a token count.

## Archive results

After the worker and telemetry extraction complete, create the versioned result
archive in this repository and copy the isolated outputs into it:

```bash
RESULT_DIR="/home/tschallacka/git/ai-skills/benchmark/results/${REVISION}/${RUN_ID}"
mkdir -p "$RESULT_DIR"
cp -R "$BENCH_ROOT"/. "$RESULT_DIR"/
```

The result archive must contain:

- `benchmark-test.md`, the exact packet given to the worker.
- `telemetry.txt`, containing `thread_id`, `usage_records`, and
  `total_usage_tokens`, or an explicit unavailable result.
- The generated plan directory.
- The worker's analysis report.
- Any context snapshots, validation output, audit notes, and review artifacts
  generated inside `BENCH_ROOT`.

Write the evaluation in:

```text
benchmark/results/REVISION/RUN_ID/evaluation.md
```

The evaluation must state whether the run is accepted or tainted. It must
record the isolated directory path, result archive path, thread ID, telemetry
records, total usage tokens, start/end timestamps, elapsed seconds, work-unit
count, goal count, validation result, context audit result, context benchmark
result, review result, and artifact/process audit result. If any telemetry or
required evidence is missing, label the run tainted rather than filling gaps
from prior reports.

## Completion checks

Before accepting the benchmark:

- [ ] The worker was launched from a new `BENCH_ROOT` outside this repository.
- [ ] `BENCH_ROOT` contained only `benchmark-test.md` before the worker started.
- [ ] The plan directory is new under `BENCH_ROOT` and contains the complete
      durable artifact set.
- [ ] The plan has the intended revision and exact future task contract.
- [ ] The worker used only the repository-local `planning/` skill and its
      relative references, not the installed Codex skill.
- [ ] The worker did not inspect previous proof directories, benchmark
      results, repository notes, `.plans/`, git history, or unrelated files.
- [ ] `planning/scripts/validate-plan.sh PLAN_NAME` passes from the isolated
      run.
- [ ] Required context audit and benchmark checks pass.
- [ ] No HTML/HTM artifact was created anywhere in `BENCH_ROOT` or the result
      archive.
- [ ] No browser, server, driver, or unrelated process was started by the run.
- [ ] Pre-existing repository files were unchanged until the explicit result
      archive copy.
- [ ] `benchmark/results/REVISION/RUN_ID/` contains the copied worker output and
      `evaluation.md`.
- [ ] The report records start, end, elapsed time, worker result, and review status.
- [ ] SQLite telemetry is recorded, or its absence is explicitly documented.

## Result template

Record one row per revision in the benchmark comparison:

| Revision | Isolated root | Result archive | Plan | Start | End | Elapsed seconds | Usage tokens | Records | Work units | Goals | Validation | Review | Status |
|---|---|---|---|---|---|---:|---:|---:|---:|---:|---|---|---|
| `REVISION` | `BENCH_ROOT` | `benchmark/results/REVISION/RUN_ID` | `PLAN_NAME` |  |  |  |  |  |  |  |  |  |  |

Token counts from different runner modes must be labelled clearly. SQLite
telemetry totals are preferred; a CLI-reported total is not equivalent unless
the accounting scope is documented. Runs without persisted telemetry are
tainted unless the benchmark purpose explicitly allows a no-token smoke test.

## Isolation rule

Each benchmark must use a new isolated directory outside this repository, a
new plan directory inside that isolated directory, and a new persisted runner
thread. Do not modify, overwrite, read, or use previous benchmark artifacts as
inputs. The only reusable inputs are the approved task specification, the
repository-local `planning/` skill, and references explicitly named by that
skill and resolved relative to the repository's `planning/` directory.

Previous results are available only to the evaluator after the worker run has
finished and after the isolated output has been copied into
`benchmark/results/REVISION/RUN_ID/`. A worker that reads prior benchmark
material before completing its plan is tainted, even if the generated plan
passes validation.
