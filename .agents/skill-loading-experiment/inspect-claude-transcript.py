#!/usr/bin/env python3
# MODE: DEV
"""Report what a claude session transcript shows was actually sent.

Direct evidence, as opposed to quizzing the model: every Read tool call with
its offset/limit, the size of each tool result, and any truncation notice the
harness injected. Read-only.
"""

from __future__ import annotations

import json
import sys


def text_of(content) -> str:
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        parts = []
        for block in content:
            if isinstance(block, dict):
                parts.append(block.get("text") or "")
        return "\n".join(parts)
    return ""


def main() -> None:
    path = sys.argv[1]
    markers = set(sys.argv[2:])
    with open(path, encoding="utf-8") as handle:
        for number, raw in enumerate(handle, start=1):
            try:
                event = json.loads(raw)
            except json.JSONDecodeError:
                continue
            message = event.get("message") or {}
            content = message.get("content")
            if not isinstance(content, list):
                continue
            for block in content:
                if not isinstance(block, dict):
                    continue
                kind = block.get("type")
                if kind == "tool_use":
                    payload = json.dumps(block.get("input", {}), sort_keys=True)
                    print(f"L{number} TOOL_USE {block.get('name')} {payload[:300]}")
                elif kind == "tool_result":
                    body = text_of(block.get("content"))
                    first = body.split("\n", 1)[0][:120]
                    hits = sorted(m for m in markers if m in body)
                    print(
                        f"L{number} TOOL_RESULT chars={len(body)} "
                        f"lines={body.count(chr(10)) + 1} first={first!r}"
                    )
                    if hits:
                        print(f"          markers_present={','.join(hits)}")
                    for needle in ("truncat", "too long", "exceeds", "limit"):
                        if needle in body.lower():
                            print(f"          NOTICE contains {needle!r}")


if __name__ == "__main__":
    main()
