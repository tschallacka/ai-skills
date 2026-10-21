#!/usr/bin/env bash
# run-cell.sh -- RETIRED. Do not use.
#
# This script's own experimenter step shelled out to `claude -p --model
# haiku ...` as a detached subprocess. That is the design a full pass on
# 2026-10-19 lost roughly half its cells to (a silent external kill of the
# whole `claude -p` process tree, no error text anywhere) -- `claude -p`
# must never be invoked as the experimenter again; see PROTOCOL.md's "Why
# the experimenter is a real subagent, not a prompt template". Use
# `prepare-cell.sh` + an Agent-tool subagent (model haiku) driven by the
# orchestrating session itself + `finish-cell.sh` instead -- see
# PROTOCOL.md's "Orchestrating a full pass".
set -euo pipefail
echo "run-cell.sh: retired -- it invokes claude -p as the experimenter, which" >&2
echo "  must never be done (PROTOCOL.md). Use prepare-cell.sh + an Agent-tool" >&2
echo "  subagent + finish-cell.sh instead." >&2
exit 1
