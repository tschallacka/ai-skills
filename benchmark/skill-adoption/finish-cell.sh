#!/usr/bin/env bash
# finish-cell.sh WORK_DIR
#
# Second half of one cell (see prepare-cell.sh). Expects the experimenter
# -- a real Agent-tool subagent, never `claude -p` -- to have already
# written WORK_DIR/bundle.json itself (make-experimenter-brief.py's own
# Step 6 instructs this directly, rather than this script extracting a
# bundle from a `claude -p --output-format=json` envelope, which no longer
# exists). Validates and records it, then removes WORK_DIR.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/common.sh
. "$HERE/lib/common.sh"

WORK="${1:-}"
if [ -z "$WORK" ] || [ ! -d "$WORK" ]; then
    echo "usage: finish-cell.sh WORK_DIR" >&2
    exit 64
fi

META_FILE="$WORK/meta.json"
if [ ! -f "$META_FILE" ]; then
    echo "finish-cell.sh: no meta.json in $WORK -- was this prepared by prepare-cell.sh?" >&2
    exit 66
fi

RUN_ID="$(python3 -c "import json; print(json.load(open('$META_FILE'))['run_id'])")"
SCENARIO_ID="$(python3 -c "import json; print(json.load(open('$META_FILE'))['scenario_id'])")"
CLI="$(python3 -c "import json; print(json.load(open('$META_FILE'))['cli'])")"
MODE="$(python3 -c "import json; print(json.load(open('$META_FILE'))['mode'])")"
REP="$(python3 -c "import json; print(json.load(open('$META_FILE'))['rep'])")"
SUBJECT_UUID="$(python3 -c "import json; print(json.load(open('$META_FILE'))['subject_session_id'])")"
STARTED_AT="$(python3 -c "import json; print(json.load(open('$META_FILE'))['started_at_utc'])")"

RESULTS_DIR="$HERE/results"
ENV_FILE="$WORK/environment.json"
BUNDLE_FILE="$WORK/bundle.json"
TRANSCRIPT_FILE="$WORK/project/.transcript.txt"

if [ ! -f "$BUNDLE_FILE" ]; then
    echo "finish-cell.sh: no bundle.json in $WORK -- the experimenter never wrote one" >&2
    exit 1
fi
if [ ! -f "$TRANSCRIPT_FILE" ]; then
    echo "(no .transcript.txt was written by the experimenter)" > "$TRANSCRIPT_FILE"
fi

python3 "$HERE/record-run.py" "$RESULTS_DIR" "$BUNDLE_FILE" "$ENV_FILE" "$TRANSCRIPT_FILE" "$REP"

FINISHED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
python3 -c "
import json
print(json.dumps({
    'run_id': '$RUN_ID', 'scenario_id': '$SCENARIO_ID', 'cli': '$CLI',
    'mode': '$MODE', 'rep': $REP, 'subject_label': '$RUN_ID',
    'subject_session_id': '$SUBJECT_UUID', 'experimenter_session_id': None,
    'workspace': '$WORK', 'started_at_utc': '$STARTED_AT',
    'finished_at_utc': '$FINISHED_AT', 'outcome': 'completed',
}))
" | python3 "$HERE/lib/jsonl-append.py" "$RESULTS_DIR/registry.jsonl"

rm -rf "$WORK"

echo "finish-cell.sh: done ($RUN_ID)" >&2
