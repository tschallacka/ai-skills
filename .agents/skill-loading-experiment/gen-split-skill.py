#!/usr/bin/env python3
# MODE: DEV
"""Build the split skill structures under test.

Three layouts, all from the same body of content so the comparison is fair:

  imperative - a main file that orders each part read now, no proof required
  index      - a short main file that merely lists the parts
  verify     - the self-verifying load: every part ends with a LOAD TOKEN and
               the main file requires all of them echoed back before the skill
               may be used

The LOAD TOKEN sits on the LAST line of each part on purpose. A token at the
top would be echoed by an agent that read only the opening of the part, which
would make the mechanism prove nothing; on the last line, echoing it means the
part was delivered to its end.

Emits, alongside the files, a manifest of deep content markers and a manifest
of load tokens, so both completeness and recall can be scored.
"""

from __future__ import annotations

import argparse
import os
import secrets

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


def build_part(index: int, lines: int, marker_at: int) -> tuple[str, str, str]:
    """Return (text, content marker token, load token) for one part."""
    marker = secrets.token_hex(6).upper()
    load = secrets.token_hex(6).upper()
    body = [f"# Part {index}"]
    for number in range(2, lines):
        if number == marker_at:
            body.append(f"VERIFICATION TOKEN: {marker}")
        elif number % 40 == 0:
            body.append(f"## Part {index} section {number // 40}")
        else:
            body.append(f"- {FILLER[number % len(FILLER)]}")
    body.append(f"LOAD TOKEN: {load}")
    return "\n".join(body) + "\n", marker, load


MAIN = {
    "imperative": """# Synthetic Skill Under Test

This skill is split across {count} part files. Read every one of them NOW,
in full, before doing anything else. Do not skip a part and do not stop
early.

{listing}
""",
    "index": """# Synthetic Skill Under Test

This skill's content lives in the part files below.

{listing}
""",
    "verify": """# Synthetic Skill Under Test

This skill is split across {count} part files. Read every one of them NOW, in
full, before doing anything else.

{listing}

## Mandatory load check

The last line of each part file is a `LOAD TOKEN:`. You cannot see a part's
load token unless that part reached you complete, so echoing them is the proof
that the skill loaded.

Before you answer anything else, print one line per part, exactly:

    <part filename> <its LOAD TOKEN>

If a part's load token is not in your context, print `<part filename> MISSING`
instead of guessing. A missing token means the skill did not load and you must
say so rather than proceeding.
""",
}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--outdir", required=True)
    parser.add_argument("--parts", type=int, default=5)
    parser.add_argument("--part-lines", type=int, default=520)
    # Fault injection: make one part far too big for the harness caps, so
    # its final-line LOAD TOKEN cannot arrive. The question each structure
    # then answers is whether the loss is reported or passes unnoticed.
    parser.add_argument("--fat-part", type=int, default=0)
    parser.add_argument("--fat-lines", type=int, default=2600)
    parser.add_argument(
        "--structure", choices=sorted(MAIN), required=True
    )
    args = parser.parse_args()

    os.makedirs(args.outdir, exist_ok=True)
    names = []
    markers = {}
    loads = {}
    for index in range(1, args.parts + 1):
        name = f"part-{index}.md"
        part_lines = args.fat_lines if index == args.fat_part else args.part_lines
        text, marker, load = build_part(
            index, part_lines, marker_at=part_lines - 30
        )
        with open(os.path.join(args.outdir, name), "w", encoding="utf-8") as handle:
            handle.write(text)
        names.append(name)
        markers[name] = marker
        loads[name] = load

    listing = "\n".join(f"- `{name}` - read this file now, in full." for name in names)
    with open(os.path.join(args.outdir, "SKILL.md"), "w", encoding="utf-8") as handle:
        handle.write(
            MAIN[args.structure].format(count=args.parts, listing=listing)
        )

    with open(os.path.join(args.outdir, "markers.tsv"), "w", encoding="utf-8") as handle:
        handle.write("part\ttoken\n")
        for name in names:
            handle.write(f"{name}\t{markers[name]}\n")
    with open(os.path.join(args.outdir, "loads.tsv"), "w", encoding="utf-8") as handle:
        handle.write("part\ttoken\n")
        for name in names:
            handle.write(f"{name}\t{loads[name]}\n")
    print(f"{args.outdir}: {args.structure}, {args.parts} parts")


if __name__ == "__main__":
    main()
