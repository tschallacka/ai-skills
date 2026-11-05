#!/usr/bin/env bash
# S01-ai-text-editor: setup WORKDIR | verify WORKDIR
#
# Delegates to the existing dedicated benchmark instead of re-deriving a
# task -- see scenario.json's "notes". `setup` copies its fixture into this
# cell's workspace; `verify` runs its own scorer against the result.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ATE_BENCH="$HERE/../../../ai-text-editor-usage"
MODE="${1:-}"
WORKDIR="${2:-}"

if [ -z "$MODE" ] || [ -z "$WORKDIR" ]; then
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
fi

case "$MODE" in
setup)
    cp "$ATE_BENCH/fixtures/task_source.py" "$WORKDIR/task_source.py"
    cp "$ATE_BENCH/fixtures/test_task_source.py" "$WORKDIR/test_task_source.py"
    cp "$ATE_BENCH/task-prompt.md" "$WORKDIR/.task-prompt.md"
    ;;
verify)
    if [ ! -f "$WORKDIR/task_source.py" ]; then
        printf '{"task_correct": false, "notes": "task_source.py missing from workspace"}\n'
        exit 0
    fi
    # score.py takes the WORKSPACE dir, not the file -- it appends
    # task_source.py (and looks for test_task_source.py) itself. Passing
    # the file path here doubled it (WORKDIR/task_source.py/task_source.py),
    # which score.py's own Path.read_text then refused with ENOTDIR --
    # measured live (2026-10-19), scored every correct run as task_correct:
    # false for a harness reason that had nothing to do with the subject.
    # score.py never prints a literal "PASS" string anywhere (confirmed by
    # reading it) -- the previous `grep -qi "PASS"` here could not have
    # matched on ANY run, correct or not, so task_correct was always false
    # regardless of the subject's work. score.py's own verdict is its
    # "outcome" field ("pass"/"deferred"/"fail"; "pass" already requires
    # the three source edits plus the shipped regression test to actually
    # pass -- see score.py's own docstring for "deferred").
    SCORE_OUT="$(python3 "$ATE_BENCH/score.py" "$WORKDIR" 2>&1)" || true
    printf '%s' "$SCORE_OUT" | python3 -c '
import json, sys
raw = sys.stdin.read()
try:
    report = json.loads(raw)
    correct = report.get("outcome") == "pass"
    notes = json.dumps(report)
except json.JSONDecodeError:
    correct = False
    notes = raw[:2000]
print(json.dumps({"task_correct": correct, "notes": notes}))
'
    ;;
*)
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
    ;;
esac
