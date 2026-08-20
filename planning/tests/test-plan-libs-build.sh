#!/usr/bin/env bash
# test-plan-libs-build.sh — the four compiled libraries match their sources.
#
# planning/scripts/lib/<group>/*.sh is the maintained form, one function per
# file; the four plan-*-lib.sh files are compiled from it and are what ships.
# That makes them generated artifacts under CODE-CONTRACTS.md contract 7:
# regenerated, never hand-edited, and a stale one has to fail rather than drift.
#
# Also pins the properties that make the split worth having: every function file
# is sourceable on its own, every library stays under the 500-line cap
# CODE-STYLE.md sets, and a function added to a group directory reaches the
# library with no other edit.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
scripts_dir="$repo_root/planning/scripts"
builder="$scripts_dir/build-plan-libs.sh"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/plan-libs-build.XXXXXX")"
trap 'rm -rf "$work"' EXIT

libraries='plan-core-lib.sh plan-document-lib.sh plan-table-lib.sh plan-progress-lib.sh'

# ---- the committed libraries are what the sources compile to ----------------
rc=0
"$builder" --check >/dev/null 2>&1 || rc=$?
t_assert_eq 'the committed libraries are up to date' "$rc" 0

# ---- and --check has teeth --------------------------------------------------
# A one-byte edit to a compiled library must be reported, or the guard proves
# nothing about the ones nobody edited either.
probe_target="$scripts_dir/plan-progress-lib.sh"
cp "$probe_target" "$work/pristine"
printf '# a byte that the sources do not carry\n' >> "$probe_target"
rc=0
"$builder" --check >/dev/null 2>&1 || rc=$?
cp "$work/pristine" "$probe_target"
[ "$rc" -ne 0 ] || t_fail 'an edited library passed --check'

# ---- every library stays under the cap --------------------------------------
for library in $libraries; do
    lines="$(grep -c . "$scripts_dir/$library")"
    [ "$lines" -le 500 ] \
        || t_fail "$library is $lines lines, over the 500-line library cap"
done

# ---- every function file is sourceable on its own ---------------------------
# This is what the split buys: a test can pull in one function without the rest.
# A definition must not need its siblings present, even though calling it may.
for member in "$scripts_dir"/lib/*/*.sh; do
    rc=0
    "$BASH" -c "source '$member'" >/dev/null 2>&1 || rc=$?
    [ "$rc" -eq 0 ] \
        || t_fail "${member#"$scripts_dir"/} cannot be sourced on its own (rc=$rc)"
done

# ---- a new function reaches its library with no other edit ------------------
# The group directory is the registration. If this ever needs a list updated
# somewhere, the claim in the builder's docblock is false.
probe_function="$scripts_dir/lib/progress/plan_zz_build_probe.sh"
printf '#!/usr/bin/env bash\nplan_zz_build_probe() { printf probe; }\n' > "$probe_function"
"$builder" >/dev/null 2>&1 || t_fail 'the build failed with a new function file present'
if grep -Fq 'plan_zz_build_probe()' "$scripts_dir/plan-progress-lib.sh"; then :; else
    t_fail 'a new function file did not reach its compiled library'
fi
rm -f "$probe_function"
"$builder" >/dev/null 2>&1 || t_fail 'the build failed after removing the probe'
if grep -Fq 'plan_zz_build_probe' "$scripts_dir/plan-progress-lib.sh"; then
    t_fail 'removing a function file left it in the compiled library'
fi

# ---- the façade still provides every symbol its callers expect --------------
# 40-plus scripts source plan-document-lib.sh. The split has to be invisible to
# them, so this counts the symbols rather than trusting the arrangement.
symbol_count="$("$BASH" -c "source '$scripts_dir/plan-document-lib.sh'; declare -F | wc -l" 2>/dev/null | tr -d ' ')"
[ "${symbol_count:-0}" -ge 60 ] \
    || t_fail "sourcing the façade defined only ${symbol_count:-0} functions; it carried 66 before the split"

t_end
