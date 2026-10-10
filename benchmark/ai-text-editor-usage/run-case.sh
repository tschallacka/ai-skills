#!/usr/bin/env bash
# MODE: DEV
# Runs ONE condition (treatment: ai-text-editor-mcp available; baseline:
# stock Read/Write/Edit only) of the ai-text-editor-usage comparison, end to
# end: seed an isolated workspace, launch the real `claude -p` worker, land
# TWO external edits once the worker's own checkpoint files say it is the
# right moment -- a single-line TAX_RATE bump and a whole-file reindent,
# both backgrounded, simulating a teammate's own concurrent change (a
# business-constant bump) and their editor's format-on-save (PhpStorm,
# VS Code) landing on top of it -- wait for the worker to finish, and write
# a scored result under results/.
#
# An interactive-shell + Haiku-classifier driven version of this launch was
# built and run nine times to validate it (see git history on this file):
# dialog handling, prompt injection and completion detection all worked, but
# the resulting transcript kept being attributed to the PARENT Claude Code
# session driving the benchmark rather than the worker's own pinned
# --session-id, even after clearing the obvious inherited env var
# (CLAUDE_CODE_SESSION_ID) -- treated at the time as a real, unresolved
# platform behavior. It has since been root-caused and fixed (see LAB mode
# below and LAB-PROTOCOL.md's "Known limitations"): CLAUDE_CODE_CHILD_SESSION
# was the var that mattered and was never tried. `-p` mode has never shown
# this (it reports its own session id directly in its JSON stdout,
# regardless of parent-session context), so the worker launch reverted to
# it; the event-driven collision triggers and the CSV-checkpoint task design
# below do not depend on how the worker is launched and are unaffected.
#
# Every condition sees the IDENTICAL task-prompt.md -- no mention of
# ai-text-editor anywhere in it, ever, for any run this script makes. Whether
# the treatment worker reaches for the tool at all is itself part of what
# this measures: a tool only available but never pitched in the prompt is
# exactly how a real session works, and is the fairest test of whether the
# tool's own description sells itself. An end user should never have to
# nudge a prompt to get good tool behavior -- the only legitimate nudge
# surfaces are things the TOOL AUTHOR controls, and this harness varies all
# three it currently has, independently: the MCP server's own top-level
# `instructions` pitch (normally absent; AI_TEXT_EDITOR_MCP_INSTRUCTIONS),
# and editor-gate-plugin's two PreToolUse hooks, Edit/Write and Read
# (EDITOR_GATE_EDIT_WRITE_TEXT / EDITOR_GATE_READ_TEXT). See
# run-hook-matrix-sweep.sh, which runs the full 2^3 factorial over them to
# find which combination actually moves unprompted adoption -- the question
# this whole benchmark exists to answer.
#
# The two collisions are EVENT-driven, not time-driven: they fire when the
# worker's own tax_rates.csv / departments.csv checkpoint files appear in the
# workspace (task-prompt.md asks for both, after the real edits and right
# before the test run), not after a fixed wall-clock delay. A fixed delay
# cannot mean the same thing across workers of very different speed -- the
# lever factorial's own runs ranged 23-55s wall clock, so a "+8s" collision
# landed at roughly a third of the way through a 23s run and barely a
# seventh of the way through a 55s one, silently confounding any comparison
# across conditions. Waiting for the task's own checkpoint files lands each
# collision at the same point in every worker's own progress instead. This
# part of the design is launch-mode-independent -- both [launch-mode] values
# below use it unchanged.
#
# [launch-mode] "print" (default) is the mechanism above: a headless
# `claude -p --permission-mode bypassPermissions` worker. [launch-mode] "lab"
# is a different mechanism entirely -- see LAB-PROTOCOL.md for the full
# procedure. In short: the worker ("the subject") runs as a genuine
# interactive `claude --session-id <pinned-uuid>` session with real
# permission prompts, driven by a separate `claude -p --model haiku` process
# ("the experimenter") that types the task prompt, answers permission
# prompts the way a human operator would, detects completion, runs a fixed
# debrief, captures token/cost data via the interactive session's own
# `/cost` command, and writes one JSON bundle. Lab mode was originally
# designed to not need the subject's own transcript file at all (everything
# in the bundle comes from the live screen or from hook-based telemetry),
# precisely because of the bug described above -- but that bug is now
# fixed (LAB_SUBJECT_ARGV below strips CLAUDE_CODE_CHILD_SESSION and related
# vars for the subject process), so the transcript is available too when
# wanted, confirmed by a live `claude --resume <pinned-uuid>` call.
#
# Usage: run-case.sh <treatment|baseline> <results-base-dir> <run-id> [edit-timeout-seconds] [reformat-timeout-seconds] [levers] [launch-mode]
#
# [edit-timeout-seconds] / [reformat-timeout-seconds] (default 90 each) bound
# how long external-edit.sh / external-format.sh wait for their marker file
# before giving up -- a ceiling, not a bet, per github-ci-runners.md's own
# "a readiness budget is a ceiling, not a sleep."
#
# [levers] (default: edit+read -- today's shipped default: both hooks fire,
# no MCP instructions pitch, same as a plain run-comparison.sh invocation)
# is "none" or a "+"-joined subset of:
#   mcp   the MCP server advertises a short top-level instructions pitch
#   edit  the Edit/Write hook fires with its shipped reminder text
#   read  the Read hook fires with its shipped reminder text
# e.g. "mcp", "edit+read", "mcp+edit+read" (every lever on, the ceiling).
#
# [launch-mode] (default: print) is "print" or "lab" -- see above.

set -euo pipefail

if [ "$#" -lt 3 ] || [ "$#" -gt 7 ]; then
    echo "Usage: $0 <treatment|baseline> <results-base-dir> <run-id> [edit-timeout-seconds] [reformat-timeout-seconds] [levers] [launch-mode]" >&2
    exit 64
fi

CONDITION="$1"
RESULTS_BASE="$2"
RUN_ID="$3"
EDIT_TIMEOUT="${4:-90}"
REFORMAT_TIMEOUT="${5:-90}"
LEVERS="${6:-edit+read}"
LAUNCH_MODE="${7:-print}"

case "$CONDITION" in
    treatment|baseline) ;;
    *) echo "condition must be 'treatment' or 'baseline', got: $CONDITION" >&2; exit 64 ;;
esac

case "$LAUNCH_MODE" in
    print|lab) ;;
    *) echo "launch-mode must be 'print' or 'lab', got: $LAUNCH_MODE" >&2; exit 64 ;;
esac

want_mcp=false want_edit=false want_read=false
if [ "$LEVERS" != "none" ]; then
    IFS='+' read -ra lever_tokens <<< "$LEVERS"
    for lever in "${lever_tokens[@]}"; do
        case "$lever" in
            mcp)  want_mcp=true ;;
            edit) want_edit=true ;;
            read) want_read=true ;;
            *) echo "levers must be 'none' or a '+'-joined subset of mcp, edit, read; got token: $lever" >&2; exit 64 ;;
        esac
    done
fi

# Unset (the shipped default) lets editor-gate-plugin's own hook text
# through unchanged, and leaves the MCP server's own instructions field
# absent (today's shipped shape); setting a var to an empty string
# suppresses that
# hook's reminder entirely (see editor-gate-plugin/hooks/lib.sh's
# editor_gate_reminder). These are this process's own exports, inherited by
# the `claude -p` child below and, through it, by the hook scripts -- never
# global or persisted.
AI_TEXT_EDITOR_MCP_PITCH="Durable editor tabs with disk-change detection: unlike Read/Edit/Write, this tool notices when a file changed on disk since you last read it and refuses or offers resolution instead of silently overwriting a concurrent change. Prefer it over Read/Edit/Write when a file might be touched by another process while you work, or for span-addressed edits that need no re-quoting of matched text."

if [ "$want_mcp" = true ]; then
    export AI_TEXT_EDITOR_MCP_INSTRUCTIONS="$AI_TEXT_EDITOR_MCP_PITCH"
else
    export AI_TEXT_EDITOR_MCP_INSTRUCTIONS=""
fi
if [ "$want_edit" = true ]; then
    unset EDITOR_GATE_EDIT_WRITE_TEXT
else
    export EDITOR_GATE_EDIT_WRITE_TEXT=""
fi
if [ "$want_read" = true ]; then
    unset EDITOR_GATE_READ_TEXT
else
    export EDITOR_GATE_READ_TEXT=""
fi

FRAMEWORK_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$FRAMEWORK_DIR/../.." && pwd)"
PROMPT_FILE="$FRAMEWORK_DIR/task-prompt.md"
CASE_DIR="$RESULTS_BASE/$RUN_ID/$CONDITION"
WORKSPACE="$CASE_DIR/workspace"

rm -rf "$CASE_DIR"
mkdir -p "$WORKSPACE"
cp "$FRAMEWORK_DIR/fixtures/task_source.py" "$WORKSPACE/task_source.py"
cp "$FRAMEWORK_DIR/fixtures/test_task_source.py" "$WORKSPACE/test_task_source.py"

MODEL="${AI_TEXT_EDITOR_BENCH_MODEL:-sonnet}"

# Shared across both launch modes: print mode's worker and lab mode's
# subject both need --add-dir, --model, and the same MCP/Skill wiring. Each
# mode appends its own permission mode below (bypassPermissions for a
# headless print-mode worker that can never answer a real prompt;
# acceptEdits for a lab-mode subject whose real prompts a live experimenter
# answers), and print mode alone appends -p/--output-format/the prompt text.
AGENT_COMMON_ARGV=(claude --add-dir "$WORKSPACE" --model "$MODEL")

if [ "$CONDITION" = treatment ]; then
    MCP_CONFIG="$CASE_DIR/ai-text-editor.json"
    AI_TEXT_EDITOR_MCP_BIN="$REPO_ROOT/target/release/ai-text-editor-mcp"
    if [ ! -x "$AI_TEXT_EDITOR_MCP_BIN" ]; then
        echo "run-case.sh: ai-text-editor-mcp binary not found at $AI_TEXT_EDITOR_MCP_BIN -- build it first (cargo build --release -p ai-text-editor-mcp)" >&2
        exit 69
    fi
    sed "s#__AI_TEXT_EDITOR_MCP_BIN__#$AI_TEXT_EDITOR_MCP_BIN#" \
        "$FRAMEWORK_DIR/mcp-config/ai-text-editor.json" > "$MCP_CONFIG"
    AGENT_COMMON_ARGV+=(--mcp-config "$MCP_CONFIG" --strict-mcp-config)
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
    AGENT_COMMON_ARGV+=(--strict-mcp-config --disallowedTools=Skill)
fi

EXTERNAL_EDIT_LOG="$CASE_DIR/external-edit.log"
EXTERNAL_FORMAT_LOG="$CASE_DIR/external-format.log"
: > "$EXTERNAL_EDIT_LOG"
: > "$EXTERNAL_FORMAT_LOG"
"$FRAMEWORK_DIR/external-edit.sh" "$WORKSPACE" "$EDIT_TIMEOUT" "$EXTERNAL_EDIT_LOG" &
EXTERNAL_EDIT_PID=$!
"$FRAMEWORK_DIR/external-format.sh" "$WORKSPACE" "$REFORMAT_TIMEOUT" "$EXTERNAL_FORMAT_LOG" &
EXTERNAL_FORMAT_PID=$!

if [ "$LAUNCH_MODE" = print ]; then
    # bypassPermissions, not acceptEdits: acceptEdits only pre-approves file
    # edits, not Bash -- confirmed across every earlier run, both workers
    # correctly diagnosed the TAX_RATE collision and tried to run the
    # shipped test, but every `python3 test_task_source.py` call sat
    # pending approval that a headless run can never answer, so the
    # test-resolution dimension was never actually exercised in print mode
    # (lab mode below exercises it for real). Each run's workspace is a
    # disposable copy under results/<run-id>/<condition>/workspace, never
    # the live repo, so bypassing the prompt here is scoped appropriately.
    AGENT_ARGV=("${AGENT_COMMON_ARGV[@]}" -p --output-format=json --permission-mode bypassPermissions)
    PROMPT="$(cat "$PROMPT_FILE")"
    AGENT_ARGV+=("$PROMPT")

    echo "run-case.sh: launching $CONDITION worker (model=$MODEL, levers=$LEVERS, launch=print)" >&2
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
else
    # lab mode -- see LAB-PROTOCOL.md for the full procedure this launches.
    SESSION_UUID="$(uuidgen 2>/dev/null || python3 -c 'import uuid; print(uuid.uuid4())')"
    REGISTRY="$RESULTS_BASE/subjects-registry.jsonl"
    mkdir -p "$RESULTS_BASE"
    EXISTING_SUBJECTS=0
    if [ -f "$REGISTRY" ]; then
        EXISTING_SUBJECTS="$(wc -l < "$REGISTRY" | tr -d ' ')"
    fi
    SUBJECT_LABEL="programmer$((EXISTING_SUBJECTS + 1))"

    # This whole benchmark normally runs AS a Claude Code session, so the
    # shell run-case.sh itself executes in already carries identity env vars
    # for THIS session -- confirmed present here: CLAUDE_CODE_CHILD_SESSION,
    # CLAUDE_CODE_SESSION_ID, CLAUDE_CODE_SESSION_ATTENDED,
    # CLAUDE_CODE_BRIDGE_SESSION_ID, CLAUDE_CODE_MESSAGING_SOCKET,
    # CLAUDE_CODE_MESSAGING_TOKEN. These are inherited by the experimenter
    # and, through it, by the subject's own nested `claude` process. The
    # experimenter itself is unaffected (its `-p` session id/transcript has
    # been reliable in every run so far regardless), but the subject's
    # first live run reported "Transcript saving is off" in its own status
    # bar (captured verbatim in that run's `session_rename_evidence`) and
    # its transcript was never resolvable under any id by `--resume` --
    # consistent with CLAUDE_CODE_CHILD_SESSION=1 specifically marking it as
    # a child session Claude Code intentionally does not persist, to avoid
    # exactly the nested-transcript collisions the orchestrator-driven
    # attempt hit. The earlier investigation tried unsetting only
    # CLAUDE_CODE_SESSION_ID/CLAUDE_CODE_SESSION_ATTENDED (and found that
    # made things worse) -- CLAUDE_CODE_CHILD_SESSION itself was never
    # tried. `env -u` strips all of these for the subject process alone,
    # between `interactive-shell` and `claude`, so the subject looks like a
    # genuinely independent top-level session rather than inheriting this
    # orchestrator's own identity. This is this attempt's hypothesis, not a
    # confirmed fix -- validate against a run's resume-probe*.json before
    # trusting it.
    LAB_SUBJECT_ARGV=(env -u CLAUDE_CODE_CHILD_SESSION -u CLAUDE_CODE_SESSION_ID \
        -u CLAUDE_CODE_SESSION_ATTENDED -u CLAUDE_CODE_BRIDGE_SESSION_ID \
        -u CLAUDE_CODE_MESSAGING_SOCKET -u CLAUDE_CODE_MESSAGING_TOKEN \
        "${AGENT_COMMON_ARGV[@]}" --session-id "$SESSION_UUID" --permission-mode acceptEdits)
    SUBJECT_ARGV_STR="${LAB_SUBJECT_ARGV[*]}"

    resolve_interactive_shell_bin_dir() {
        local shared="${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills/bin"
        if [ -x "$shared/interactive-shell" ]; then
            printf '%s\n' "$shared"
            return 0
        fi
        local dev
        dev="$(find "$REPO_ROOT/bin" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | head -n1)"
        if [ -n "$dev" ] && [ -x "$dev/interactive-shell" ]; then
            printf '%s\n' "$dev"
            return 0
        fi
        return 1
    }
    if ! INTERACTIVE_SHELL_BIN_DIR="$(resolve_interactive_shell_bin_dir)"; then
        echo "run-case.sh: interactive-shell binaries not found (checked \${XDG_CONFIG_HOME:-\$HOME/.config}/tsch-ai-skills/bin and $REPO_ROOT/bin/*) -- install the interactive-shell skill or run setup-dev-env.sh first" >&2
        exit 69
    fi
    export PATH="$INTERACTIVE_SHELL_BIN_DIR:$PATH"

    EXPERIMENTER_PROMPT_FILE="$CASE_DIR/experimenter-prompt.md"
    BUNDLE_OUT="$CASE_DIR/lab-bundle.json"
    python3 "$FRAMEWORK_DIR/build-experimenter-prompt.py" \
        "$FRAMEWORK_DIR/experimenter-prompt.template.md" "$EXPERIMENTER_PROMPT_FILE" \
        "SUBJECT_LABEL=$SUBJECT_LABEL" \
        "SESSION_UUID=$SESSION_UUID" \
        "WORKSPACE=$WORKSPACE" \
        "SUBJECT_ARGV=$SUBJECT_ARGV_STR" \
        "EXTERNAL_EDIT_LOG=$EXTERNAL_EDIT_LOG" \
        "EXTERNAL_FORMAT_LOG=$EXTERNAL_FORMAT_LOG" \
        "BUNDLE_OUT=$BUNDLE_OUT" \
        "DEBRIEF_SCRIPT_PATH=$FRAMEWORK_DIR/debrief-script.md" \
        "TASK_PROMPT=@$PROMPT_FILE"

    EXPERIMENTER_MODEL="${AI_TEXT_EDITOR_BENCH_EXPERIMENTER_MODEL:-haiku}"
    EXPERIMENTER_PROMPT_TEXT="$(cat "$EXPERIMENTER_PROMPT_FILE")"

    echo "run-case.sh: launching $CONDITION lab run (subject=$SUBJECT_LABEL, model=$MODEL, experimenter=$EXPERIMENTER_MODEL, levers=$LEVERS, launch=lab)" >&2
    START_EPOCH="$(date -u +%s.%N)"
    set +e
    # --add-dir takes a variadic directory list and only stops consuming at
    # the next `--flag` token (same trap --disallowedTools has, noted
    # above) -- it must never be the token immediately before the bare
    # prompt text, or the whole prompt is silently swallowed as a second
    # directory and claude -p fails with "Input must be provided either
    # through stdin or as a prompt argument" (confirmed: this exact ordering
    # bug was caught by this script's own first live lab-mode run). Keep
    # --add-dir followed by another flag, exactly like AGENT_COMMON_ARGV's
    # own ordering above, and the bare prompt last, after a single-value
    # flag (--permission-mode) that cannot swallow it.
    ( cd "$WORKSPACE" && claude --add-dir "$WORKSPACE" --model "$EXPERIMENTER_MODEL" \
        -p --output-format=json --permission-mode bypassPermissions "$EXPERIMENTER_PROMPT_TEXT" ) \
        >"$CASE_DIR/experimenter-stdout.json" 2>"$CASE_DIR/experimenter-stderr.log"
    WORKER_EXIT=$?
    set -e
    END_EPOCH="$(date -u +%s.%N)"
    WALL_SECONDS="$(awk -v a="$START_EPOCH" -v b="$END_EPOCH" 'BEGIN{printf "%.3f", b-a}')"

    wait "$EXTERNAL_EDIT_PID" || true
    wait "$EXTERNAL_FORMAT_PID" || true

    SESSION_ID="$SESSION_UUID"
    echo "$SESSION_ID" > "$CASE_DIR/session-id.txt"

    # The bundle write is the experimenter's own last action before its
    # process exits, but this script's `$?` for the subshell above has been
    # observed (first live lab-mode run) to become available a few seconds
    # before experimenter-stdout.json / lab-bundle.json are actually fully
    # flushed to disk -- the same class of readiness gap external-edit.sh's
    # own polling exists to cover, just on the experimenter's own exit
    # rather than the subject's checkpoint files. Poll briefly rather than
    # trusting mere existence; a ceiling, not a sleep.
    BUNDLE_SETTLE_DEADLINE=$(( $(date +%s) + 15 ))
    while [ "$(date +%s)" -lt "$BUNDLE_SETTLE_DEADLINE" ]; do
        if [ -s "$BUNDLE_OUT" ] && python3 -c "import json,sys; json.load(open(sys.argv[1]))" "$BUNDLE_OUT" 2>/dev/null \
            && [ -s "$CASE_DIR/experimenter-stdout.json" ] && python3 -c "import json,sys; json.load(open(sys.argv[1]))" "$CASE_DIR/experimenter-stdout.json" 2>/dev/null; then
            break
        fi
        sleep 1
    done

    EXPERIMENTER_SESSION_ID="$(sed -nE 's/.*"session_id"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' "$CASE_DIR/experimenter-stdout.json" 2>/dev/null | sed -n '1p')"
    echo "${EXPERIMENTER_SESSION_ID:-}" > "$CASE_DIR/experimenter-session-id.txt"

    if [ -f "$BUNDLE_OUT" ]; then
        python3 "$FRAMEWORK_DIR/synthesize-worker-stdout.py" "$BUNDLE_OUT" "$CASE_DIR/worker-stdout.json"
    else
        echo "run-case.sh: WARNING: experimenter did not write a bundle at $BUNDLE_OUT" >&2
        echo '{"result": "", "session_id": ""}' > "$CASE_DIR/worker-stdout.json"
    fi

    STARTED_AT_UTC="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    python3 "$FRAMEWORK_DIR/append-lab-registry.py" "$REGISTRY" "$SUBJECT_LABEL" "$SESSION_UUID" \
        "${EXPERIMENTER_SESSION_ID:-}" "$RUN_ID" "$CONDITION" "$LEVERS" "$WORKSPACE" "$STARTED_AT_UTC" "$MODEL"

    # The actual, empirical test of resumability -- not an assumption. See
    # write-resume-probe.py's own doc comment for why 80000 is the probe.
    # Two attempts: by the pinned --session-id (the first live run found
    # this unresolvable), and by the /rename'd label itself, since
    # `claude --resume --help` says a bare value can be "a session ID, or
    # ... a search term" -- the label is this mode's attempted mitigation
    # for the id not being reliably honored on this launch shape.
    RESUME_QUESTION="In one short sentence: what numeric budget did you write for the Support department in departments.csv?"

    RESUME_PROBE_FILE="$CASE_DIR/resume-probe.json"
    RESUME_STDOUT="$CASE_DIR/resume-probe-stdout.json"
    set +e
    claude -p --resume "$SESSION_UUID" --output-format=json "$RESUME_QUESTION" \
        >"$RESUME_STDOUT" 2>"$CASE_DIR/resume-probe-stderr.log"
    RESUME_EXIT=$?
    set -e
    python3 "$FRAMEWORK_DIR/write-resume-probe.py" "$RESUME_STDOUT" "$RESUME_EXIT" "$RESUME_PROBE_FILE"

    RESUME_PROBE_BY_LABEL_FILE="$CASE_DIR/resume-probe-by-label.json"
    RESUME_BY_LABEL_STDOUT="$CASE_DIR/resume-probe-by-label-stdout.json"
    set +e
    claude -p --resume "$SUBJECT_LABEL" --output-format=json "$RESUME_QUESTION" \
        >"$RESUME_BY_LABEL_STDOUT" 2>"$CASE_DIR/resume-probe-by-label-stderr.log"
    RESUME_BY_LABEL_EXIT=$?
    set -e
    python3 "$FRAMEWORK_DIR/write-resume-probe.py" "$RESUME_BY_LABEL_STDOUT" "$RESUME_BY_LABEL_EXIT" "$RESUME_PROBE_BY_LABEL_FILE"
fi

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
        python3 "$FRAMEWORK_DIR/extract_edit_timeline.py" "$TRANSCRIPT" "$EXTERNAL_EDIT_LOG" "$EXTERNAL_FORMAT_LOG" \
            > "$CASE_DIR/edit-timeline.json" || echo '{}' > "$CASE_DIR/edit-timeline.json"
    else
        echo '{}' > "$CASE_DIR/telemetry.json"
        echo '{}' > "$CASE_DIR/edit-timeline.json"
    fi
else
    echo '{}' > "$CASE_DIR/telemetry.json"
    echo '{}' > "$CASE_DIR/edit-timeline.json"
fi

python3 "$FRAMEWORK_DIR/score.py" "$WORKSPACE" > "$CASE_DIR/score.json" || echo '{}' > "$CASE_DIR/score.json"

cat > "$CASE_DIR/evaluation.md" <<EOF
# ai-text-editor-usage: $CONDITION ($RUN_ID)

- Model: $MODEL
- Launch mode: $LAUNCH_MODE
- Prompt file: $PROMPT_FILE (identical across every condition)
- Levers on: $LEVERS (mcp = MCP instructions pitch, edit/read = editor-gate-plugin's hooks)
- Worker/experimenter exit code: $WORKER_EXIT
- Wall clock: ${WALL_SECONDS}s
- Session id: ${SESSION_ID:-(none parsed)}
- External edit logs: see external-edit.log (TAX_RATE bump), external-format.log (reindent)
- Telemetry: see telemetry.txt / telemetry.json
- Edit mechanism timeline (internal vs mcp, switches, disk-change correlation): see edit-timeline.json
- Score: see score.json
EOF

if [ "$LAUNCH_MODE" = lab ]; then
    cat >> "$CASE_DIR/evaluation.md" <<EOF

## Lab mode data (see LAB-PROTOCOL.md for the full procedure)

- Subject label: $SUBJECT_LABEL (registry: $REGISTRY)
- Subject session id (pinned): $SESSION_UUID
- Experimenter session id: ${EXPERIMENTER_SESSION_ID:-(none parsed)}
- Experimenter bundle: see lab-bundle.json (includes \`session_rename_confirmed\` / \`session_rename_evidence\` for the \`/rename $SUBJECT_LABEL\` step)
- Resume probe by pinned session id (\`claude --resume $SESSION_UUID\`): see resume-probe.json
- Resume probe by label (\`claude --resume $SUBJECT_LABEL\`): see resume-probe-by-label.json
EOF
fi

echo "run-case.sh: $CONDITION done, exit=$WORKER_EXIT, wall=${WALL_SECONDS}s, session=${SESSION_ID:-none}" >&2
