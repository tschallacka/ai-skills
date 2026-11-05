#!/usr/bin/env python3
"""Writes the lab-mode resume-probe.json from a `claude --resume` attempt.

This is the empirical test of whether a lab subject's pinned --session-id
can actually be reached again later with `claude --resume <id>` -- not an
assumption. A failure here is exactly as informative as a success: it says
whether LAB-PROTOCOL.md's registry (results/subjects-registry.jsonl) is
actually sufficient to "interrogate programmer3 later", or whether that
still runs into the same transcript-attribution limitation this mode was
built to avoid needing in the first place.

Usage: write-resume-probe.py RESUME_STDOUT_PATH EXIT_CODE OUT_PATH
The probe asks the resumed session a question whose correct answer (80000,
the Support department's budget from departments.csv) only a genuine
continuation of that exact session could produce.
"""
import json
import sys


def main():
    if len(sys.argv) != 4:
        print("usage: write-resume-probe.py RESUME_STDOUT_PATH EXIT_CODE OUT_PATH", file=sys.stderr)
        return 64
    stdout_path, exit_code_str, out_path = sys.argv[1], sys.argv[2], sys.argv[3]

    raw_result = None
    try:
        with open(stdout_path) as f:
            data = json.load(f)
        raw_result = data.get("result")
    except (OSError, json.JSONDecodeError):
        pass

    exit_code = int(exit_code_str)
    succeeded_heuristic = bool(raw_result) and "80000" in raw_result

    report = {
        "attempted": True,
        "exit_code": exit_code,
        "raw_result": raw_result,
        "resume_succeeded_heuristic": succeeded_heuristic,
        "note": (
            "heuristic only: checks whether the resumed session's answer "
            "mentions 80000 (the Support department's budget it itself "
            "wrote to departments.csv), which only a genuine continuation "
            "of that exact session could reproduce"
        ),
    }
    with open(out_path, "w") as f:
        json.dump(report, f, indent=2)
    return 0


if __name__ == "__main__":
    sys.exit(main())
