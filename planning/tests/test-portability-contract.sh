#!/usr/bin/env bash
# MODE: DEV
# test-portability-contract — the portability catalogue stays true, and a gotcha
# caught once stays caught.
#
# Usage: test-portability-contract.sh
#
# Four assertions:
#   1. PORTABILITY.md matches what generate-portability.sh produces.
#   2. Every `# PORTABILITY(<id>)` marker names a rule in portability-rules.json.
#   3. No untagged `# PORTABILITY:` markers remain (they cannot be indexed).
#   4. No tracked script contains a detectable banned construct, unless the file
#      is allowlisted for that rule.
#
# Assertion 4 is the one that saves the next agent from rediscovering a trap in
# an unrelated file.
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
rules="$repo_root/portability-rules.json"
generator="$repo_root/generate-portability.sh"

note_fail() { printf 'portability: %s\n' "$1" >&2; t_record "$1"; }

command -v rjq >/dev/null 2>&1 || {
    printf 'portability: UNCONFIGURED (rjq)\n' >&2
    exit 64
}
[ -f "$rules" ] || { note_fail "missing $rules"; exit 1; }
[ -x "$generator" ] || { note_fail "missing or non-executable $generator"; exit 1; }

# Files that legitimately contain a pattern: the registry names every construct,
# the catalogue publishes them, and plan-map-lib.sh is the replacement for one.
in_allowlist() {
    case "$1" in
        ./generate-portability.sh|./planning/tests/test-portability-contract.sh) return 0 ;;
    esac
    case "$1:$2" in
        # The probe that chooses between the GNU and BSD forms must name both.
        # It lives in one function file and is compiled into plan-core-lib.sh.
        ./planning/scripts/lib/core/plan_stat_probe.sh:stat-format) return 0 ;;
        ./planning/scripts/plan-core-lib.sh:stat-format) return 0 ;;
        ./planning/scripts/plan-env.sh:stat-format) return 0 ;;
        ./planning/tests/lib-test.sh:stat-format) return 0 ;;
        ./planning/tests/lib-test.sh:sha256-tool) return 0 ;;
        # This test forces each branch of the chain, so it has to name all three.
        ./planning/tests/test-sha256-fallbacks.sh:sha256-tool) return 0 ;;
        # The T48b gate's known-runtime universe is a word LIST the build
        # compares declarations against; naming a tool is not requiring it.
        ./installer/build.sh:python3-shipped) return 0 ;;
        ./installer/build.sh:sha256-tool) return 0 ;;
        # There is now one probe in the skill, and this is it: plan_sha256_hex
        # chooses between the compiled plan-crypt binary, the GNU form and the
        # BSD form, so it must name all three. It lives in one function file and
        # is compiled into plan-crypt-lib.sh, so both spellings are exempt --
        # the same arrangement plan_stat_probe.sh has above.
        # mint-fix-keys.sh, verify-fix-keys.sh, plan-context-lib.sh and
        # generate-reviewer.sh each held their own copy of the chain and were
        # each exempt; they call plan_sha256_hex now and name no tool, so their
        # exemptions went with the copies.
        ./planning/scripts/lib/crypt/plan_sha256_hex.sh:sha256-tool) return 0 ;;
        ./planning/scripts/lib/crypt/plan_sha256_chain.sh:sha256-tool) return 0 ;;
        ./planning/scripts/plan-crypt-lib.sh:sha256-tool) return 0 ;;
        # Pins the rungs to each other, so it has to name them.
        ./planning/tests/test-plan-crypt.sh:sha256-tool) return 0 ;;
        ./planning/tests/test-fix-keys.sh:sha256-tool) return 0 ;;
        ./planning/tests/test-add-fix-claim.sh:sha256-tool) return 0 ;;
        # The generated dependency tables name every optional runtime they may
        # verify or hint (chat's any-of group, T39). Naming is not requiring:
        # strength still decides, and chat's members are soft.
        ./install.sh:python3-shipped) return 0 ;;
        ./planning/tests/test-register-helpers.sh:bash-by-path-lookup) return 0 ;;
        ./planning/tests/test-installer-noninteractive.sh:bash-by-path-lookup) return 0 ;;
        ./planning/tests/test-plan-dir-synonym.sh:bash-by-path-lookup) return 0 ;;
        ./planning/scripts/runtime/overview-serve-handler.sh:bash-by-path-lookup) return 0 ;;
        ./planning/tests/test-overview-state.sh:bash-by-path-lookup) return 0 ;;
        ./planning/tests/test-runtime-dependencies.sh:sha256-tool) return 0 ;;
        ./benchmark/planning/lib-portable.sh:sha256-tool) return 0 ;;
        ./benchmark/planning/tests/test-review-lifecycle.sh:sha256-tool) return 0 ;;
        # Development-only tooling may use python3 (CODE-STYLE.md §1).
        ./benchmark/*:python3-shipped) return 0 ;;
        ./run-tests.sh:python3-shipped) return 0 ;;
        ./planning/tests/*:python3-shipped) return 0 ;;
        # The overview serve script names its runtime chain (T43a); the
        # generated installer tables name optional runtimes for verify/hint.
        ./planning/scripts/overview-serve.sh:python3-shipped) return 0 ;;

    esac
    return 1
}

script_list() {
    ( cd "$repo_root" && find . -name '*.sh' -type f \
        -not -path './benchmark/results/*' -not -path './.git/*' -not -path './.plans/*' \
        -not -path './.claude/*' \
        | LC_ALL=C sort )
}

# The generator and this test quote the marker format in their own prose, so they
# are not scanned for markers or constructs.
documents_the_format() {
    case "$1" in
        ./generate-portability.sh|./planning/tests/test-portability-contract.sh) return 0 ;;
    esac
    return 1
}

# 1. Freshness.
if ! "$generator" --check >/dev/null 2>&1; then
    note_fail 'PORTABILITY.md is stale; run ./generate-portability.sh'
fi

# 2 and 3. Marker hygiene.
while IFS= read -r file; do
    documents_the_format "$file" && continue
    while IFS= read -r hit; do
        [ -n "$hit" ] || continue
        line="${hit%%:*}"
        id="${hit#*:}"
        if ! rjq -e --arg i "$id" '.rules[]|select(.id==$i)' "$rules" >/dev/null 2>&1; then
            note_fail "$file:$line names unknown rule id '$id' (add it to portability-rules.json)"
        fi
    done < <(awk '/# PORTABILITY\(/ {
                     id = $0
                     sub(/^.*# PORTABILITY\(/, "", id)
                     sub(/\).*$/, "", id)
                     printf "%d:%s\n", FNR, id
                 }' "$repo_root/$file")

    # An untagged marker cannot be indexed into the catalogue.
    while IFS= read -r line; do
        [ -n "$line" ] || continue
        note_fail "$file:$line uses the untagged '# PORTABILITY:' form; use '# PORTABILITY(<rule-id>):'"
    done < <(awk '/# PORTABILITY:/ { print FNR }' "$repo_root/$file")
done < <(script_list)

# 4. No new banned constructs. Comments are stripped first, so prose that names a
# construct (a marker, a docblock) is not mistaken for a use of it.
#
# Each file is stripped once into a mirror tree, then each rule greps that tree
# in a single pass. The obvious shape -- strip and grep per rule per file -- runs
# sed and grep 23x246 times and cost 15.6s of a 164s suite; this is about 270
# processes instead of 11,000. Substitution keeps the line count, so a line
# number in the mirror is the line number in the source.
stripped_root="$(mktemp -d "${TMPDIR:-/tmp}/portability-stripped.XXXXXX")"
scan_files="$(mktemp "${TMPDIR:-/tmp}/portability-files.XXXXXX")"
script_list > "$scan_files"

# One mkdir for every directory, not one per file.
# PORTABILITY(xargs-empty): BSD xargs runs the command once with no arguments when
# input is empty, so the list is known non-empty here by construction.
sed 's|/[^/]*$||' "$scan_files" | sort -u \
    | while IFS= read -r dir; do printf '%s\0' "$stripped_root/$dir"; done \
    | xargs -0 mkdir -p
while IFS= read -r file; do
    sed 's/[[:space:]]*#.*$//' "$repo_root/$file" > "$stripped_root/$file"
done < "$scan_files"

# One grep per rule over the whole mirror. No -q and no -m1: either would close
# the pipe on the first match and report the writer's SIGPIPE (141) instead of
# the finding. Only the first hit per file is reported, as before -- a rule
# violated twice in one file is one thing to fix.
while IFS="$(printf '\t')" read -r rule_id detect; do
    [ -n "$rule_id" ] || continue
    reported=''
    while IFS= read -r hit; do
        [ -n "$hit" ] || continue
        # The ./ prefix is kept: script_list yields ./path and in_allowlist
        # matches that form, so stripping it silently disabled every exemption.
        file="${hit%%:*}"
        rest="${hit#*:}"
        line="${rest%%:*}"
        case " $reported " in *" $file "*) continue ;; esac
        in_allowlist "$file" "$rule_id" && continue
        reported="$reported $file"
        note_fail "$file uses banned construct '$rule_id' at line $line — see PORTABILITY.md"
    done < <(cd "$stripped_root" && grep -rnE -- "$detect" . 2>/dev/null || true)
done < <(rjq -r '.rules[] | select(.detect != null) | "\(.id)\t\(.detect)"' "$rules")
rm -rf "$stripped_root" "$scan_files"

# 5. A foreign checkout under the repo does not reach the catalogue. An agent
# worktree checks the repo out under .claude/, and the scanner walks the
# filesystem by design, so without a prune every finding is listed once per
# worktree under a path that exists on one machine -- which made the committed
# catalogue read as fresh here and stale in every clone.
# Freshness of the committed file is a separate question, asked separately: it
# goes stale whenever a *.sh change is committed without regenerating last, and
# reporting that as pollution would be a false diagnosis of a real failure.
"$repo_root/generate-portability.sh" --check >/dev/null 2>&1 \
    || note_fail 'PORTABILITY.md is stale; a *.sh change was committed without regenerating the catalogue last'

# Two scans to temp paths, compared with each other rather than with the
# committed file, so the prune is tested whatever state that file is in.
scan_without="$(mktemp "${TMPDIR:-/tmp}/portability-scan.XXXXXX")"
scan_with="$(mktemp "${TMPDIR:-/tmp}/portability-scan.XXXXXX")"
probe_dir="$repo_root/.claude/worktrees/probe-portability-scan/planning/scripts"
PORTABILITY_OUTPUT="$scan_without" "$repo_root/generate-portability.sh" >/dev/null 2>&1
mkdir -p "$probe_dir"
# The marker, not the construct: "In the tree" lists marker sightings, so a probe
# carrying only `declare -A` is invisible to the scan and the comparison below
# would pass whether the prune existed or not.
#
# The marker word is assembled at runtime. Spelled out here, this file would
# itself hold a marker, and the scan would harvest it and publish a sighting
# against this test -- which it did, until the scan named this file.
marker_word="PORTABILITY"
printf '#!/usr/bin/env bash\n# %s(assoc-array): a foreign checkout must not be listed.\ndeclare -A probe_map\n' \
    "$marker_word" > "$probe_dir/probe-lib.sh"
PORTABILITY_OUTPUT="$scan_with" "$repo_root/generate-portability.sh" >/dev/null 2>&1
if diff <(grep -v '^<!-- generated: ' "$scan_without") \
        <(grep -v '^<!-- generated: ' "$scan_with") >/dev/null 2>&1; then :; else
    note_fail 'a checkout under .claude/ changed the catalogue; the scanner must prune it'
fi
if grep -Fq 'probe-portability-scan' "$scan_with"; then
    note_fail 'the catalogue names a file from a foreign checkout'
fi
rm -f "$scan_without" "$scan_with"
rm -rf "$repo_root/.claude/worktrees/probe-portability-scan"

[ "$(t_failures)" -eq 0 ] || exit 1
printf '%s\n' 'test-portability-contract: PASS'
