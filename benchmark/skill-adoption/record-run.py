#!/usr/bin/env python3
"""Validates and appends one finished cell's bundle to results/runs.jsonl.

Usage: record-run.py RESULTS_DIR BUNDLE_JSON_FILE ENVIRONMENT_JSON_FILE \
           TRANSCRIPT_FILE REP

- BUNDLE_JSON_FILE: the experimenter's own final JSON message (step 6 of
  PROTOCOL.md), extracted by run-cell.sh from the experimenter's `-p
  --output-format=json` result field. Per PROTOCOL.md's "Known
  limitations", this falls back to scanning for the last well-formed JSON
  object in that field if it does not parse cleanly on its own -- logged
  either way.
- ENVIRONMENT_JSON_FILE: the `environment` object run-cell.sh captured
  itself (lib/common.sh's wa_capture_environment) -- merged in here rather
  than trusted from the experimenter, which cannot know e.g. the real
  ai_skills_commit.
- REP: this attempt's rep number (the experimenter does not track this).

Appends the merged, schema-validated record to RESULTS_DIR/runs.jsonl,
copies TRANSCRIPT_FILE to RESULTS_DIR/transcripts/<run_id>.txt, and updates
RESULTS_DIR/registry.jsonl's matching line's outcome to "completed" (by
appending a corrected line -- registry.jsonl is append-only, see
schema/registry-entry.schema.json; the LAST line for a given run_id is the
authoritative one on read).
"""
import fcntl
import json
import pathlib
import shutil
import sys
import uuid

try:
    import jsonschema
except ImportError:
    jsonschema = None

HERE = pathlib.Path(__file__).parent.resolve()
RUN_RECORD_SCHEMA = json.loads((HERE / "schema" / "run-record.schema.json").read_text())


def _try_parse(text):
    """Strict first; Haiku frequently emits a literal newline or tab inside
    a JSON string value (observer_notes especially) instead of escaping it,
    which is invalid per the JSON spec but something json.loads's strict=False
    mode explicitly accepts. Measured against a full live pass (2026-10-18):
    11 of 50 cells failed to parse at all before this fallback existed --
    this is the dominant real failure mode, not an edge case."""
    try:
        return json.loads(text), False
    except json.JSONDecodeError:
        pass
    return json.loads(text, strict=False), True


def extract_bundle(raw_text, results_dir):
    """The experimenter's final message should be exactly one JSON object.
    Falls back to the last well-formed JSON object found in the text if it
    is not -- see this file's own docstring and PROTOCOL.md's note on this."""
    raw_text = raw_text.strip()
    try:
        bundle, used_lenient = _try_parse(raw_text)
        return bundle, used_lenient
    except json.JSONDecodeError:
        pass
    # Fallback: scan for every top-level '{' ... '}' span and parse each.
    # The raw text can legitimately contain more than one top-level JSON
    # object -- Rule 2 of the experimenter's brief has it repeat the bundle
    # verbatim if a late notification arrives after it already emitted one,
    # and incidental JSON-looking text earlier in the message is also
    # possible. Taking the first candidate that merely parses (the previous
    # behavior) could return a non-bundle object and fail later with
    # KeyError: 'run_id' -- prefer the LAST candidate that actually looks
    # like a bundle (has "run_id"), falling back to the last parseable
    # object only if none do.
    depth = 0
    start_candidates = []
    candidates = []
    for i, ch in enumerate(raw_text):
        if ch == "{":
            if depth == 0:
                start_candidates.append(i)
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0 and start_candidates:
                end = i + 1
                start = start_candidates[-1]
                try:
                    bundle, _ = _try_parse(raw_text[start:end])
                except json.JSONDecodeError:
                    continue
                candidates.append(bundle)
    bundle_candidates = [c for c in candidates if isinstance(c, dict) and "run_id" in c]
    if bundle_candidates:
        return bundle_candidates[-1], True
    if candidates:
        return candidates[-1], True
    sys.exit(_unparsed_bundle_error(raw_text, results_dir))


def _unparsed_bundle_error(raw_text, results_dir):
    """Preserves the raw text that defeated every parse attempt -- the
    workspace that produced it is gone by the time run-cell.sh's cleanup
    trap fires, so without this the only evidence of a real parse failure
    is this error message, which isn't enough to diagnose it."""
    unparsed_dir = results_dir / "unparsed"
    unparsed_dir.mkdir(parents=True, exist_ok=True)
    dump_path = unparsed_dir / f"{uuid.uuid4()}.txt"
    dump_path.write_text(raw_text, encoding="utf-8")
    return (
        "record-run.py: could not find any well-formed JSON object in the "
        f"bundle text -- raw text preserved at {dump_path}"
    )


def main():
    if len(sys.argv) != 6:
        sys.exit(
            "usage: record-run.py RESULTS_DIR BUNDLE_JSON_FILE "
            "ENVIRONMENT_JSON_FILE TRANSCRIPT_FILE REP"
        )
    results_dir = pathlib.Path(sys.argv[1])
    bundle_file = pathlib.Path(sys.argv[2])
    environment_file = pathlib.Path(sys.argv[3])
    transcript_file = pathlib.Path(sys.argv[4])
    rep = int(sys.argv[5])

    results_dir.mkdir(parents=True, exist_ok=True)
    (results_dir / "transcripts").mkdir(exist_ok=True)

    bundle, used_fallback = extract_bundle(bundle_file.read_text(encoding="utf-8"), results_dir)
    if not isinstance(bundle, dict) or "run_id" not in bundle:
        sys.exit(_unparsed_bundle_error(bundle_file.read_text(encoding="utf-8"), results_dir))
    if used_fallback:
        print(
            "record-run.py: NOTE -- the experimenter's bundle did not parse "
            "as clean JSON on its own; recovered via fallback scan. This is "
            "the signal PROTOCOL.md's 'Known limitations' asks to be logged, "
            "not silently absorbed -- consider tightening the brief wording "
            "if this recurs often.",
            file=sys.stderr,
        )

    environment = json.loads(environment_file.read_text(encoding="utf-8"))
    bundle["environment"] = environment
    bundle["rep"] = rep
    bundle["transcript_path"] = f"transcripts/{bundle['run_id']}.txt"

    if jsonschema is not None:
        try:
            jsonschema.validate(bundle, RUN_RECORD_SCHEMA)
        except jsonschema.ValidationError as e:
            sys.exit(
                f"record-run.py: bundle for run_id={bundle.get('run_id', '?')} "
                f"fails schema validation at {list(e.path)}: {e.message} -- "
                "NOT recorded. Fix the experimenter's brief wording or the "
                "schema, then re-run this cell; nothing was appended to "
                "runs.jsonl or registry.jsonl."
            )
    else:
        print(
            "record-run.py: WARNING -- python jsonschema package not "
            "available, skipping schema validation. Install it "
            "(pip install jsonschema) before treating this run as trustworthy.",
            file=sys.stderr,
        )

    # Concurrent cells (full-pass.sh) all append to the same runs.jsonl; a
    # long observer_notes field can push a line past PIPE_BUF (4096 bytes on
    # Linux), where plain O_APPEND writes stop being atomic. flock
    # serializes the append so two cells' lines never interleave.
    with open(results_dir / "runs.jsonl", "a", encoding="utf-8") as f:
        fcntl.flock(f, fcntl.LOCK_EX)
        f.write(json.dumps(bundle) + "\n")
        fcntl.flock(f, fcntl.LOCK_UN)

    shutil.copy(transcript_file, results_dir / "transcripts" / f"{bundle['run_id']}.txt")

    registry_entry = {
        "run_id": bundle["run_id"],
        "scenario_id": bundle["scenario_id"],
        "cli": bundle["cli"],
        "mode": bundle["mode"],
        "rep": rep,
        "subject_session_id": bundle.get("subject_session_id"),
        "experimenter_session_id": bundle.get("experimenter_session_id"),
        "workspace": None,  # run-cell.sh's own registry.jsonl line already has this
        "started_at_utc": bundle["started_at_utc"],
        "finished_at_utc": bundle["finished_at_utc"],
        "outcome": "completed",
    }
    with open(results_dir / "registry.jsonl", "a", encoding="utf-8") as f:
        fcntl.flock(f, fcntl.LOCK_EX)
        f.write(json.dumps(registry_entry) + "\n")
        fcntl.flock(f, fcntl.LOCK_UN)

    print(f"record-run.py: recorded run_id={bundle['run_id']} "
          f"adopted={bundle['adoption']['adopted']} "
          f"task_correct={bundle['regression']['task_correct']}")


if __name__ == "__main__":
    main()
