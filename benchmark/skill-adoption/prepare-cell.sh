#!/usr/bin/env bash
# prepare-cell.sh SCENARIO_ID CLI MODE [REP]
#
# First half of one cell, per PROTOCOL.md: builds a disposable workspace,
# runs the scenario's setup, installs the full skill collection into a
# disposable HOME, and writes the experimenter's brief -- everything
# run-cell.sh used to do before handing off to a `claude -p` subprocess.
#
# That handoff is gone: claude -p must never be invoked as the experimenter
# (nested headless claude invocations were the likely cause of the mass
# silent-kill failures a `claude -p`-per-cell full pass hit on 2026-10-19 --
# see PROTOCOL.md). The experimenter is now a real Agent-tool subagent
# (model haiku) driven directly by the orchestrating session, reading the
# brief this script writes and following it verbatim. finish-cell.sh is the
# second half, run once that subagent has written WORK/bundle.json itself.
#
# Prints one JSON line to stdout: {"work_dir", "project_dir", "brief_file",
# "env_file", "run_id", "subject_session_id", "scenario_id", "cli", "mode",
# "rep", "started_at_utc"}. The same object (plus `outcome`) is also what
# meta.json holds inside work_dir, for finish-cell.sh to read back.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$HERE/../.." && pwd)"
# shellcheck source=lib/common.sh
. "$HERE/lib/common.sh"

SCENARIO_ID="${1:-}"
CLI="${2:-}"
MODE="${3:-}"
REP="${4:-1}"

if [ -z "$SCENARIO_ID" ] || [ -z "$CLI" ] || [ -z "$MODE" ]; then
    echo "usage: prepare-cell.sh SCENARIO_ID CLI MODE [REP]" >&2
    echo "  SCENARIO_ID  e.g. S04-decisions" >&2
    echo "  CLI          claude | opencode" >&2
    echo "  MODE         skill | mcp | n/a" >&2
    exit 64
fi

SCENARIO_DIR="$HERE/scenarios/$SCENARIO_ID"
SCENARIO_JSON="$SCENARIO_DIR/scenario.json"
if [ ! -f "$SCENARIO_JSON" ]; then
    echo "prepare-cell.sh: no such scenario: $SCENARIO_JSON" >&2
    exit 66
fi

RESULTS_DIR="$HERE/results"
mkdir -p "$RESULTS_DIR"

RUN_ID="$(python3 -c 'import uuid; print(uuid.uuid4())')"
SUBJECT_UUID="$(python3 -c 'import uuid; print(uuid.uuid4())')"
STARTED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

echo "prepare-cell.sh: $SCENARIO_ID cli=$CLI mode=$MODE rep=$REP run_id=$RUN_ID" >&2

WORK="$(wa_new_workspace)"

# registry.jsonl placeholder line -- written before the subject exists, so
# a crash leaves a trace. See schema/registry-entry.schema.json.
python3 -c "
import json
print(json.dumps({
    'run_id': '$RUN_ID', 'scenario_id': '$SCENARIO_ID', 'cli': '$CLI',
    'mode': '$MODE', 'rep': $REP, 'subject_label': '$RUN_ID',
    'subject_session_id': '$SUBJECT_UUID', 'experimenter_session_id': None,
    'workspace': '$WORK', 'started_at_utc': '$STARTED_AT',
    'finished_at_utc': None, 'outcome': 'abandoned',
}))
" | python3 "$HERE/lib/jsonl-append.py" "$RESULTS_DIR/registry.jsonl"

# Resolve setup script (scenario.json's own "setup" field, relative to
# SCENARIO_DIR) and run it, if present.
SETUP_REL="$(python3 -c "
import json
d = json.load(open('$SCENARIO_JSON'))
print(d.get('setup', ''))
")"
# A scenario whose premise needs a tool genuinely absent (S09's jq) names it
# in requires_path_without; its setup step's own precondition check must see
# the same absence the subject will, which make-experimenter-brief.py
# arranges separately for the subject's own launch command.
REQUIRES_PATH_WITHOUT="$(python3 -c "
import json
d = json.load(open('$SCENARIO_JSON'))
print(' '.join(d.get('requires_path_without', [])))
")"
if [ -n "$SETUP_REL" ]; then
    if [ -n "$REQUIRES_PATH_WITHOUT" ]; then
        # shellcheck disable=SC2086  # bare space-separated binary names, meant to word-split
        SETUP_PATH="$(wa_path_without "$WORK/path-without-setup" $REQUIRES_PATH_WITHOUT)"
        PATH="$SETUP_PATH" "$SCENARIO_DIR/$SETUP_REL" setup "$WORK/project"
    else
        "$SCENARIO_DIR/$SETUP_REL" setup "$WORK/project"
    fi
fi

# Seed auth and (for claude) a real settings.json BEFORE installing: the
# installer's own permission writers refuse to touch a settings.json that
# does not exist yet (see wa_seed_agent_auth's doc comment in lib/common.sh),
# which a fresh disposable HOME never has until this seeds one.
wa_seed_agent_auth "$CLI" "$WORK/home"

# Install the full skill collection into the disposable HOME. n/a-mode
# scenarios (no dual-surface skill under test) still get a real install --
# mcp is the default, matching every mode-offering skill's own default.
INSTALL_MODE="$MODE"
if [ "$MODE" = "n/a" ]; then
    INSTALL_MODE="mcp"
fi
wa_install_skills "$REPO_ROOT" "$WORK/home" "$CLI" "$INSTALL_MODE"

ENV_FILE="$WORK/environment.json"
wa_capture_environment "$REPO_ROOT" "$CLI" "$WORK/home" > "$ENV_FILE"

BRIEF_FILE="$WORK/brief.txt"
python3 "$HERE/make-experimenter-brief.py" "$SCENARIO_JSON" "$CLI" "$MODE" \
    "$WORK/project" "$WORK/home" "$RUN_ID" "$SUBJECT_UUID" > "$BRIEF_FILE"

python3 -c "
import json
print(json.dumps({
    'run_id': '$RUN_ID', 'scenario_id': '$SCENARIO_ID', 'cli': '$CLI',
    'mode': '$MODE', 'rep': $REP, 'subject_session_id': '$SUBJECT_UUID',
    'work_dir': '$WORK', 'project_dir': '$WORK/project',
    'brief_file': '$BRIEF_FILE', 'env_file': '$ENV_FILE',
    'started_at_utc': '$STARTED_AT',
}))
" | tee "$WORK/meta.json"
