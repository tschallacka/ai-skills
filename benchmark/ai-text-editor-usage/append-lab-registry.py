#!/usr/bin/env python3
"""Appends one subject entry to results/subjects-registry.jsonl (lab mode).

Usage: append-lab-registry.py REGISTRY_PATH LABEL SESSION_ID EXPERIMENTER_SESSION_ID \
           LAB_RUN_ID CONDITION LEVERS WORKSPACE STARTED_AT_UTC MODEL

The registry is the durable index lab mode exists to produce: to
interrogate "programmer3" later, look its session_id up here and
`claude --resume <session_id>`. One JSON object per line, append-only.

session_id is the --session-id this run REQUESTED for the subject -- not
confirmed resumable (the first live run found neither the requested id nor
the id the experimenter observed on screen ever resolved to a findable
transcript; see LAB-PROTOCOL.md's "Known limitations"). experimenter_session_id
is the one id this process can actually confirm works, because the
experimenter itself always launches via `claude -p`, the same reliable
mechanism print mode has always used -- resuming it reaches an agent that
watched the subject's whole session and can be interrogated about what it
saw, even when the subject's own session cannot be reached directly.
"""
import json
import sys


def main():
    if len(sys.argv) != 11:
        print(
            "usage: append-lab-registry.py REGISTRY_PATH LABEL SESSION_ID "
            "EXPERIMENTER_SESSION_ID LAB_RUN_ID CONDITION LEVERS WORKSPACE "
            "STARTED_AT_UTC MODEL",
            file=sys.stderr,
        )
        return 64
    (
        registry_path,
        label,
        session_id,
        experimenter_session_id,
        lab_run_id,
        condition,
        levers,
        workspace,
        started_at_utc,
        model,
    ) = sys.argv[1:11]

    entry = {
        "label": label,
        "session_id": session_id,
        "experimenter_session_id": experimenter_session_id,
        "lab_run_id": lab_run_id,
        "condition": condition,
        "levers": levers,
        "workspace": workspace,
        "started_at_utc": started_at_utc,
        "model": model,
    }
    with open(registry_path, "a") as f:
        f.write(json.dumps(entry) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
