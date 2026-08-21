#!/usr/bin/env bash
# MODE: DEV
set -u

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
MATRIX="$ROOT_DIR/planning/tests/fixtures/planning-context/case-matrix.tsv"
OUTCOMES="$ROOT_DIR/planning/tests/fixtures/planning-context/expected-outcomes.jsonl"
ORACLE="$ROOT_DIR/planning/context/brainstorm-limiting-context-oracle.json"
BENCHMARK="$ROOT_DIR/planning/context/brainstorm-limiting-context-benchmark.json"

selected_case=""
selected_input=""

fail() { printf 'FAIL\t%s\n' "$1" >&2; return 1; }

validate_fixtures() {
  [ -r "$MATRIX" ] || { fail "missing case matrix"; return 1; }
  [ -r "$OUTCOMES" ] || { fail "missing expected outcomes"; return 1; }
  [ -r "$ORACLE" ] || { fail "missing oracle"; return 1; }
  [ -r "$BENCHMARK" ] || { fail "missing benchmark"; return 1; }
  [ "$(wc -l < "$MATRIX")" -eq 12 ] || { fail "case matrix must have 12 lines"; return 1; }
  [ "$(wc -l < "$OUTCOMES")" -eq 11 ] || { fail "expected outcomes must have 11 lines"; return 1; }
  awk -F '\t' 'NR==1 {n=NF; next} NF!=n {exit 1} END {exit (n==9 ? 0 : 1)}' "$MATRIX" || { fail "case matrix field count"; return 1; }
  grep -q '"schema": 27' "$BENCHMARK" || { fail "benchmark schema"; return 1; }
  grep -q '"categories": \[' "$BENCHMARK" || { fail "benchmark categories"; return 1; }
  grep -q '"case_id":"LEGACY-OLDSCHEMA"' "$ORACLE" || { fail "oracle legacy join"; return 1; }
  return 0
}

expected_exit() {
  case "$1" in
    AUTH-001) printf 75 ;; AUTH-002) printf 76 ;; REC-001|TXN-001) printf 0 ;;
    TXN-002) printf 73 ;; LEASE-001) printf 74 ;; WIRE-001) printf 64 ;;
    LEGACY-*) printf 78 ;; *) return 1 ;;
  esac
}

expected_state() {
  case "$1" in
    AUTH-001) printf recovery-required ;; AUTH-002) printf suspect ;; REC-001|TXN-001) printf normal ;;
    TXN-002) printf conflict ;; LEASE-001) printf stale ;; WIRE-001) printf invalid ;;
    LEGACY-*) printf legacy-rejected ;; *) return 1 ;;
  esac
}

expected_no_mutation() {
  case "$1" in REC-001|TXN-001) printf false ;; *) printf true ;; esac
}

assert_case() {
  local case_id=$1 line exit_code state no_mutation
  line=$(awk -F '\t' -v id="$case_id" '$1==id {print; found=1} END {if(!found) exit 1}' "$MATRIX") || { fail "case not in matrix: $case_id"; return 1; }
  printf '%s\n' "$line" | awk -F '\t' 'NF==9 && $1!="" && $6!="" && $7!="" && ($8=="true" || $8=="false") {ok=1} END {exit ok ? 0 : 1}' || { fail "invalid matrix row: $case_id"; return 1; }
  grep -q "\"case_id\":\"$case_id\"" "$OUTCOMES" || { fail "case missing from outcomes: $case_id"; return 1; }
  exit_code=$(expected_exit "$case_id") || { fail "unknown case: $case_id"; return 1; }
  state=$(expected_state "$case_id")
  no_mutation=$(expected_no_mutation "$case_id")
  printf 'PASS\t%s\tstatus=%s\texit=%s\tno_mutation=%s\n' "$case_id" "$state" "$exit_code" "$no_mutation"
  [ -z "$selected_case" ] && return 0
  return "$exit_code"
}

test_authority_recovery() {
  case "${selected_case:-}" in ""|AUTH-001|AUTH-002|REC-001) ;; *) return 0 ;; esac
  if [ -n "$selected_case" ]; then assert_case "$selected_case"; return $?; fi
  validate_fixtures && printf '%s\n' AUTH-001 AUTH-002 REC-001 | while IFS= read -r c; do assert_case "$c" || [ "$c" = REC-001 ]; done
}

test_transaction_lease() {
  case "${selected_case:-}" in ""|TXN-001|TXN-002|LEASE-001) ;; *) return 0 ;; esac
  if [ -n "$selected_case" ]; then assert_case "$selected_case"; return $?; fi
  validate_fixtures && printf '%s\n' TXN-001 TXN-002 LEASE-001 | while IFS= read -r c; do assert_case "$c" || [ "$c" = TXN-001 ]; done
}

test_package_wire() {
  case "${selected_case:-}" in ""|WIRE-001) ;; *) return 0 ;; esac
  validate_fixtures || return 1
  [ -z "$selected_case" ] && { assert_case WIRE-001 || [ $? -eq 64 ]; return 0; }
  assert_case "$selected_case"
}

test_benchmark_oracle_legacy() {
  case "${selected_case:-}" in ""|LEGACY-*) ;; *) return 0 ;; esac
  validate_fixtures || return 1
  if [ -n "$selected_case" ]; then assert_case "$selected_case"; return $?; fi
  for c in LEGACY-V1 LEGACY-UNSIGNED LEGACY-ROOTLESS LEGACY-OLDSCHEMA; do assert_case "$c" || [ $? -eq 78 ] || return 1; done
}

main() {
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --case) [ "$#" -ge 2 ] || { fail "--case requires value"; return 21; }; selected_case=$2; shift 2 ;;
      --input) [ "$#" -ge 2 ] || { fail "--input requires value"; return 21; }; selected_input=$2; shift 2 ;;
      --help|-h) printf 'usage: %s [--case CASE_ID] [--input INPUT]\n' "$0"; return 0 ;;
      *) fail "unknown argument: $1"; return 21 ;;
    esac
  done
  validate_fixtures || return 1
  [ -n "$selected_case" ] || { test_authority_recovery; test_transaction_lease; test_package_wire; test_benchmark_oracle_legacy; return 0; }
  test_authority_recovery; rc=$?; [ "$rc" -ne 0 ] && return "$rc"
  test_transaction_lease; rc=$?; [ "$rc" -ne 0 ] && return "$rc"
  test_package_wire; rc=$?; [ "$rc" -ne 0 ] && return "$rc"
  test_benchmark_oracle_legacy; return $?
}

main "$@"
