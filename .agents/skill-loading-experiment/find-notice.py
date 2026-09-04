#!/usr/bin/env python3
# MODE: DEV
"""Search a transcript for any truncation or size notice, anywhere in it.

The point is to distinguish a genuinely silent truncation from one whose
notice merely sat somewhere the tool_result parser did not look -- a
system-reminder, a separate event, an appended footer.
"""

from __future__ import annotations

import sys

NEEDLES = (
    "exceeds",
    "truncat",
    "maximum allowed",
    "system-reminder",
    "limit param",
    "read the rest",
    "remaining lines",
)


def main() -> None:
    for path in sys.argv[1:]:
        raw = open(path, encoding="utf-8").read()
        lowered = raw.lower()
        print(f"== {path} ({len(raw)} chars)")
        for needle in NEEDLES:
            index = lowered.find(needle)
            if index < 0:
                print(f"   {needle!r}: absent")
                continue
            print(f"   {needle!r}: at {index}")
            print(f"      {raw[max(0, index - 180):index + 240]!r}")


if __name__ == "__main__":
    main()
