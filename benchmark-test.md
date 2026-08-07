# Planning benchmark test

Reusable procedure for benchmarking a new planning skill revision after an
improvement. Replace the version and plan-name placeholders for each run.

## Inputs

- `REVISION`: the skill revision being benchmarked, such as `1.4.1`
- `PLAN_NAME`: a new unique plan directory, such as
  `basic-test-proof-${REVISION}-independent-plan`
- `THREAD_ID`: the `thread.started` identifier emitted by the runner

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

## Benchmark prompt

Paste the following prompt into a fresh runner after replacing `REVISION` and
`PLAN_NAME`:

```text
Run a fresh planning-only basic test proof for the repository-local planning
skill revision REVISION.

Use only this repository-local skill as the benchmark target:

/home/tschallacka/git/ai-skills/basic-test-proof-plan.md
/home/tschallacka/git/ai-skills/planning/SKILL.md

Read the task specification and the repository-local planning skill in full.
When planning/SKILL.md names another reference, resolve that path relative to
/home/tschallacka/git/ai-skills/planning/ and read the resolved repository-local
file. This includes the UI planning reference when the skill requires it.

Do not inspect, read, copy, compare against, or use any prior proof directory,
report, plan, or generated artifact. This includes all files and directories
from earlier benchmark revisions. Do not read or use the installed skill at
/home/tschallacka/.codex/skills/planning. Only the task specification and the
repository-local planning skill plus its relative references may be used as
benchmark inputs.

Create a fresh plan under:

PLAN_NAME

The future task is:

- Create button-chain.html with one initial button.
- Pressing the current last button appends exactly one button below it.
- Pressing the fourth generated button clears the document.
- The completion state prints the exact lowercase text “finished” with a visible white border.

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

End with a concise execution summary and the exact path to the analysis report.
```

## Telemetry command

Run this after completion. Replace `THREAD_ID` with the `thread.started` ID.
The run must not use `--ephemeral`, or the SQLite record may be unavailable.

```bash
THREAD_ID="REPLACE_WITH_THREAD_ID" python3 - <<'PY'
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

If SQLite has no matching records, record token usage as unavailable. Never
infer or invent a token count.

## Completion checks

Before accepting the benchmark:

- [ ] The plan directory is new and contains the complete durable artifact set.
- [ ] The plan has the intended revision and exact future task contract.
- [ ] The worker used only the repository-local `planning/` skill and its
      relative references, not the installed Codex skill.
- [ ] `planning/scripts/validate-plan.sh PLAN_NAME` passes.
- [ ] Required context audit and benchmark checks pass.
- [ ] No HTML/HTM artifact was created anywhere in the benchmark output.
- [ ] No browser, server, driver, or unrelated process was started by the run.
- [ ] Pre-existing files are unchanged.
- [ ] The report records start, end, elapsed time, worker result, and review status.
- [ ] SQLite telemetry is recorded, or its absence is explicitly documented.

## Result template

Record one row per revision in the benchmark comparison:

| Revision | Plan | Start | End | Elapsed seconds | Usage tokens | Records | Work units | Goals | Validation | Review |
|---|---|---|---|---:|---:|---:|---:|---:|---|---|
| `REVISION` | `PLAN_NAME` |  |  |  |  |  |  |  |  |  |

Token counts from different runner modes must be labelled clearly. SQLite
telemetry totals are preferred; a CLI-reported total is not equivalent unless
the accounting scope is documented.

## Isolation rule

Each benchmark must use a new plan directory and a new persisted runner
thread. Do not modify, overwrite, or use previous benchmark artifacts as
inputs. The only reusable inputs are the approved task specification and the
repository-local `planning/` skill with references resolved relative to that
directory.
