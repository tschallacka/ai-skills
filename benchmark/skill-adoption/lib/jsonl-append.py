#!/usr/bin/env python3
"""Appends one line from stdin to PATH under an exclusive flock.

Usage: jsonl-append.py PATH

Concurrent cells in a full pass (full-pass.sh) all append to the same
results/registry.jsonl; a long observer_notes field can push a line past
PIPE_BUF (4096 bytes on Linux), where plain O_APPEND writes stop being
atomic and two cells' lines can interleave. The flock here serializes the
append so that never happens -- see record-run.py for runs.jsonl's own
in-process equivalent.
"""
import fcntl
import sys


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: jsonl-append.py PATH")
    path = sys.argv[1]
    line = sys.stdin.readline()
    if not line:
        sys.exit("jsonl-append.py: no input on stdin")
    if not line.endswith("\n"):
        line += "\n"
    with open(path, "a", encoding="utf-8") as f:
        fcntl.flock(f, fcntl.LOCK_EX)
        f.write(line)
        fcntl.flock(f, fcntl.LOCK_UN)


if __name__ == "__main__":
    main()
