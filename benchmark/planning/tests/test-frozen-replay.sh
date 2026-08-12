#!/usr/bin/env bash
set -euo pipefail

# W14: deterministic frozen-archive replay. The two sealed Reviewer B
# approvals (iterative 20260811T203343Z and fresh 20260811T205844Z) are
# re-graded against pilot-blinded-defects.json using a mirror of the grader's
# predicate logic (PATH / LOCATION / SIGNAL / CORRECTION / mutated-conflict),
# and the pinned classifications are asserted. No archived file is edited.

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
results_root="$(cd "$root/.." && pwd)/results"
defects_file="$root/pilot-blinded-defects.json"

[ -f "$defects_file" ] || { echo "pilot-blinded-defects.json not found" >&2; exit 1; }

iterative_approval=""
fresh_approval=""
for approval in "$results_root"/*/current/reviewers/*-B-*/plan/approval.json; do
    [ -f "$approval" ] || continue
    case "$approval" in
        *20260811T203343Z*)
            iterative_approval="$approval"
            ;;
        *20260811T205844Z*)
            fresh_approval="$approval"
            ;;
    esac
done

[ -n "$iterative_approval" ] || { echo "frozen iterative approval not found" >&2; exit 1; }
[ -n "$fresh_approval" ] || { echo "frozen fresh approval not found" >&2; exit 1; }

regrade() {
    python3 - "$1" "$defects_file" <<'PY'
import json
import re
import sys

approval_path, defects_path = sys.argv[1], sys.argv[2]
approval = json.load(open(approval_path, encoding="utf-8"))
manifest = json.load(open(defects_path, encoding="utf-8"))
if not isinstance(manifest, dict) or not isinstance(manifest.get("defects"), list):
    raise SystemExit("defects file is not a dict with a defects list")
defects = manifest["defects"]
findings = approval["approved_findings"]

ORDINALS = {"first": "1", "second": "2", "third": "3", "fourth": "4", "fifth": "5",
            "sixth": "6", "seventh": "7", "eighth": "8", "ninth": "9", "tenth": "10"}


def canonical_words(value):
    return [ORDINALS.get(word, word) for word in re.findall(r"[a-z0-9]+", str(value).lower())]


def norm(value):
    return " ".join(canonical_words(value))


def tokens(value):
    stop = {"a", "an", "and", "as", "for", "from", "in", "of", "replace", "the", "to", "with"}
    words = []
    for word in canonical_words(value):
        if word in stop:
            continue
        if len(word) > 2 or (word.isdigit() and int(word) <= 20):
            words.append(word)
    return set(words)


def section_token(location):
    match = re.search(r"(?:§|section|sec)\s*([0-9]+(?:\.[0-9]+)*)", str(location), re.IGNORECASE)
    if match:
        return match.group(1)
    match = re.search(r"(?<![0-9])([0-9]+\.[0-9]+)(?![0-9])", str(location))
    return match.group(1) if match else ""


def path_segments(finding):
    raw = str(finding.get("path", ""))
    if not raw.strip() and " § " in str(finding.get("location", "")):
        raw = str(finding["location"]).split(" § ", 1)[0]
    parts = [part for part in re.split(r"[;,\n]+", raw) if part.strip()]
    return {norm(part).lstrip("./") for part in parts}


def candidate_matches(finding, defect):
    segments = path_segments(finding)
    defect_path = norm(defect["path"]).lstrip("./")
    if not segments:
        derived = norm(finding.get("path", "")).lstrip("./")
        return bool(derived) and derived == defect_path
    return defect_path in segments


def location_matches(finding, defect):
    listing = [str(finding.get(field, "")) for field in ("location", "summary", "observed_contradiction", "evidence")]
    blob = norm(" ∥ ".join(listing))
    defect_file = norm(defect["path"])
    section = norm(section_token(defect.get("location", "")))
    has_file = bool(defect_file) and defect_file in blob
    has_section = bool(section) and section in blob
    has_line = bool(re.search(r"line[s]?\b", norm(" ".join(listing))))
    return has_file and (has_section or has_line)


def observation_text(finding):
    return " ".join(str(finding.get(field, "")) for field in ("summary", "observed_contradiction", "impact", "evidence"))


def mutated_conflict(finding, defect):
    observed = norm(observation_text(finding))
    mutated = tokens(defect.get("old", "")) | tokens(defect.get("new", ""))
    if mutated:
        return bool(tokens(observed) & mutated)
    return bool(re.search(r"contradict|inconsisten|conflict", observed))


def signal_matches(finding, defect):
    observed = norm(observation_text(finding))
    expected = norm(defect["expected_signal"])
    if expected and expected in observed:
        matched = True
    else:
        expected_words = tokens(defect["expected_signal"])
        observed_words = tokens(observed)
        overlap = len(expected_words & observed_words)
        min_overlap = 2 if len(expected_words) <= 4 else 1
        matched = bool(expected_words) and overlap >= min_overlap and overlap * 2 >= len(expected_words)
    if not matched:
        return False
    return mutated_conflict(finding, defect)


def correction_matches(finding, defect):
    correction = norm(finding.get("required_correction", ""))
    expected = norm(defect["required_correction"])
    if expected and expected in correction:
        return True
    expected_words = tokens(defect["required_correction"])
    if not expected_words:
        return False
    overlap = len(expected_words & tokens(correction))
    min_overlap = 2 if len(expected_words) <= 4 else 1
    return overlap >= min_overlap and overlap * 2 >= len(expected_words)


rows = []
used_findings = set()
for defect in defects:
    targeted = []
    incomplete = []
    for index, finding in enumerate(findings):
        if not candidate_matches(finding, defect) or not location_matches(finding, defect):
            continue
        if signal_matches(finding, defect) and correction_matches(finding, defect):
            targeted.append(index)
        else:
            predicates = []
            if not signal_matches(finding, defect):
                predicates.append("SIGNAL")
            if not correction_matches(finding, defect):
                predicates.append("CORRECTION")
            incomplete.append({"index": index, "predicates": predicates})
    if len(targeted) > 1:
        classification, selected = "duplicate", targeted
    elif targeted:
        classification, selected = "true_positive", targeted
    elif incomplete:
        classification, selected = "partial", [item["index"] for item in incomplete]
    else:
        classification, selected = "false_positive", []
    candidate_indices = targeted + [item["index"] for item in incomplete]
    used_findings.update(candidate_indices)
    failed_predicates = []
    for item in incomplete:
        for pred in item["predicates"]:
            if pred not in failed_predicates:
                failed_predicates.append(pred)
    if classification == "false_positive" and not candidate_indices:
        failed_predicates.append("PATH")
    rows.append({
        "defect_id": defect["id"],
        "finding_ids": [findings[i].get("finding_id") for i in selected],
        "candidate_finding_ids": [findings[i].get("finding_id") for i in candidate_indices],
        "failed_predicates": failed_predicates,
        "classification": classification,
    })

print(json.dumps({
    "mode": approval.get("mode"),
    "rows": rows,
    "unused_findings": len(findings) - len(used_findings),
}, sort_keys=True))
PY
}

iterative_report="$(regrade "$iterative_approval")"
fresh_report="$(regrade "$fresh_approval")"

python3 - "$iterative_report" "$fresh_report" <<'PY'
import json
import sys

iterative = json.loads(sys.argv[1])
fresh = json.loads(sys.argv[2])

assert iterative["mode"] == "iterative", "iterative approval mode mismatch"
assert fresh["mode"] == "fresh-review", "fresh approval mode mismatch"

# Iterative Reviewer B found all three seeded defects in AR-01.
assert [row["classification"] for row in iterative["rows"]] == ["true_positive", "true_positive", "true_positive"], iterative
for row in iterative["rows"]:
    assert row["finding_ids"] == ["AR-01"], row
    assert row["failed_predicates"] == [], row

# Fresh Reviewer B found SD-02 and SD-03; SD-01 is an honest miss.
fresh_by_id = {row["defect_id"]: row for row in fresh["rows"]}
assert fresh_by_id["SD-02"]["classification"] == "true_positive", fresh
assert fresh_by_id["SD-03"]["classification"] == "true_positive", fresh
sd01 = fresh_by_id["SD-01"]
assert sd01["classification"] == "partial", sd01
assert set(sd01["candidate_finding_ids"]) == {"AR-02", "AR-03"}, sd01
assert set(sd01["failed_predicates"]) == {"SIGNAL", "CORRECTION"}, sd01
# Document the honest miss: AR-02 (initial-button) and AR-03 (border) both
# reference plan-description.md section 3.1 but neither carries the
# fourth-generated-button signal nor the replace-third-with-fourth correction.
assert sd01["finding_ids"] == ["AR-02", "AR-03"], sd01

print("Frozen replay: iterative 3/3 true positives; fresh 2/3 with SD-01 partial (honest miss).")
PY

printf 'Frozen archive replay tests passed.\n'