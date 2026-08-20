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
#
# The two targets are pinned here too. Everything that separates them -- the
# provenance lines and the dev-only functions -- is exercised on a copy of the
# scripts directory, so a target that stopped stripping cannot be masked by the
# committed prod artifacts and a failed run cannot leave a planted function file
# in the real lib tree.

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

# ---- the two build targets differ in exactly the ways they are meant to -----
# On a copy, because these cases plant a function file: doing that in the real
# tree would change the committed libraries if the test were interrupted.
copy="$work/tree"
mkdir -p "$copy"
cp -R "$scripts_dir" "$copy/scripts"
copied_builder="$copy/scripts/build-plan-libs.sh"

cat > "$copy/scripts/lib/core/plan_probe_dev_only.sh" <<'PROBE'
#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: DEV
plan_probe_dev_only() { printf 'dev only\n'; }
PROBE

# The two targets write the same path, so the dev build is copied aside before
# the prod build overwrites it -- comparing them otherwise compares prod to prod,
# which is how a target that stopped stripping markers read as passing here.
"$copied_builder" --target dev >/dev/null 2>&1
dev_core="$work/dev-core-lib.sh"
cp "$copy/scripts/plan-core-lib.sh" "$dev_core"
t_assert_eq 'the dev target keeps a function file marked PACKAGE: DEV' \
    "$(grep -c '^plan_probe_dev_only()' "$dev_core")" '1'
t_assert_eq 'the dev target says which target built it' \
    "$(grep -c '^# Target: dev$' "$dev_core")" '1'
# One provenance line per source file, so a grep hit in the compiled library
# names the file to edit rather than a line number in a generated file.
t_assert_eq 'the dev target carries provenance for every source file' \
    "$(grep -c '^# from scripts/lib/core/' "$dev_core")" \
    "$(ls "$copy/scripts/lib/core"/*.sh | wc -l | tr -d ' ')"

"$copied_builder" --target prod >/dev/null 2>&1
prod_core="$copy/scripts/plan-core-lib.sh"
t_assert_eq 'the prod target drops the dev-only function' \
    "$(grep -c '^plan_probe_dev_only()' "$prod_core" || true)" '0'
t_assert_eq 'the prod target carries no provenance' \
    "$(grep -c '^# from scripts/lib/' "$prod_core" || true)" '0'
t_assert_eq 'the prod target says which target built it' \
    "$(grep -c '^# Target: prod$' "$prod_core")" '1'
# The markers are a property of the source file, not of the artifact compiled
# from it: a compiled library claiming '# MODE: DEV' would be read as a source.
t_assert_eq 'no MODE or PACKAGE marker reaches a compiled library' \
    "$(grep -c '^# MODE: \|^# PACKAGE: ' "$prod_core" "$dev_core" 2>/dev/null | grep -v ':0$' | wc -l | tr -d ' ')" '0'
# The prod build is what --check compares against, so a dev build in the tree
# has to read as stale -- that is the only thing stopping it being committed.
"$copied_builder" --target dev >/dev/null 2>&1
rc=0
"$copied_builder" --check >/dev/null 2>&1 || rc=$?
t_assert_eq 'a dev build in the tree is reported as stale' "$rc" '1'
# --check is about what is committed, and what is committed is a prod build. So
# it has to ignore an explicit --target dev rather than validate against it.
rc=0
"$copied_builder" --check --target dev >/dev/null 2>&1 || rc=$?
t_assert_eq '--check compares against prod even when asked for dev' "$rc" '1'
"$copied_builder" --target prod >/dev/null 2>&1
rc=0
"$copied_builder" --check --target dev >/dev/null 2>&1 || rc=$?
t_assert_eq '--check passes on a prod tree even when asked for dev' "$rc" '0'

t_assert_eq 'an unknown target is refused' \
    "$("$copied_builder" --target sideways >/dev/null 2>&1; printf '%s' "$?")" '64'
t_assert_eq 'a --target with no value is refused' \
    "$("$copied_builder" --target >/dev/null 2>&1; printf '%s' "$?")" '64'

t_end
