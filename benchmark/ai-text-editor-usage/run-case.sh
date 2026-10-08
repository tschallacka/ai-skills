#!/usr/bin/env bash
# MODE: DEV
# Runs ONE condition (treatment: ai-text-editor-mcp available; baseline:
# stock Read/Write/Edit only) of the ai-text-editor-usage comparison, end to
# end: seed an isolated workspace, launch the real `claude -p` worker, land
# TWO external edits mid-task -- a single-line TAX_RATE bump and a whole-file
# reindent, both backgrounded, simulating a teammate's own concurrent change
# (a business-constant bump) and their editor's format-on-save (PhpStorm,
# VS Code) landing on top of it -- wait for the worker to finish, and write a
# scored result under results/.
#
# Both conditions see the IDENTICAL task-prompt.md -- no mention of
# ai-text-editor anywhere in it. Whether the treatment worker reaches for the
# tool at all is itself part of what this measures: a tool only available
# but never pitched in the prompt is exactly how a real session works, and
# is the fairest test of whether the tool's own description sells itself.
#
# Usage: run-case.sh <treatment|baseline> <results-base-dir> <run-id> [edit-delay-seconds] [reformat-delay-seconds]

set -euo pipefail

if [ "$#" -lt 3 ] || [ "$#" -gt 5 ]; then
    echo "Usage: $0 <treatment|baseline> <results-base-dir> <run-id> [edit-delay-seconds] [reformat-delay-seconds]" >&2
    exit 64
fi

CONDITION="$1"
RESULTS_BASE="$2"
RUN_ID="$3"
EDIT_DELAY="${4:-8}"
REFORMAT_DELAY="${5:-14}"

case "$CONDITION" in
    treatment|baseline) ;;
    *) echo "condition must be 'treatment' or 'baseline', got: $CONDITION" >&2; exit 64 ;;
esac

FRAMEWORK_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$FRAMEWORK_DIR/../.." && pwd)"
CASE_DIR="$RESULTS_BASE/$RUN_ID/$CONDITION"
WORKSPACE="$CASE_DIR/workspace"

rm -rf "$CASE_DIR"
mkdir -p "$WORKSPACE"
cp "$FRAMEWORK_DIR/fixtures/task_source.py" "$WORKSPACE/task_source.py"
cp "$FRAMEWORK_DIR/fixtures/test_task_source.py" "$WORKSPACE/test_task_source.py"

MODEL="${AI_TEXT_EDITOR_BENCH_MODEL:-sonnet}"

# bypassPermissions, not acceptEdits: acceptEdits only pre-approves file
# edits, not Bash -- confirmed across every earlier run, both workers
# correctly diagnosed the TAX_RATE collision and tried to run the shipped
# test, but every `python3 test_task_source.py` call sat pending approval
# that a headless run can never answer, so the test-resolution dimension
# was never actually exercised. Each run's workspace is a disposable copy
# under results/<run-id>/<condition>/workspace, never the live repo, so
# bypassing the prompt here is scoped appropriately.
AGENT_ARGV=(claude -p --output-format=json --add-dir "$WORKSPACE" --permission-mode bypassPermissions --model "$MODEL")

if [ "$CONDITION" = treatment ]; then
    MCP_CONFIG="$CASE_DIR/ai-text-editor.json"
    AI_TEXT_EDITOR_MCP_BIN="$REPO_ROOT/target/release/ai-text-editor-mcp"
    if [ ! -x "$AI_TEXT_EDITOR_MCP_BIN" ]; then
        echo "run-case.sh: ai-text-editor-mcp binary not found at $AI_TEXT_EDITOR_MCP_BIN -- build it first (cargo build --release -p ai-text-editor-mcp)" >&2
        exit 69
    fi
    sed "s#__AI_TEXT_EDITOR_MCP_BIN__#$AI_TEXT_EDITOR_MCP_BIN#" \
        "$FRAMEWORK_DIR/mcp-config/ai-text-editor.json" > "$MCP_CONFIG"
    AGENT_ARGV+=(--mcp-config "$MCP_CONFIG" --strict-mcp-config)
else
    # --strict-mcp-config only gates MCP *servers* -- it does nothing about
    # the separate Skill tool, which still loads the globally-installed
    # ai-text-editor SKILL.md regardless (confirmed: a baseline run without
    # this flag discovered and considered the skill anyway, rejecting it
    # only because its CLI binary sat outside --add-dir, not because it was
    # unaware of it). Disallow Skill entirely so baseline is a genuine
    # zero-exposure control, not "MCP unavailable, skill still discoverable".
    # The =-form binds exactly one value; a bare `--disallowedTools Skill`
    # followed by the prompt string greedily swallows the prompt into the
    # variadic tool list instead (confirmed: the worker then fails instantly
    # with "Input must be provided either through stdin or as a prompt
    # argument when using --print").
    AGENT_ARGV+=(--strict-mcp-config --disallowedTools=Skill)
fi

PROMPT="$(cat "$FRAMEWORK_DIR/task-prompt.md")"
AGENT_ARGV+=("$PROMPT")

EXTERNAL_EDIT_LOG="$CASE_DIR/external-edit.log"
EXTERNAL_FORMAT_LOG="$CASE_DIR/external-format.log"
: > "$EXTERNAL_EDIT_LOG"
: > "$EXTERNAL_FORMAT_LOG"
"$FRAMEWORK_DIR/external-edit.sh" "$WORKSPACE" "$EDIT_DELAY" "$EXTERNAL_EDIT_LOG" &
EXTERNAL_EDIT_PID=$!
"$FRAMEWORK_DIR/external-format.sh" "$WORKSPACE" "$REFORMAT_DELAY" "$EXTERNAL_FORMAT_LOG" &
EXTERNAL_FORMAT_PID=$!

echo "run-case.sh: launching $CONDITION worker (model=$MODEL, TAX_RATE bump at +${EDIT_DELAY}s, reindent at +${REFORMAT_DELAY}s)" >&2
START_EPOCH="$(date -u +%s.%N)"
set +e
( cd "$WORKSPACE" && "${AGENT_ARGV[@]}" ) >"$CASE_DIR/worker-stdout.json" 2>"$CASE_DIR/worker-stderr.log"
WORKER_EXIT=$?
set -e
END_EPOCH="$(date -u +%s.%N)"
WALL_SECONDS="$(awk -v a="$START_EPOCH" -v b="$END_EPOCH" 'BEGIN{printf "%.3f", b-a}')"

wait "$EXTERNAL_EDIT_PID" || true
wait "$EXTERNAL_FORMAT_PID" || true

SESSION_ID="$(sed -nE 's/.*"session_id"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' "$CASE_DIR/worker-stdout.json" 2>/dev/null | sed -n '1p')"
echo "${SESSION_ID:-}" > "$CASE_DIR/session-id.txt"

{
    echo "worker_exit=$WORKER_EXIT"
    echo "wall_seconds=$WALL_SECONDS"
    if [ -n "${SESSION_ID:-}" ]; then
        "$FRAMEWORK_DIR/telemetry.sh" "$SESSION_ID"
    else
        echo "telemetry_status=unavailable:no session id parsed from worker output"
    fi
} > "$CASE_DIR/telemetry.txt"

if [ -n "${SESSION_ID:-}" ]; then
    TRANSCRIPT="$(find "${CLAUDE_PROJECTS_DIR:-$HOME/.claude/projects}" -type f -name "${SESSION_ID}.jsonl" -print -quit 2>/dev/null || true)"
    if [ -n "$TRANSCRIPT" ]; then
        python3 "$FRAMEWORK_DIR/extract_telemetry.py" "$TRANSCRIPT" --json > "$CASE_DIR/telemetry.json" || echo '{}' > "$CASE_DIR/telemetry.json"
    else
        echo '{}' > "$CASE_DIR/telemetry.json"
    fi
else
    echo '{}' > "$CASE_DIR/telemetry.json"
fi

python3 "$FRAMEWORK_DIR/score.py" "$WORKSPACE" > "$CASE_DIR/score.json" || echo '{}' > "$CASE_DIR/score.json"

cat > "$CASE_DIR/evaluation.md" <<EOF
# ai-text-editor-usage: $CONDITION ($RUN_ID)

- Model: $MODEL
- Worker exit code: $WORKER_EXIT
- Wall clock: ${WALL_SECONDS}s
- Session id: ${SESSION_ID:-(none parsed)}
- External edit logs: see external-edit.log (TAX_RATE bump), external-format.log (reindent)
- Telemetry: see telemetry.txt / telemetry.json
- Score: see score.json
EOF

echo "run-case.sh: $CONDITION done, exit=$WORKER_EXIT, wall=${WALL_SECONDS}s, session=${SESSION_ID:-none}" >&2
