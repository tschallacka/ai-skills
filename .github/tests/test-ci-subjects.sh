#!/usr/bin/env bash
# MODE: DEV
# test-ci-subjects.sh — ci-subjects.sh maps crates onto build subjects, and
# fails safe when it cannot.
#
# The property under test is asymmetric: wrongly building a subject costs
# runner minutes, wrongly SKIPPING one produces a green tick that means "we
# did not look". So every ambiguous input must turn everything on.
set -uo pipefail
export LC_ALL=C

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
subjects="$here/../ci-subjects.sh"
failures=0

# flags <scope> <crates> -> "rjq chat plan_crypt planning_commands editor"
flags() {
    "$subjects" --scope "$1" --crates "${2:-}" \
        | awk -F= '{ printf "%s%s", sep, $2; sep = " " } END { print "" }'
}

check() { # <label> <want> <scope> [crates]
    local label="$1" want="$2" scope="$3" crates="${4:-}" got
    got="$(flags "$scope" "$crates")"
    if [ "$got" = "$want" ]; then
        printf '  ok    %s\n' "$label"
    else
        printf '  FAIL  %s\n         want: %s\n         got:  %s\n' "$label" "$want" "$got"
        failures=$((failures + 1))
    fi
}

echo "ci-subjects: the safe default"
check "full builds everything"            "true true true true true"      full
check "an unknown scope builds everything" "true true true true true"     wat
check "an empty scope builds everything"   "true true true true true"     ""
check "none builds nothing"                "false false false false false" none

echo "ci-subjects: selective picks the right subject"
check "rjq alone"          "true false false false false"  selective "rjq"
check "plan-crypt alone"   "false false true false false"  selective "plan-crypt"
check "a chat crate"       "false true false false false"  selective "chat-proto"
check "every chat crate"   "false true false false false"  selective "chat-proto chat-server-rs chat-client-rs"
check "an editor crate"    "false false false false true"  selective "ai-text-editor-mcp"
check "a planning crate"   "false false false true false"  selective "planning-core"
check "an unknown crate falls to planning commands" \
                           "false false false true false"  selective "some-new-crate"
check "several subjects at once" \
                           "true true false true false"    selective "rjq chat-proto plan-overview"
check "selective with no crates builds nothing" \
                           "false false false false false" selective ""

echo "ci-subjects: usage"
exit_code() { # <args...> -> the exit status, never the output
    "$subjects" "$@" >/dev/null 2>&1
    printf '%s' "$?"
}

if [ "$(exit_code --nonsense)" -eq 64 ]; then
    printf '  ok    an unknown flag exits 64\n'
else
    printf '  FAIL  an unknown flag should exit 64\n'
    failures=$((failures + 1))
fi

if [ "$(exit_code --help)" -eq 0 ]; then
    printf '  ok    --help exits 0\n'
else
    printf '  FAIL  --help should exit 0\n'
    failures=$((failures + 1))
fi

echo
if [ "$failures" -eq 0 ]; then
    echo "test-ci-subjects: PASS"
    exit 0
fi
printf 'test-ci-subjects: FAIL (%s)\n' "$failures"
exit 1
