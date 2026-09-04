#!/usr/bin/env python3
# MODE: DEV
"""Generate a synthetic skill-shaped markdown file with high-entropy markers.

Markers are random hex tokens placed at exact 1-based line numbers, so a
recall answer can be scored against a manifest. Token text carries no hint of
its line number on purpose: a guessable name like MARKER_400 would let a model
answer without ever having seen the file.

Usage:
    gen-marker-file.py --lines 1600 --out FILE --manifest FILE.tsv \
        --marker 50 --marker 250 ...
"""

from __future__ import annotations

import argparse
import secrets

# Filler sentences shaped like real skill prose, so the file's token density
# and structure resemble planning/SKILL.md rather than random noise.
FILLER = [
    "Record the decision in the plan document before starting the work unit.",
    "The verification command must fail before the fix and pass after it.",
    "Do not mark a goal complete while any of its steps remain unverified.",
    "A handoff note names the next reader, not the previous author.",
    "Prefer an explicit refusal over a silent partial success.",
    "State what was measured and what was merely assumed.",
    "Every gate reports the check it ran, never a bare success line.",
    "Keep the progress tracker in the same commit as the change it tracks.",
]


def build(
    total_lines: int, markers: list[int], width: int = 0
) -> tuple[list[str], dict[int, str]]:
    marker_set = set(markers)
    manifest: dict[int, str] = {}
    lines: list[str] = []
    for number in range(1, total_lines + 1):
        if width and number not in marker_set and number != 1:
            # Fixed-width filler, for telling a line cap apart from a byte cap:
            # the same line count at two widths must truncate at the same line
            # if the cap counts lines, and at different lines if it counts bytes.
            body = " ".join(FILLER)
            lines.append(("- " + body * 10)[:width])
            continue
        if number in marker_set:
            token = secrets.token_hex(6).upper()
            manifest[number] = token
            lines.append(f"VERIFICATION TOKEN: {token}")
        elif number == 1:
            lines.append("# Synthetic Skill Under Test")
        elif number % 40 == 0:
            lines.append(f"## Section {number // 40}")
        elif number % 40 == 1:
            lines.append("")
        else:
            lines.append(f"- {FILLER[number % len(FILLER)]}")
    return lines, manifest


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--lines", type=int, required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--manifest", required=True)
    parser.add_argument("--marker", type=int, action="append", default=[])
    parser.add_argument("--width", type=int, default=0, help="fixed filler width")
    args = parser.parse_args()

    for marker in args.marker:
        if not 1 <= marker <= args.lines:
            raise SystemExit(f"marker {marker} outside 1..{args.lines}")

    lines, manifest = build(args.lines, args.marker, args.width)
    with open(args.out, "w", encoding="utf-8") as handle:
        handle.write("\n".join(lines) + "\n")
    with open(args.manifest, "w", encoding="utf-8") as handle:
        handle.write("line\ttoken\n")
        for number in sorted(manifest):
            handle.write(f"{number}\t{manifest[number]}\n")
    print(f"{args.out}: {args.lines} lines, {len(manifest)} markers")


if __name__ == "__main__":
    main()
