#!/usr/bin/env python3
# MODE: DEV
"""One TSV row summarising the Read traffic in a claude session transcript.

Columns: label, model, source lines, source bytes, number of Read calls,
total lines delivered, total chars delivered, and whether any tool result
carried a truncation notice.
"""

from __future__ import annotations

import json
import sys


def text_of(content) -> str:
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        return "\n".join(
            block.get("text") or "" for block in content if isinstance(block, dict)
        )
    return ""


def main() -> None:
    path, label, model, src_lines, src_bytes = sys.argv[1:6]
    reads = 0
    delivered_lines = 0
    delivered_chars = 0
    notice = "none"
    with open(path, encoding="utf-8") as handle:
        for raw in handle:
            try:
                event = json.loads(raw)
            except json.JSONDecodeError:
                continue
            content = (event.get("message") or {}).get("content")
            if not isinstance(content, list):
                continue
            for block in content:
                if not isinstance(block, dict):
                    continue
                if block.get("type") == "tool_use" and block.get("name") == "Read":
                    reads += 1
                elif block.get("type") == "tool_result":
                    body = text_of(block.get("content"))
                    if not body:
                        continue
                    delivered_chars += len(body)
                    delivered_lines += body.count("\n") + 1
                    # Match only the harness's own wording. A bare "truncat"
                    # search reported a notice on every read of
                    # planning/SKILL.md, because that file's own prose uses
                    # the word three times -- a detector that fired on the
                    # content it was inspecting rather than on the harness.
                    if "exceeds maximum allowed" in body:
                        notice = "present"
    print(
        "\t".join(
            [
                label,
                model,
                src_lines,
                src_bytes,
                str(reads),
                str(delivered_lines),
                str(delivered_chars),
                notice,
            ]
        )
    )


if __name__ == "__main__":
    main()
