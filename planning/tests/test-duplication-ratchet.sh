#!/usr/bin/env bash
# MODE: DEV
# test-duplication-ratchet — the known duplication may shrink, never grow.
#
# Usage: test-duplication-ratchet.sh
#
# MAINTAINER.md section 3 inventories logic that exists in several places and
# should be one helper. That table used to be 58 `# DEDUPE:` comments scattered
# through the scripts, 51 of which named a helper that had already landed: a
# comment nobody re-reads becomes misinformation. A table has the same failure
# mode unless something checks it, so these caps are the check.
#
# On a genuine migration the count drops: lower the cap in the same commit and
# update the table. Never raise a cap to make this pass.
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scripts="$root/scripts"

note_fail() { printf 'duplication-ratchet: %s\n' "$1" >&2; t_record "$1"; }

# Cap, label, and the counting command. Keep in step with MAINTAINER.md §3.
check_cap() {
    local label="$1" cap="$2" actual="$3"
    if [ "$actual" -gt "$cap" ]; then
        note_fail "$label grew to $actual (cap $cap): use the helper in MAINTAINER.md section 3 instead of adding a copy"
    elif [ "$actual" -lt "$cap" ]; then
        note_fail "$label is down to $actual (cap $cap): lower the cap and update MAINTAINER.md section 3 in this commit (tests/test-gate-caps.sh --clamp applies the low mechanically)"
    fi
}

# Hand-rolled `"$f.tmp.$$"` temp files, which plan_atomic_write/plan_track_tmp own.
check_cap 'hand-rolled .tmp.$$ temp sites' 43 \
    "$( { grep -ho '\.tmp\.\$\$' "$scripts"/*.sh || true; } | wc -l | tr -d ' ')"

# Tests that do not source lib-test.sh, and so cannot record a finding that
# survives a command substitution. Six of them kept a byte-identical copy of the
# library's reporter that exited on the first finding; that count is now zero,
# but "a reporter whose body exits" needs brace matching to count and
# CODE-STYLE.md section 12 rules out parsing shell structure with a pattern. The
# library-source count is the countable precondition for accumulation, so it is
# what this caps.
#
# Counted with grep -L over planning/tests, not the scripts directory the other
# rows use. `fail() { t_fail "$*"; }` shims are deliberate and must not count:
# 32 tests have one, and they are how the call sites stayed unchanged.
check_cap 'tests not sourcing lib-test.sh' 7 \
    "$( { grep -L 'lib-test\.sh' "$root"/tests/test-*.sh || true; } | wc -l | tr -d ' ')"

# Inline inventory-row parsing with hard-coded field indices. plan_inventory_row
# now owns the work-unit rows; the remainder are other tables plus the two
# inventory rewriters, and the floor is 1 (the helper's own parser).
# 29th site: render-plan-overview.sh cells() is a generic canonical-table reader
# (any table, header-aware), not the ten-field inventory parse; admitted here so
# the debt is visible rather than hidden behind a rewritten pattern.
# 30th site: remove-coverage.sh (T17) matches coverage rows by outcome cell --
# a new distinct table, admitted on the same terms as the 29th. The shared
# reader that would absorb both remains future work tracked in MAINTAINER §3.
check_cap "inline awk -F'|' parsers" 19 \
    "$( { grep -ho "awk -F'|'" "$scripts"/*.sh || true; } | wc -l | tr -d ' ')"

# The seed progress-bar literal. test-progress-bar-shape.sh pins the glyphs, so a
# migration must stay byte-identical.
check_cap 'seed progress-bar literal copies' 3 \
    "$( { grep -l '0%%  #### ' "$scripts"/*.sh || true; } | wc -l | tr -d ' ')"

# percent/bar/icon derivation; update-progress.sh is the canonical copy.
check_cap 'percent/bar/icon derivation copies' 3 \
    "$( { grep -l 'completed \* 100 + total / 2' "$scripts"/*.sh || true; } | wc -l | tr -d ' ')"

# Repo-wide, not just $scripts: two `# DEDUPE:` markers once survived in
# benchmark/ while this check was scoped to a single directory.
markers="$( { grep -rl '# DEDUPE:' --include='*.sh' "$root/.." 2>/dev/null || true; } \
    | { grep -v '/benchmark/results/' || true; } \
    | { grep -v 'test-duplication-ratchet' || true; } | wc -l | tr -d ' ')"
[ "$markers" -eq 0 ] \
    || note_fail "$markers file(s) reintroduced a '# DEDUPE:' comment; record it in MAINTAINER.md section 3 instead"

# ── Deliberate duplicates that must stay byte-identical ──────────────────────
# Not everything duplicated should shrink. These pairs are intentionally copied
# and are only correct while they match; a silent divergence is the failure.
assert_identical() {
    local label="$1" fn="$2" a="$3" b="$4"
    local left right
    left="$(awk "/^${fn}\\(\\)/,/^}/" "$a")"
    right="$(awk "/^${fn}\\(\\)/,/^}/" "$b")"
    if [ -z "$left" ] || [ -z "$right" ]; then
        note_fail "$label: could not extract $fn() from both files"
    elif [ "$left" != "$right" ]; then
        note_fail "$label: $fn() has diverged between $(basename "$a") and $(basename "$b")"
    fi
}

# A fix key is SHA-256 over (secret)(sid|finding|unit), secret first (T16
# retired HMAC). If the minting and verifying derivations drift, every
# previously minted key silently stops verifying.
assert_identical 'fix-key derivation' fix_key \
    "$scripts/mint-fix-keys.sh" "$scripts/verify-fix-keys.sh"

# The table must actually exist and name every capped row, so the caps and the
# prose cannot drift apart.
maintainer="$root/MAINTAINER.md"
[ -f "$maintainer" ] || note_fail 'MAINTAINER.md is missing'
if [ -f "$maintainer" ]; then
    grep -Fq '## 3. Pending consolidation' "$maintainer" \
        || note_fail 'MAINTAINER.md has no "Pending consolidation" section for these caps'
    for helper in plan_atomic_write plan_inventory_row plan_progress_bar plan_status_label; do
        grep -Fq "$helper" "$maintainer" \
            || note_fail "MAINTAINER.md section 3 does not mention $helper"
    done
fi

# A helper the table calls "exists" must really exist.
for helper in plan_atomic_write plan_track_tmp plan_progress_bar plan_stat_mode \
    plan_inventory_row plan_inventory_rows plan_inventory_split plan_status_label; do
    grep -rq "${helper}()" "$scripts"/*.sh \
        || note_fail "$helper is referenced by MAINTAINER.md section 3 but no longer exists"
done

[ "$(t_failures)" -eq 0 ] || exit 1
printf '%s\n' 'test-duplication-ratchet: PASS'
