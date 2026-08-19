#!/usr/bin/env bash
set -euo pipefail

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=benchmark/planning/lib-portable.sh
source "$root/lib-portable.sh"

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
[ "$(benchmark_hash_file "$defective_root/plan.md")" = "$(openssl enc -d -aes-256-cbc -pbkdf2 -in "$private_root/defect-map.enc" -pass file:"$private_root/oracle-key" 2>/dev/null | python3 -c 'import json,sys,hashlib; print(json.load(sys.stdin)["defects"][0]["defective_sha256"])')" ]

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

# W12: natural reviewer shapes exercised against the blinded grader. Each case
# seeds a fresh workspace, grades the findings, and pins the per-defect
# classification plus the exact counts dict (which now includes the partial key).
natural="$tmp/natural"
mkdir -p "$natural"
SEED_NATURAL='one initial button
fourth generated button
visible white border
'
run_natural_case() {
    local case="$1" spec="$2" evidence="$3" report="$4"
    local seed="$natural/$case/seed"
    local defective="$natural/$case/defective"
    local private="$natural/$case/private"
    mkdir -p "$seed"
    printf '%s' "$SEED_NATURAL" > "$seed/plan.md"
    "$root/seed-blinded-defects.sh" "$seed" "$defective" "$private" "$spec" >/dev/null
    ORACLE_ROLE=independent-oracle "$root/review-oracle.sh" blinded "$private" "$defective" "$evidence" "$report" >/dev/null
}
check_natural_row() {
    python3 - "$1" "$2" "$3" "$4" "$5" <<'PY'
import json
import sys

report = json.load(open(sys.argv[1], encoding="utf-8"))
row = next(r for r in report["per_defect"] if r["index"] == "defect_%s" % int(sys.argv[2]))
assert row["classification"] == sys.argv[3], row
expected_ids = [] if sys.argv[4] == "-" else sys.argv[4].split(",")
expected_preds = [] if sys.argv[5] == "-" else sys.argv[5].split(",")
assert sorted(row["finding_ids"]) == sorted(expected_ids), row
assert sorted(row["failed_predicates"]) == sorted(expected_preds), row
PY
}
check_natural_counts() {
    python3 - "$1" "$2" <<'PY'
import json
import sys

report = json.load(open(sys.argv[1], encoding="utf-8"))
expected = json.loads(sys.argv[2])
assert report["counts"] == expected, (report["counts"], expected)
PY
}

# Consolidated multi-file path (';'-joined): the defect file must appear as one
# of the path segments. Positive finding lists plan.md among several files.
cat > "$tmp/natural/spec-buttons.json" <<'JSON'
{"defects":[{"id":"SD-01","path":"plan.md","old":"one initial button","new":"two initial buttons","location":"plan.md § 3.1","expected_signal":"one initial button","required_correction":"replace two initial buttons with one","severity":"high"}]}
JSON
cat > "$tmp/natural/spec-generated.json" <<'JSON'
{"defects":[{"id":"SD-02","path":"plan.md","old":"fourth generated button","new":"third generated button","location":"plan.md § 3.1","expected_signal":"fourth generated button","required_correction":"replace third generated button with fourth","severity":"medium"}]}
JSON
cat > "$tmp/natural/spec-border.json" <<'JSON'
{"defects":[{"id":"SD-03","path":"plan.md","old":"visible white border","new":"visible black border","location":"plan.md § 3.1","expected_signal":"visible white border","required_correction":"replace visible black border with visible white border","severity":"low"}]}
JSON

cat > "$tmp/natural/multipath-positive.json" <<'JSON'
{"terminal":true,"target_role":"reviewer-b","transcript_sha256":"t","findings":[{"finding_id":"AR-01","path":"plan.md; plan-description.md; work-unit-inventory.md","location":"plan.md § 3.1","summary":"The plan says one initial button at load, but the deliverable requires two initial buttons.","observed_contradiction":"plan-description.md section 3.1 requires two initial buttons while the proof work units require one initial button.","impact":"A future executor could ship the wrong initial state.","evidence":"plan.md § 3.1 states one initial button and two initial buttons.","required_correction":"replace two initial buttons with one","independent":true}],"evidence_paths":[]}
JSON
run_natural_case multi-path "$tmp/natural/spec-buttons.json" "$tmp/natural/multipath-positive.json" "$tmp/natural/multipath-positive-report.json"
check_natural_row "$tmp/natural/multipath-positive-report.json" 1 true_positive AR-01 -
check_natural_counts "$tmp/natural/multipath-positive-report.json" '{"ambiguous":0,"duplicates":0,"false_negatives":0,"false_positives":0,"independent_catches":1,"true_positives":1,"unresolved":0,"partial":0}'

# Negative multi-path: names plan-description.md but is about an unrelated
# defect (the approval gate), and never lists the defect file plan.md. It must
# stay false_positive.
cat > "$tmp/natural/multipath-negative.json" <<'JSON'
{"terminal":true,"target_role":"reviewer-b","transcript_sha256":"t","findings":[{"finding_id":"AR-02","path":"plan-description.md; work-unit-inventory.md","location":"approval gate status","summary":"The plan's approval gate contradicts the validation verdict.","observed_contradiction":"The plan retains a pending adversarial-review status while the validator reports failures.","impact":"The plan cannot be adopted.","evidence":"validation.md records a pending verdict.","required_correction":"Resolve the approval gate.","independent":true}],"evidence_paths":[]}
JSON
run_natural_case multi-path-negative "$tmp/natural/spec-buttons.json" "$tmp/natural/multipath-negative.json" "$tmp/natural/multipath-negative-report.json"
check_natural_row "$tmp/natural/multipath-negative-report.json" 1 false_positive - PATH
check_natural_counts "$tmp/natural/multipath-negative-report.json" '{"ambiguous":0,"duplicates":0,"false_negatives":1,"false_positives":2,"independent_catches":0,"true_positives":0,"unresolved":0,"partial":0}'

# Prose location: a line-style prose citation that names the defect file.
cat > "$tmp/natural/prose-positive.json" <<'JSON'
{"terminal":true,"target_role":"reviewer-b","transcript_sha256":"t","findings":[{"finding_id":"AR-01","path":"plan.md","location":"In plan.md, the line about generated buttons","summary":"The plan requires the fourth generated button, but the draft finishes on the third generated button.","observed_contradiction":"The contract says the fourth generated button; the implementation drafts stop at the third generated button.","impact":"The chain could end one button early.","evidence":"The generated-button line of plan.md and the finish-handler section both reference the third generated button and the fourth generated button.","required_correction":"replace third generated button with fourth","independent":true}],"evidence_paths":[]}
JSON
run_natural_case prose "$tmp/natural/spec-generated.json" "$tmp/natural/prose-positive.json" "$tmp/natural/prose-positive-report.json"
check_natural_row "$tmp/natural/prose-positive-report.json" 1 true_positive AR-01 -
check_natural_counts "$tmp/natural/prose-positive-report.json" '{"ambiguous":0,"duplicates":0,"false_negatives":0,"false_positives":0,"independent_catches":1,"true_positives":1,"unresolved":0,"partial":0}'

# Negative prose location: same file path but the prose never names the defect
# file, so the location gate rejects it.
cat > "$tmp/natural/prose-negative.json" <<'JSON'
{"terminal":true,"target_role":"reviewer-b","transcript_sha256":"t","findings":[{"finding_id":"AR-02","path":"plan.md","location":"The line about the approval gate","summary":"The approval gate is still unresolved.","observed_contradiction":"The retained gate records a pending verdict.","impact":"The plan cannot be adopted.","evidence":"The gate transcript shows open findings.","required_correction":"Close the gate.","independent":true}],"evidence_paths":[]}
JSON
run_natural_case prose-negative "$tmp/natural/spec-generated.json" "$tmp/natural/prose-negative.json" "$tmp/natural/prose-negative-report.json"
check_natural_row "$tmp/natural/prose-negative-report.json" 1 false_positive - PATH
check_natural_counts "$tmp/natural/prose-negative-report.json" '{"ambiguous":0,"duplicates":0,"false_negatives":1,"false_positives":2,"independent_catches":0,"true_positives":0,"unresolved":0,"partial":0}'

# Section location variants (section / sec. / § / bare 3.1): each must match.
section_variant=0
for location in "plan.md section 3.1" "plan.md sec. 3.1" "plan.md § 3.1" "plan.md 3.1"; do
    section_variant=$((section_variant + 1))
    variant="v${section_variant}"
    cat > "$tmp/natural/section-${variant}.json" <<JSON
{"terminal":true,"target_role":"reviewer-b","transcript_sha256":"t","findings":[{"finding_id":"AR-01","path":"plan.md","location":"$location","summary":"The plan requires a visible white border, but the finish handler draws a visible black border.","observed_contradiction":"The white border and black border requirements conflict.","impact":"The final state could render the wrong border.","evidence":"The border requirement is stated in plan.md alongside the button contract.","required_correction":"replace visible black border with visible white border","independent":true}],"evidence_paths":[]}
JSON
    run_natural_case "section-${variant}" "$tmp/natural/spec-border.json" "$tmp/natural/section-${variant}.json" "$tmp/natural/section-${variant}-report.json"
    check_natural_row "$tmp/natural/section-${variant}-report.json" 1 true_positive AR-01 -
    check_natural_counts "$tmp/natural/section-${variant}-report.json" '{"ambiguous":0,"duplicates":0,"false_negatives":0,"false_positives":0,"independent_catches":1,"true_positives":1,"unresolved":0,"partial":0}'
done

# Negative section variant: a different section number must not match.
cat > "$tmp/natural/section-negative.json" <<'JSON'
{"terminal":true,"target_role":"reviewer-b","transcript_sha256":"t","findings":[{"finding_id":"AR-02","path":"plan.md","location":"plan.md section 4.2","summary":"The plan requires a visible white border, but the finish handler draws a visible black border.","observed_contradiction":"The white border and black border requirements conflict.","impact":"The final state could render the wrong border.","evidence":"The border requirement is stated in plan.md alongside the button contract.","required_correction":"replace visible black border with visible white border","independent":true}],"evidence_paths":[]}
JSON
run_natural_case section-negative "$tmp/natural/spec-border.json" "$tmp/natural/section-negative.json" "$tmp/natural/section-negative-report.json"
check_natural_row "$tmp/natural/section-negative-report.json" 1 false_positive - PATH
check_natural_counts "$tmp/natural/section-negative-report.json" '{"ambiguous":0,"duplicates":0,"false_negatives":1,"false_positives":2,"independent_catches":0,"true_positives":0,"unresolved":0,"partial":0}'

# Hyphenated signal: fourth-generated-button equals fourth generated button.
cat > "$tmp/natural/hyphen-positive.json" <<'JSON'
{"terminal":true,"target_role":"reviewer-b","transcript_sha256":"t","findings":[{"finding_id":"AR-01","path":"plan.md","location":"plan.md § 3.1","summary":"The contract requires the fourth-generated-button; the draft creates a third-generated-button instead.","observed_contradiction":"fourth-generated-button completion versus third-generated-button completion","impact":"The chain could end one button early.","evidence":"the fourth-generated-button requirement and the third-generated-button draft both appear in plan.md § 3.1.","required_correction":"replace third-generated-button with the fourth-generated-button","independent":true}],"evidence_paths":[]}
JSON
run_natural_case hyphen "$tmp/natural/spec-generated.json" "$tmp/natural/hyphen-positive.json" "$tmp/natural/hyphen-positive-report.json"
check_natural_row "$tmp/natural/hyphen-positive-report.json" 1 true_positive AR-01 -
check_natural_counts "$tmp/natural/hyphen-positive-report.json" '{"ambiguous":0,"duplicates":0,"false_negatives":0,"false_positives":0,"independent_catches":1,"true_positives":1,"unresolved":0,"partial":0}'

# Negative hyphenated shape: a hyphenated phrase about a different defect in
# the same file/section shares no signal tokens, so it stays partial.
cat > "$tmp/natural/hyphen-negative.json" <<'JSON'
{"terminal":true,"target_role":"reviewer-b","transcript_sha256":"t","findings":[{"finding_id":"AR-02","path":"plan.md","location":"plan.md § 3.1","summary":"The visible-black-border requirement is unambiguous.","observed_contradiction":"The visible-black-border requirement conflicts with the white-border artifacts.","impact":"A reader could misread the border contract.","evidence":"The visible-black-border requirement appears in plan.md § 3.1.","required_correction":"Keep the visible-black-border requirement.","independent":true}],"evidence_paths":[]}
JSON
run_natural_case hyphen-negative "$tmp/natural/spec-generated.json" "$tmp/natural/hyphen-negative.json" "$tmp/natural/hyphen-negative-report.json"
check_natural_row "$tmp/natural/hyphen-negative-report.json" 1 partial AR-02 SIGNAL,CORRECTION
check_natural_counts "$tmp/natural/hyphen-negative-report.json" '{"ambiguous":0,"duplicates":0,"false_negatives":1,"false_positives":0,"independent_catches":0,"true_positives":0,"unresolved":0,"partial":1}'

# Paraphrase signal: "a single initial-button" paraphrases "one initial button"
# through the token-overlap fallback.
cat > "$tmp/natural/paraphrase-positive.json" <<'JSON'
{"terminal":true,"target_role":"reviewer-b","transcript_sha256":"t","findings":[{"finding_id":"AR-01","path":"plan.md","location":"plan.md § 3.1","summary":"The plan calls for a single initial-button at load, and the proof work units contradict that state.","observed_contradiction":"The deliverable drafts show two initial buttons while the proof requires a single initial-button state.","impact":"The wrong initial state could be built.","evidence":"plan.md § 3.1 and the proof sections describe the initial-button contract.","required_correction":"replace two initial buttons with one","independent":true}],"evidence_paths":[]}
JSON
run_natural_case paraphrase "$tmp/natural/spec-buttons.json" "$tmp/natural/paraphrase-positive.json" "$tmp/natural/paraphrase-positive-report.json"
check_natural_row "$tmp/natural/paraphrase-positive-report.json" 1 true_positive AR-01 -
check_natural_counts "$tmp/natural/paraphrase-positive-report.json" '{"ambiguous":0,"duplicates":0,"false_negatives":0,"false_positives":0,"independent_catches":1,"true_positives":1,"unresolved":0,"partial":0}'

# Negative paraphrase: too little token overlap with the signal, stays partial.
cat > "$tmp/natural/paraphrase-negative.json" <<'JSON'
{"terminal":true,"target_role":"reviewer-b","transcript_sha256":"t","findings":[{"finding_id":"AR-02","path":"plan.md","location":"plan.md § 3.1","summary":"The loading screen presents the buttons at the top of the page.","observed_contradiction":"The buttons are presented identically in every artifact.","impact":"A reader may overlook the layout.","evidence":"plan.md § 3.1 describes the button presentation.","required_correction":"Keep the button presentation unchanged.","independent":true}],"evidence_paths":[]}
JSON
run_natural_case paraphrase-negative "$tmp/natural/spec-buttons.json" "$tmp/natural/paraphrase-negative.json" "$tmp/natural/paraphrase-negative-report.json"
check_natural_row "$tmp/natural/paraphrase-negative-report.json" 1 partial AR-02 SIGNAL,CORRECTION
check_natural_counts "$tmp/natural/paraphrase-negative-report.json" '{"ambiguous":0,"duplicates":0,"false_negatives":1,"false_positives":0,"independent_catches":0,"true_positives":0,"unresolved":0,"partial":1}'

# Ordinal/digit forms: "the fourth generated button (button 4)" and a digit
# correction "replace generated button 3 with generated button 4".
cat > "$tmp/natural/ordinal-positive.json" <<'JSON'
{"terminal":true,"target_role":"reviewer-b","transcript_sha256":"t","findings":[{"finding_id":"AR-01","path":"plan.md","location":"plan.md § 3.1","summary":"Pressing the fourth generated button (button 4) clears the document; the draft finishes at generated button 3.","observed_contradiction":"The contract requires completion on the fourth generated button, but the implementation stops at the third generated button.","impact":"The chain could end one button early.","evidence":"plan.md § 3.1 pairs the fourth generated button with button 4 and the third generated button with button 3.","required_correction":"replace generated button 3 with generated button 4","independent":true}],"evidence_paths":[]}
JSON
run_natural_case ordinal "$tmp/natural/spec-generated.json" "$tmp/natural/ordinal-positive.json" "$tmp/natural/ordinal-positive-report.json"
check_natural_row "$tmp/natural/ordinal-positive-report.json" 1 true_positive AR-01 -
check_natural_counts "$tmp/natural/ordinal-positive-report.json" '{"ambiguous":0,"duplicates":0,"false_negatives":0,"false_positives":0,"independent_catches":1,"true_positives":1,"unresolved":0,"partial":0}'

# Negative ordinal/digit: a different trigger (button 5) shares only one token.
cat > "$tmp/natural/ordinal-negative.json" <<'JSON'
{"terminal":true,"target_role":"reviewer-b","transcript_sha256":"t","findings":[{"finding_id":"AR-02","path":"plan.md","location":"plan.md § 3.1","summary":"Button 5 must also clear the document.","observed_contradiction":"Button 5 is the only clear trigger mentioned.","impact":"A reader could confuse the triggers.","evidence":"plan.md § 3.1 mentions button 5.","required_correction":"Keep button 5 as the clear trigger.","independent":true}],"evidence_paths":[]}
JSON
run_natural_case ordinal-negative "$tmp/natural/spec-generated.json" "$tmp/natural/ordinal-negative.json" "$tmp/natural/ordinal-negative-report.json"
check_natural_row "$tmp/natural/ordinal-negative-report.json" 1 partial AR-02 SIGNAL,CORRECTION
check_natural_counts "$tmp/natural/ordinal-negative-report.json" '{"ambiguous":0,"duplicates":0,"false_negatives":1,"false_positives":0,"independent_catches":0,"true_positives":0,"unresolved":0,"partial":1}'

# Mutated-conflict negative: echoes one value with no contradiction. Path,
# location and signal all match, but the other side of the conflict is never
# named, so the grader must pin it partial with CORRECTION failed.
cat > "$tmp/natural/mutated-negative.json" <<'JSON'
{"terminal":true,"target_role":"reviewer-b","transcript_sha256":"t","findings":[{"finding_id":"AR-02","path":"plan.md","location":"plan.md § 3.1","summary":"The plan requires one initial button.","observed_contradiction":"The plan requires one initial button consistently.","impact":"A reader takes the requirement at face value.","evidence":"plan.md § 3.1.","required_correction":"The plan is consistent; no correction is needed.","independent":true}],"evidence_paths":[]}
JSON
run_natural_case mutated-negative "$tmp/natural/spec-buttons.json" "$tmp/natural/mutated-negative.json" "$tmp/natural/mutated-negative-report.json"
check_natural_row "$tmp/natural/mutated-negative-report.json" 1 partial AR-02 CORRECTION
check_natural_counts "$tmp/natural/mutated-negative-report.json" '{"ambiguous":0,"duplicates":0,"false_negatives":1,"false_positives":0,"independent_catches":0,"true_positives":0,"unresolved":0,"partial":1}'

printf 'Review oracle semantic contract tests passed.\n'
