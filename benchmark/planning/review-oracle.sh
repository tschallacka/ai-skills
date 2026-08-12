#!/usr/bin/env bash
set -euo pipefail
# The fixture output is a redacted public adjudication projection.
# Private rows use defect_id and are never written to the public fixture report.
# Published output excludes mutation details and other private material.
# Private filesystem references are published only as the literal <private> token.
if [ "${1:-}" = blinded ]; then
    [ "$#" -eq 5 ] || exit 64
    [ "${ORACLE_ROLE:-}" = independent-oracle ] || { printf 'Independent oracle role is required\n' >&2; exit 64; }
    exec "$(dirname "$0")/grade-blinded-run.sh" "$2" "$3" "$4" "$5"
fi
if [ "$#" -ne 2 ]; then printf 'Usage: %s <semantic-fixture.json> <output.json>\n' "$(basename "$0")" >&2; exit 64; fi
python3 - "$1" "$2" <<'PY'
import json, pathlib, re, sys
fixture = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
def norm(value): return re.sub(r"\s+", " ", str(value).strip().lower())
def nonempty(value): return isinstance(value, str) and bool(value.strip())
def words(value):
    stop = {"a", "an", "and", "as", "for", "from", "in", "of", "replace", "the", "to", "with"}
    return {word for word in re.findall(r"[a-z0-9]+", norm(value)) if len(word) > 2 and word not in stop}
def path_for(finding):
    path = finding.get("path", "")
    if not path and " § " in str(finding.get("location", "")): path = str(finding["location"]).split(" § ", 1)[0]
    return norm(path).lstrip("./")
def valid(finding):
    required = ("finding_id", "path", "location", "summary", "evidence", "required_correction")
    return isinstance(finding, dict) and all(nonempty(finding.get(key)) for key in required) and isinstance(finding.get("independent"), bool)
defects, findings = fixture.get("defects"), fixture.get("findings")
if not isinstance(defects, list) or not defects or not isinstance(findings, list): raise SystemExit("semantic fixture requires non-empty defects and findings lists")
rows, used = [], set()
for defect in defects:
    location = defect.get("location")
    path = defect.get("path") or (str(location).split(" § ", 1)[0] if isinstance(location, str) and " § " in location else "")
    if not all(nonempty(defect.get(key)) for key in ("id", "expected_signal", "required_correction")) or not nonempty(path) or not nonempty(location): raise SystemExit("semantic defect is missing required fields")
    targeted, incomplete = [], []
    for index, raw in enumerate(findings):
        finding = dict(raw) if isinstance(raw, dict) else {}
        finding.setdefault("path", path_for(finding)); finding.setdefault("evidence", finding.get("summary", ""))
        floc, dloc = norm(finding.get("location", "")), norm(location)
        same_location = floc == dloc or floc in dloc or dloc in floc
        if path_for(finding) != norm(path).lstrip("./") or not same_location: continue
        correction = norm(finding.get("required_correction", "")); expected = norm(defect["required_correction"])
        correction_ok = expected in correction or len(words(expected) & words(correction)) * 2 >= len(words(expected))
        if not valid(finding) or finding.get("ambiguous") is True: incomplete.append(index)
        elif norm(defect["expected_signal"]) in norm(f"{finding['summary']} {finding['evidence']}") and correction_ok: targeted.append(index)
        else: incomplete.append(index)
    if len(targeted) > 1: classification, selected = "duplicate", targeted
    elif targeted: classification, selected = "true_positive", targeted
    elif incomplete: classification, selected = ("ambiguous" if any(isinstance(findings[i], dict) and findings[i].get("ambiguous") is True for i in incomplete) else "unresolved"), incomplete
    else: classification, selected = "false_positive", []
    used.update(selected); rows.append((classification, selected))
seeded = {defect["id"] for defect in defects}
exact = {row.get("finding_id") for row in findings if isinstance(row, dict) and row.get("finding_id") in seeded}
counts = {"true_positives": sum(kind == "true_positive" for kind, _ in rows), "false_positives": sum(kind == "false_positive" for kind, _ in rows) + max(0, len(findings) - len(used)), "duplicates": sum(kind == "duplicate" for kind, _ in rows), "unresolved": sum(kind == "unresolved" for kind, _ in rows), "ambiguous": sum(kind == "ambiguous" for kind, _ in rows), "false_negatives": sum(kind != "true_positive" for kind, _ in rows), "independent_catches": sum(kind == "true_positive" and any(isinstance(findings[i], dict) and findings[i].get("independent") is True for i in selected) for kind, selected in rows)}
denominator = len(defects)
result = {"schema_version": "1.4.2-semantic-oracle-fixture", "fixture_id": fixture.get("fixture_id"), "counts": counts, "denominators": {"seeded": denominator}, "semantic_true_positive_rate": counts["true_positives"] / denominator if denominator else None, "independent_catch_rate": counts["independent_catches"] / denominator if denominator else None, "mechanical_exact_id_matches": len(exact), "mechanical_exact_id_rate": len(exact) / denominator if denominator else None, "evidence_paths": [path for path in fixture.get("evidence_paths", []) if isinstance(path, str)]}
pathlib.Path(sys.argv[2]).write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
