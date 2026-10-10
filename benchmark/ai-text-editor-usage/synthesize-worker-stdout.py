#!/usr/bin/env python3
"""Builds a worker-stdout.json-shaped file from a lab-mode bundle.

score.py's detect_feedback_request() reads <case_dir>/worker-stdout.json's
"result" field, written by `claude -p --output-format=json` in print mode.
Lab mode has no such process for the subject (it runs interactively), so
this produces an equivalent-enough stand-in from the experimenter's own
bundle's "final_summary" field, letting score.py run unmodified either way.

Usage: synthesize-worker-stdout.py BUNDLE_PATH OUT_PATH
Missing or unparseable BUNDLE_PATH writes an empty-result stand-in rather
than failing -- a scored "no summary captured" is more useful than no
score.json at all.
"""
import json
import sys


def main():
    if len(sys.argv) != 3:
        print("usage: synthesize-worker-stdout.py BUNDLE_PATH OUT_PATH", file=sys.stderr)
        return 64
    bundle_path, out_path = sys.argv[1], sys.argv[2]

    bundle = {}
    try:
        with open(bundle_path) as f:
            bundle = json.load(f)
    except (OSError, json.JSONDecodeError):
        pass

    stand_in = {
        "result": bundle.get("final_summary") or "",
        "session_id": bundle.get("subject_session_id") or "",
    }
    with open(out_path, "w") as f:
        json.dump(stand_in, f)
    return 0


if __name__ == "__main__":
    sys.exit(main())
