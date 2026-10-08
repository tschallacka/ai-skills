"""Reindents a Python file in place: every line whose leading run of plain
spaces is a non-zero multiple of 3 is rewritten at 4 spaces per level
instead of 3 (N -> N // 3 * 4). A line with no leading spaces, or a leading
run that isn't a multiple of 3 (an edit already landed at 4-space width,
or it's inside something this blunt a pass shouldn't touch), is left
untouched.

This is the SECOND, broad external-edit collision surface for the
ai-text-editor-usage benchmark, alongside external-edit.sh's single-line
TAX_RATE bump: it touches essentially every indented line in the file at
once, the way a real editor's format-on-save (PhpStorm, VS Code) rewrites
a file a teammate has open while someone else edits it concurrently --
not a hypothetical, just a second, differently-shaped version of the same
everyday collision.

Usage: external_reformat.py <path>
"""

import re
import sys


def reindent(text):
    out = []
    for line in text.splitlines(keepends=True):
        match = re.match(r"^( *)(.*)$", line, re.DOTALL)
        leading, rest = match.group(1), match.group(2)
        if leading and len(leading) % 3 == 0:
            new_leading = " " * (len(leading) // 3 * 4)
            out.append(new_leading + rest)
        else:
            out.append(line)
    return "".join(out)


def main():
    if len(sys.argv) != 2:
        print("Usage: external_reformat.py <path>", file=sys.stderr)
        return 64

    path = sys.argv[1]
    with open(path, encoding="utf-8") as handle:
        text = handle.read()

    new_text = reindent(text)
    if new_text == text:
        print(f"external_reformat: {path} already matches 4-space indent, no change")
        return 0

    with open(path, "w", encoding="utf-8") as handle:
        handle.write(new_text)
    print(f"external_reformat: reindented {path} from 3-space to 4-space levels")
    return 0


if __name__ == "__main__":
    sys.exit(main())
