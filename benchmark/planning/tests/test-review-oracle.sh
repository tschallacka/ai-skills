#!/usr/bin/env bash
set -euo pipefail

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fixture="$tmp/semantic-fixture.json"
cat > "$fixture" <<'JSON'
{"schema_version":"1.4.2-semantic-oracle-fixture","fixture_id":"consolidated","defects":[{"id":"SD-01","path":"plan.md","location":"plan.md § 3.1","expected_signal":"one initial button","required_correction":"replace two initial buttons with one"},{"id":"SD-02","path":"plan.md","location":"plan.md § 3.1","expected_signal":"fourth generated button","required_correction":"replace third generated button with fourth"},{"id":"SD-03","path":"plan.md","location":"plan.md § 3.1","expected_signal":"visible white border","required_correction":"replace visible black border with visible white border"}],"findings":[{"finding_id":"AR-01","path":"plan.md","location":"plan.md § 3.1","summary":"The plan says one initial button, fourth generated button, and visible white border.","observed_contradiction":"The same plan specifies conflicting button counts and border colors.","impact":"The implementation proof cannot establish the required behavior.","evidence":"Those are the observed contradictions.","required_correction":"Synchronize the summary with one initial button, fourth generated button, and visible white border.","independent":true}]}
JSON
output="$tmp/fixture-result.json"
"$root/review-oracle.sh" "$fixture" "$output"
grep -Fq '"true_positives": 3' "$output"
grep -Fq '"mechanical_exact_id_matches": 0' "$output"
grep -Fq '"semantic_true_positive_rate": 1.0' "$output"
! grep -Eq 'SD-0[1-3]|AR-01' "$output"
python3 - "$fixture" "$output" <<'PY'
import json
import sys

fixture = json.load(open(sys.argv[1], encoding="utf-8"))
result = json.load(open(sys.argv[2], encoding="utf-8"))
required = {
    "finding_id", "path", "location", "summary", "observed_contradiction",
    "impact", "evidence", "required_correction", "independent",
}
finding = fixture["findings"][0]
assert set(finding) == required
assert finding["finding_id"] == "AR-01"
assert finding["independent"] is True
assert result["counts"] == {
    "ambiguous": 0,
    "duplicates": 0,
    "false_negatives": 0,
    "false_positives": 0,
    "independent_catches": 3,
    "true_positives": 3,
    "unresolved": 0,
}
assert "PRIVATE-ORACLE-MATERIAL" not in json.dumps(result)
PY

seed_root="$tmp/source"
defective_root="$tmp/defective"
private_root="$tmp/private"
mkdir -p "$seed_root"
printf 'one initial button\nfourth generated button\nvisible white border\n' > "$seed_root/plan.md"
cat > "$tmp/defects.json" <<'JSON'
{"defects":[{"id":"SD-01","path":"plan.md","old":"one initial button","new":"two initial buttons","location":"plan.md § 3.1","expected_signal":"one initial button","required_correction":"replace two initial buttons with one","severity":"high"},{"id":"SD-02","path":"plan.md","old":"fourth generated button","new":"third generated button","location":"plan.md § 3.1","expected_signal":"fourth generated button","required_correction":"replace third generated button with fourth","severity":"medium"},{"id":"SD-03","path":"plan.md","old":"visible white border","new":"visible black border","location":"plan.md § 3.1","expected_signal":"visible white border","required_correction":"replace visible black border with visible white border","severity":"low"}]}
JSON
"$root/seed-blinded-defects.sh" "$seed_root" "$defective_root" "$private_root" "$tmp/defects.json"
[ "$(find "$private_root" -type f -perm -004 | wc -l)" -eq 0 ]
[ ! -e "$defective_root/oracle-key" ]
[ "$(sha256sum "$defective_root/plan.md" | awk '{print $1}')" = "$(openssl enc -d -aes-256-cbc -pbkdf2 -in "$private_root/defect-map.enc" -pass file:"$private_root/oracle-key" 2>/dev/null | python3 -c 'import json,sys,hashlib; print(json.load(sys.stdin)["defects"][0]["defective_sha256"])')" ]

cat > "$tmp/evidence.json" <<'JSON'
{"terminal":true,"target_role":"reviewer-b","transcript_sha256":"transcript-sha","findings":[{"finding_id":"AR-01","path":"plan.md","location":"plan.md § 3.1","summary":"The plan contradicts itself on one initial button, fourth generated button, and visible white border.","observed_contradiction":"The plan contains all three contradictory requirements.","impact":"The proof would accept the wrong implementation.","evidence":"The observed text contains all three signals.","required_correction":"Correct one initial button, fourth generated button, and visible white border.","independent":true}],"evidence_paths":["/tmp/private/secret-path"]}
JSON
ORACLE_ROLE=independent-oracle "$root/review-oracle.sh" blinded "$private_root" "$defective_root" "$tmp/evidence.json" "$tmp/report.json"
grep -Fq '"true_positives": 3' "$tmp/report.json"
grep -Fq '"mechanical_exact_id_matches": 0' "$tmp/report.json"
grep -Fq '"evidence_paths": [' "$tmp/report.json"
! grep -Eq 'SD-0[1-3]|oracle-key|defect-map.enc|secret-path' "$tmp/report.json"

cat > "$tmp/bad-evidence.json" <<'JSON'
{"terminal":true,"target_role":"reviewer-b","transcript_sha256":"transcript-sha","findings":[{"finding_id":"AR-01","path":"plan.md","location":"plan.md § 3.1","summary":"A contradiction exists.","observed_contradiction":"The plan is inconsistent.","impact":"The proof is unreliable.","evidence":"The location is known.","required_correction":"","independent":true}]}
JSON
ORACLE_ROLE=independent-oracle "$root/review-oracle.sh" blinded "$private_root" "$defective_root" "$tmp/bad-evidence.json" "$tmp/bad-report.json"
grep -Fq '"schema_status": "malformed"' "$tmp/bad-report.json"
grep -Fq '"malformed_count": 1' "$tmp/bad-report.json"
grep -Fq '"EMPTY_FIELD"' "$tmp/bad-report.json"
grep -Fq '"REVIEW_FINDING_SCHEMA_INVALID"' "$tmp/bad-report.json"
grep -Fq '"adoption": false' "$tmp/bad-report.json"
! grep -Eq 'true_positives|semantic_true_positive_rate|unresolved' "$tmp/bad-report.json"

cat > "$tmp/id-only-evidence.json" <<'JSON'
{"terminal":true,"target_role":"reviewer-b","transcript_sha256":"transcript-sha","findings":[{"finding_id":"AR-01"}]}
JSON
ORACLE_ROLE=independent-oracle "$root/review-oracle.sh" blinded "$private_root" "$defective_root" "$tmp/id-only-evidence.json" "$tmp/id-only-report.json"
grep -Fq '"malformed_count": 1' "$tmp/id-only-report.json"
grep -Fq '"MISSING_FIELD"' "$tmp/id-only-report.json"
! grep -Eq 'true_positives|semantic_true_positive_rate' "$tmp/id-only-report.json"

python3 - "$tmp/malformed-fields.json" <<'PY'
import json
import sys

base = {
    "finding_id": "AR-01",
    "path": "plan.md",
    "location": "plan.md § 3.1",
    "summary": "A contradiction exists.",
    "observed_contradiction": "The plan is inconsistent.",
    "impact": "The proof is unreliable.",
    "evidence": "The location is known.",
    "required_correction": "Correct the contradiction.",
    "independent": True,
}
string_fields = (
    "finding_id",
    "path",
    "location",
    "summary",
    "observed_contradiction",
    "impact",
    "evidence",
    "required_correction",
    "independent",
)
findings = []
for field in string_fields:
    finding = dict(base)
    finding.pop(field)
    findings.append(finding)
wrong_type = dict(base)
for field in string_fields[:-1]:
    wrong_type[field] = 7
wrong_type["independent"] = "yes"
findings.append(wrong_type)
json.dump({
    "terminal": True,
    "target_role": "reviewer-b",
    "transcript_sha256": "transcript-sha",
    "findings": findings,
}, open(sys.argv[1], "w", encoding="utf-8"))
PY
ORACLE_ROLE=independent-oracle "$root/review-oracle.sh" blinded "$private_root" "$defective_root" "$tmp/malformed-fields.json" "$tmp/malformed-fields-report.json"
python3 - "$tmp/malformed-fields-report.json" <<'PY'
import json
import sys

result = json.load(open(sys.argv[1], encoding="utf-8"))
assert result == {
    "schema_status": "malformed",
    "malformed_count": 10,
    "malformed_findings": [
        {"finding_id": None if index == 0 else "AR-01", "index": index, "reasons": ["MISSING_FIELD"]}
        for index in range(9)
    ] + [{"finding_id": None, "index": 9, "reasons": ["WRONG_TYPE"]}],
    "review_state": {"reason": "REVIEW_FINDING_SCHEMA_INVALID"},
    "adoption": False,
}
PY

if ORACLE_ROLE=reviewer-b "$root/review-oracle.sh" blinded "$private_root" "$defective_root" "$tmp/evidence.json" "$tmp/invalid-role.json" >/dev/null 2>&1; then
    echo 'non-oracle role unexpectedly graded blinded run' >&2
    exit 1
fi

cat > "$tmp/invalid-defects.json" <<'JSON'
{"defects":[{"id":"SD-X","path":"plan.md","old":"one initial button","new":"broken","location":"plan.md § 3.1","expected_signal":"one initial button","severity":"high"}]}
JSON
if "$root/seed-blinded-defects.sh" "$seed_root" "$tmp/invalid-target" "$tmp/invalid-private" "$tmp/invalid-defects.json" >/dev/null 2>&1; then
    echo 'incomplete semantic manifest unexpectedly accepted' >&2
    exit 1
fi

printf 'Review oracle semantic contract tests passed.\n'
