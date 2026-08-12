#!/usr/bin/env bash
set -euo pipefail
# Compatibility marker: "mode": os.environ.get("ORACLE_MODE")
# The published oracle.json is the redacted aggregate report.

if [ "$#" -ne 4 ] || [ "${ORACLE_ROLE:-}" != independent-oracle ]; then
    printf 'Usage: ORACLE_ROLE=independent-oracle %s <oracle-private-dir> <defective-workspace> <terminal-evidence.json> <report.json>\n' "$(basename "$0")" >&2
    exit 64
fi
private_root="$1" target_root="$2" evidence_file="$3" report_file="$4"
[ -d "$private_root" ] || { printf 'Oracle-private directory not found\n' >&2; exit 66; }
[ -d "$target_root" ] || { printf 'Defective workspace not found\n' >&2; exit 66; }
[ -f "$private_root/oracle-key" ] || { printf 'Oracle key is not available\n' >&2; exit 66; }
[ -f "$private_root/defect-map.enc" ] || { printf 'Encrypted defect map is not available\n' >&2; exit 66; }
[ -f "$private_root/seed-metadata.json" ] || { printf 'Seed metadata is not available\n' >&2; exit 66; }
[ -f "$evidence_file" ] || { printf 'Terminal evidence is not available\n' >&2; exit 66; }

temporary_map="$(mktemp "${TMPDIR:-/tmp}/blinded-defect-map.XXXXXX.json")"
private_rows="$(mktemp "${TMPDIR:-/tmp}/blinded-adjudication.XXXXXX.json")"
trap 'rm -f "$temporary_map" "$private_rows"' EXIT
openssl enc -d -aes-256-cbc -pbkdf2 -in "$private_root/defect-map.enc" -out "$temporary_map" -pass file:"$private_root/oracle-key" >/dev/null 2>&1 || {
    printf 'Encrypted defect map could not be decrypted\n' >&2
    exit 65
}

python3 - "$private_root" "$target_root" "$evidence_file" "$temporary_map" "$private_rows" "$report_file" <<'PY'
import hashlib
import json
import os
import pathlib
import re
import sys

private_root, target_root, evidence_file, map_file, private_rows, report_file = map(pathlib.Path, sys.argv[1:])
evidence = json.loads(evidence_file.read_text(encoding="utf-8"))
if evidence.get("terminal") is not True:
    raise SystemExit("terminal evidence is required")
if evidence.get("target_role") == "independent-oracle" or not isinstance(evidence.get("transcript_sha256"), str) or not evidence["transcript_sha256"]:
    raise SystemExit("target evidence has invalid role or missing transcript hash")
seed_metadata = json.loads((private_root / "seed-metadata.json").read_text(encoding="utf-8"))
encrypted_hash = hashlib.sha256((private_root / "defect-map.enc").read_bytes()).hexdigest()
if encrypted_hash != seed_metadata.get("map_sha256"):
    raise SystemExit("encrypted map hash mismatch")
manifest = json.loads(map_file.read_text(encoding="utf-8"))
defects = manifest.get("defects")
if not isinstance(defects, list) or not defects:
    raise SystemExit("decrypted defect map is invalid")

def required_string(value):
    return isinstance(value, str) and bool(value.strip())

for defect in defects:
    required = ("id", "path", "location", "expected_signal", "required_correction", "severity", "defective_sha256", "old", "new")
    if not isinstance(defect, dict) or any(not required_string(defect.get(key)) for key in required):
        raise SystemExit("decrypted defect map is missing semantic fields")
    target = (target_root / defect["path"]).resolve()
    if target_root.resolve() not in target.parents or not target.is_file():
        raise SystemExit(f"target defect file is unavailable: {defect['id']}")
    if hashlib.sha256(target.read_bytes()).hexdigest() != defect["defective_sha256"]:
        raise SystemExit(f"target defect hash mismatch: {defect['id']}")

findings = evidence.get("findings")

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

REQUIRED_STRING_FIELDS = (
    "finding_id",
    "path",
    "location",
    "summary",
    "observed_contradiction",
    "impact",
    "evidence",
    "required_correction",
)


def envelope_reasons(finding):
    if not isinstance(finding, dict):
        return ["WRONG_TYPE"]
    reasons = []
    for field in REQUIRED_STRING_FIELDS:
        if field not in finding:
            reasons.append("MISSING_FIELD")
        elif not isinstance(finding[field], str):
            reasons.append("WRONG_TYPE")
        elif not finding[field].strip():
            reasons.append("EMPTY_FIELD")
    if "independent" not in finding:
        reasons.append("MISSING_FIELD")
    elif not isinstance(finding["independent"], bool):
        reasons.append("WRONG_TYPE")
    if "ambiguous" in finding:
        if not isinstance(finding["ambiguous"], bool):
            reasons.append("WRONG_TYPE")
        elif finding["ambiguous"] is True:
            reasons.append("AMBIGUOUS_FINDING")
    return list(dict.fromkeys(reasons))


def valid_envelope(finding):
    return not envelope_reasons(finding)


def malformed_finding(finding, index, reasons):
    finding_id = None
    if isinstance(finding, dict) and required_string(finding.get("finding_id")):
        finding_id = finding["finding_id"]
    return {"finding_id": finding_id, "index": index, "reasons": reasons}


def write_malformed_result(items):
    result = {
        "schema_status": "malformed",
        "malformed_findings": items,
        "malformed_count": len(items),
        "review_state": {"reason": "REVIEW_FINDING_SCHEMA_INVALID"},
        "adoption": False,
    }
    report_file.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")


if not isinstance(findings, list):
    reason = "MISSING_FIELD" if "findings" not in evidence else "WRONG_TYPE"
    write_malformed_result([malformed_finding({}, 0, [reason])])
    raise SystemExit(0)

malformed = []
for index, finding in enumerate(findings):
    reasons = envelope_reasons(finding)
    if reasons:
        malformed.append(malformed_finding(finding, index, reasons))
if malformed:
    write_malformed_result(malformed)
    raise SystemExit(0)

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
independent_catches = 0
for defect in defects:
    targeted = []
    incomplete = []
    for index, finding in enumerate(findings):
        if not isinstance(finding, dict):
            continue
        if not candidate_matches(finding, defect) or not location_matches(finding, defect):
            continue
        if not valid_envelope(finding):
            incomplete.append({"index": index, "predicates": ["ENVELOPE"], "ambiguous": False})
        elif finding.get("ambiguous") is True:
            incomplete.append({"index": index, "predicates": ["AMBIGUOUS"], "ambiguous": True})
        elif signal_matches(finding, defect) and correction_matches(finding, defect):
            targeted.append(index)
        else:
            predicates = []
            if not signal_matches(finding, defect):
                predicates.append("SIGNAL")
            if not correction_matches(finding, defect):
                predicates.append("CORRECTION")
            incomplete.append({"index": index, "predicates": predicates, "ambiguous": False})
    if len(targeted) > 1:
        classification, selected = "duplicate", targeted
    elif targeted:
        classification, selected = "true_positive", targeted
    elif any(item["ambiguous"] for item in incomplete):
        classification, selected = "ambiguous", [item["index"] for item in incomplete]
    elif any(any(pred in ("SIGNAL", "CORRECTION") for pred in item["predicates"]) for item in incomplete):
        classification, selected = "partial", [item["index"] for item in incomplete]
    elif incomplete:
        classification, selected = "unresolved", [item["index"] for item in incomplete]
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
    public_ids = [findings[i].get("finding_id") for i in candidate_indices if isinstance(findings[i], dict) and required_string(findings[i].get("finding_id"))]
    selected_ids = [findings[i].get("finding_id") for i in selected if isinstance(findings[i], dict) and required_string(findings[i].get("finding_id"))]
    if classification == "true_positive" and any(findings[i].get("independent") is True for i in selected if isinstance(findings[i], dict)):
        independent_catches += 1
    rows.append({"defect_id": defect["id"], "finding_ids": selected_ids, "candidate_finding_ids": public_ids, "failed_predicates": failed_predicates, "classification": classification, "confidence": "high" if classification == "true_positive" else "low", "rationale": "independent semantic adjudication"})

private_rows.write_text(json.dumps(rows, indent=2, sort_keys=True) + "\n", encoding="utf-8")
counts = {"true_positives": sum(row["classification"] == "true_positive" for row in rows), "false_positives": sum(row["classification"] == "false_positive" for row in rows) + max(0, len(findings) - len(used_findings)), "duplicates": sum(row["classification"] == "duplicate" for row in rows), "partial": sum(row["classification"] == "partial" for row in rows), "unresolved": sum(row["classification"] == "unresolved" for row in rows), "ambiguous": sum(row["classification"] == "ambiguous" for row in rows), "false_negatives": sum(row["classification"] != "true_positive" for row in rows), "independent_catches": independent_catches}
seeded = {defect["id"] for defect in defects}
exact = {finding.get("finding_id") for finding in findings if isinstance(finding, dict) and finding.get("finding_id") in seeded}
denominator = len(defects)
public_paths = []
for path in evidence.get("evidence_paths", []):
    if isinstance(path, str):
        public_paths.append(path.replace(str(private_root), "<private>"))
per_defect = [{"index": f"defect_{position}", "finding_ids": row["candidate_finding_ids"], "failed_predicates": row["failed_predicates"], "classification": row["classification"]} for position, row in enumerate(rows, start=1)]
result = {"schema_version": "1.4.2-semantic-blinded-oracle", "oracle_role": "independent-oracle", "run_id": os.environ.get("ORACLE_RUN_ID"), "revision": os.environ.get("ORACLE_REVISION"), "mode": os.environ.get("ORACLE_MODE"), "target_role": evidence["target_role"], "terminal": True, "encrypted_map_sha256": encrypted_hash, "transcript_sha256": evidence["transcript_sha256"], "counts": counts, "denominators": {"seeded": denominator}, "semantic_true_positive_rate": counts["true_positives"] / denominator if denominator else None, "independent_catch_rate": counts["independent_catches"] / denominator if denominator else None, "mechanical_exact_id_matches": len(exact), "mechanical_exact_id_rate": len(exact) / denominator if denominator else None, "evidence_paths": public_paths, "per_defect": per_defect}
report_file.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
chmod 600 "$report_file"
trap - EXIT
rm -f "$temporary_map" "$private_rows"
printf 'Independent blinded oracle report written: %s\n' "$report_file"
