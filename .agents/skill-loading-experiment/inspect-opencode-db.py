#!/usr/bin/env python3
# MODE: DEV
"""Report what opencode actually stored for a session's message parts.

Direct evidence for the `-f` attachment path: opencode inlines an attachment
as a text part, so the stored length against the source file's length shows
whether the attachment was delivered whole or trimmed -- without asking the
model anything.

Read-only, and meant to be pointed at a COPY of opencode.db, never the live
file.
"""

from __future__ import annotations

import json
import sqlite3
import sys


def main() -> None:
    db, session_id = sys.argv[1:3]
    markers = {}
    if len(sys.argv) > 3:
        with open(sys.argv[3], encoding="utf-8") as handle:
            next(handle)
            for row in handle:
                line, token = row.rstrip("\n").split("\t")
                markers[line] = token

    conn = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    rows = conn.execute(
        "select id, data from part where session_id = ? order by time_created",
        (session_id,),
    ).fetchall()
    print(f"session {session_id}: {len(rows)} parts")
    for part_id, data in rows:
        try:
            payload = json.loads(data)
        except json.JSONDecodeError:
            continue
        kind = payload.get("type")
        text = payload.get("text") or ""
        print(f"  {part_id} type={kind} chars={len(text)}")
        if kind == "file":
            print(f"    filename={payload.get('filename')} mime={payload.get('mime')}")
            print(f"    url={str(payload.get('url'))[:120]}")
        if len(text) > 2000:
            print(f"    head={text[:90]!r}")
            print(f"    tail={text[-90:]!r}")
            present = sorted(
                (int(line) for line, token in markers.items() if token in text)
            )
            missing = sorted(
                (int(line) for line, token in markers.items() if token not in text)
            )
            print(f"    markers_present={present}")
            print(f"    markers_missing={missing}")


if __name__ == "__main__":
    main()
