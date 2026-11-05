#!/usr/bin/env python3
"""Fills {{PLACEHOLDER}} tokens in experimenter-prompt.template.md.

Usage: build-experimenter-prompt.py TEMPLATE OUTPUT KEY=VALUE [KEY=VALUE ...]

A VALUE beginning with "@" is read from that file path instead of being
used literally -- the one lab-mode placeholder that needs this is
TASK_PROMPT=@task-prompt.md, since that value is multi-line and sed cannot
substitute a multi-line replacement without separately escaping every
newline and ampersand in it.
"""
import sys
from pathlib import Path


def main():
    if len(sys.argv) < 3:
        print("usage: build-experimenter-prompt.py TEMPLATE OUTPUT KEY=VALUE [...]", file=sys.stderr)
        return 64
    template_path, output_path = sys.argv[1], sys.argv[2]
    text = Path(template_path).read_text()
    for pair in sys.argv[3:]:
        key, sep, value = pair.partition("=")
        if sep != "=":
            print(f"build-experimenter-prompt.py: malformed KEY=VALUE pair: {pair}", file=sys.stderr)
            return 64
        if value.startswith("@"):
            value = Path(value[1:]).read_text()
        token = "{{" + key + "}}"
        if token not in text:
            print(f"build-experimenter-prompt.py: warning: {token} not found in template", file=sys.stderr)
        text = text.replace(token, value)
    Path(output_path).write_text(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
