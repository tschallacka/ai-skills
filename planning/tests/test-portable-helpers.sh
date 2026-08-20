#!/usr/bin/env bash
# Regression test for the portable helper vocabulary in plan-document-lib.sh:
# the bash-3.2 map emulation, POSIX symlink resolution, atomic writes, the
# progress-bar arithmetic, and the stat(1) wrapper. Every assertion here pins a
# hard rule from CODE-STYLE.md §1/§5/§8 that CI must keep true on macOS too.
#
# Usage: test-portable-helpers.sh

set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-portable-test.XXXXXX")"
# Installed before the library is sourced on purpose: the library chains the
# EXIT handler it finds, so this also proves plan_cleanup does not eat it.
trap 'rm -rf "$temporary_root"' EXIT

# shellcheck source=planning/scripts/plan-document-lib.sh
source "$script_dir/plan-document-lib.sh"

fail() {
    printf 'FAIL: %s\n' "$1" >&2
    exit 1
}

assert_eq() {
    [ "$2" = "$3" ] || fail "$1: expected [$3], got [$2]"
}

# ── plan_map_* ───────────────────────────────────────────────────────────────
# Keys that differ only in a character the encoding could drop must not collide,
# and a key full of shell metacharacters must round-trip without executing.
meta_key='w;rm -rf /$(touch '"$temporary_root"'/pwned)`touch '"$temporary_root"'/pwned2`'
plan_map_set units 'AR-01' 'dash form'
plan_map_set units 'AR_01' 'underscore form'
plan_map_set units '01-goal/step-name' 'path form'
plan_map_set units "$temporary_root/plan-description.md" 'absolute path'
plan_map_set units "$meta_key" 'metacharacters'
plan_map_set units 'W01' ''

assert_eq 'map dash key' "$(plan_map_get units 'AR-01')" 'dash form'
assert_eq 'map underscore key' "$(plan_map_get units 'AR_01')" 'underscore form'
assert_eq 'map slash key' "$(plan_map_get units '01-goal/step-name')" 'path form'
assert_eq 'map path key' "$(plan_map_get units "$temporary_root/plan-description.md")" 'absolute path'
assert_eq 'map metacharacter key' "$(plan_map_get units "$meta_key")" 'metacharacters'
if [ -e "$temporary_root/pwned" ] || [ -e "$temporary_root/pwned2" ]; then
    fail 'a metacharacter map key was evaluated by the shell'
fi

# An empty stored value is not an absent key.
assert_eq 'map empty value' "$(plan_map_get units 'W01')" ''
if plan_map_get units 'never-set' >/dev/null 2>&1; then
    fail 'plan_map_get returned success for an absent key'
fi

# plan_map_keys returns the original keys, in insertion order, one per line.
{
    printf 'AR-01\nAR_01\n01-goal/step-name\n'
    printf '%s\n' "$temporary_root/plan-description.md"
    printf '%s\nW01\n' "$meta_key"
} > "$temporary_root/expected-keys"
plan_map_keys units > "$temporary_root/actual-keys"
cmp -s "$temporary_root/expected-keys" "$temporary_root/actual-keys" \
    || fail 'plan_map_keys did not return the original keys in insertion order'

# Two maps are independent namespaces.
plan_map_set other 'AR-01' 'other map'
assert_eq 'map namespace isolation' "$(plan_map_get units 'AR-01')" 'dash form'
assert_eq 'map namespace isolation' "$(plan_map_get other 'AR-01')" 'other map'

# ── plan_resolve_symlink ─────────────────────────────────────────────────────
link_dir="$temporary_root/links"
mkdir -p "$link_dir/nested"
printf 'target\n' > "$link_dir/real.md"
ln -s real.md "$link_dir/relative"                       # relative target
ln -s "$link_dir/real.md" "$link_dir/absolute"           # absolute target
ln -s ../real.md "$link_dir/nested/up"                   # relative, one level up
ln -s relative "$link_dir/chained"                       # link to a link

assert_eq 'symlink plain file' "$(plan_resolve_symlink "$link_dir/real.md")" "$link_dir/real.md"
assert_eq 'symlink relative target' "$(plan_resolve_symlink "$link_dir/relative")" "$link_dir/real.md"
assert_eq 'symlink absolute target' "$(plan_resolve_symlink "$link_dir/absolute")" "$link_dir/real.md"
assert_eq 'symlink chain' "$(plan_resolve_symlink "$link_dir/chained")" "$link_dir/real.md"
assert_eq 'symlink up-level target' \
    "$(cat "$(plan_resolve_symlink "$link_dir/nested/up")")" 'target'

# A cycle is diagnosed, not looped on: exit 66 within the hop cap.
ln -s cycle-b "$link_dir/cycle-a"
ln -s cycle-a "$link_dir/cycle-b"
cycle_rc=0
( plan_resolve_symlink "$link_dir/cycle-a" >/dev/null 2>&1 ) || cycle_rc=$?
assert_eq 'symlink cycle exit code' "$cycle_rc" '66'

# ── plan_atomic_write ────────────────────────────────────────────────────────
write_dir="$temporary_root/atomic"
mkdir -p "$write_dir"
printf 'first\nsecond\n' | plan_atomic_write "$write_dir/doc.md"
assert_eq 'atomic write content' "$(cat "$write_dir/doc.md")" "$(printf 'first\nsecond')"

# An existing target's mode survives the replacement.
chmod 640 "$write_dir/doc.md"
printf 'replaced\n' | plan_atomic_write "$write_dir/doc.md"
assert_eq 'atomic write content after replace' "$(cat "$write_dir/doc.md")" 'replaced'
assert_eq 'atomic write preserves mode' "$(plan_stat_mode "$write_dir/doc.md")" '640'

# No temp debris beside the target, and no leftover trap-less .XXXXXX file.
leftovers="$(find "$write_dir" -maxdepth 1 -name '.doc.md.*' -print | wc -l | tr -d ' ')"
assert_eq 'atomic write leaves no temp' "$leftovers" '0'
assert_eq 'atomic write leaves only the target' \
    "$(find "$write_dir" -maxdepth 1 ! -path "$write_dir" -print | wc -l | tr -d ' ')" '1'

# A write into a missing directory is a missing-input failure (66), not a crash.
missing_rc=0
( printf 'x\n' | plan_atomic_write "$write_dir/absent/doc.md" >/dev/null 2>&1 ) || missing_rc=$?
assert_eq 'atomic write missing directory' "$missing_rc" '66'

# plan_track_tmp + plan_cleanup remove a raw temp when the process exits. This
# runs in a child bash, not a `( … )` subshell: bash resets inherited traps in a
# subshell, so the load-time EXIT trap only fires for a real process exit.
tracked_probe="$temporary_root/tracked-probe"
"$BASH" -c '
    set -euo pipefail
    source "$1/plan-document-lib.sh"
    tmp="$(mktemp "$2/raw.XXXXXX")"
    plan_track_tmp "$tmp"
    printf "%s\n" "$tmp" > "$3"
' "$BASH" "$script_dir" "$temporary_root" "$tracked_probe"
tracked="$(cat "$tracked_probe")"
[ ! -e "$tracked" ] || fail "plan_cleanup did not remove a tracked temp: $tracked"

# ── plan_progress_* ──────────────────────────────────────────────────────────
# Byte-equality against the arithmetic the three call sites share verbatim:
# update-progress.sh, update-plan-progress.sh and rebuild-plan-progress.sh.
call_site_render() {
    local completed="$1" total="$2" width="$3" percent filled empty bar icon
    percent=0
    if [ "$total" -gt 0 ]; then
        percent=$(( (completed * 100 + total / 2) / total ))
    fi
    filled=$(( percent * width / 100 )); empty=$(( width - filled ))
    bar="$(printf '%*s' "$filled" '' | tr ' ' '#')$(printf '%*s' "$empty" '' | tr ' ' '-')"
    icon='💤'
    if [ "$completed" -gt 0 ]; then icon='⏳'; fi
    if [ "$percent" -eq 100 ]; then icon='✅'; fi
    printf '`%s%%  %s  100%%` %s\n' "$percent" "$bar" "$icon"
}

for case_spec in '0 0 20' '0 1 20' '1 3 20' '1 2 20' '2 3 20' '3 3 20' \
                 '1 7 20' '5 6 20' '7 8 20' '1 3 10' '1 3 40' '99 100 20'; do
    # Intentionally unquoted: the case spec is a fixed three-field literal.
    # shellcheck disable=SC2086
    set -- $case_spec
    expected="$(call_site_render "$1" "$2" "$3")"
    percent="$(plan_progress_percent "$1" "$2")"
    actual="$(printf '`%s%%  %s  100%%` %s\n' \
        "$percent" "$(plan_progress_bar "$1" "$2" "$3")" \
        "$(plan_progress_icon "$1" "$percent")")"
    assert_eq "progress render $case_spec" "$actual" "$expected"
done

# The default width is the 20 columns every call site uses.
assert_eq 'progress default width' \
    "$(plan_progress_bar 1 3)" "$(plan_progress_bar 1 3 20)"
assert_eq 'progress full bar' "$(plan_progress_bar 4 4)" '####################'

# ── plan_stat_mode / plan_stat_uid ───────────────────────────────────────────
mode_probe="$temporary_root/mode-probe"
: > "$mode_probe"
chmod 754 "$mode_probe"
assert_eq 'stat mode 754' "$(plan_stat_mode "$mode_probe")" '754'
chmod 600 "$mode_probe"
assert_eq 'stat mode 600' "$(plan_stat_mode "$mode_probe")" '600'
# The uid wrapper must agree with the shell's view of the file's owner.
[ "$(plan_stat_uid "$mode_probe")" = "$(id -u)" ] \
    || fail 'plan_stat_uid did not report the current user as the owner'

# ── plan_fail / plan_warn / plan_die ─────────────────────────────────────────
# ---- the shared reporter survives a subshell -------------------------------
# This is the property the 26 converted tests are here for. A local counter
# incremented inside a command substitution is discarded with the subshell, so
# the epilogue reads zero and reports PASS over a real finding.
subshell_probe="$(
    set -euo pipefail
    source "$(dirname "${BASH_SOURCE[0]}")/lib-test.sh"
    t_begin
    note_fail() { printf 'FAIL: %s\n' "$1" >&2; t_record "$1"; }
    raise() { note_fail 'raised inside a command substitution'; printf 'value\n'; }
    captured="$(raise 2>/dev/null)"
    printf '%s' "$(t_failures)"
    rm -f "$T_FINDINGS"
)"
assert_eq 't_record survives a command substitution' "$subshell_probe" '1'

assert_eq 'error counter starts at zero' "$plan_error_count" '0'
plan_fail 'a recorded finding' 2>/dev/null
plan_fail 'a second finding' 2>/dev/null
assert_eq 'plan_fail increments the counter' "$plan_error_count" '2'
plan_warn 'a warning' 2>/dev/null
assert_eq 'plan_warn does not increment the counter' "$plan_error_count" '2'
assert_eq 'plan_fail writes FAIL to stderr' \
    "$(plan_fail 'shape' 2>&1 >/dev/null)" 'FAIL: shape'
assert_eq 'plan_warn writes WARN to stderr' \
    "$(plan_warn 'shape' 2>&1 >/dev/null)" 'WARN: shape'

die_rc=0
( plan_die 'fatal by default' 2>/dev/null ) || die_rc=$?
assert_eq 'plan_die default exit code' "$die_rc" '64'
die_rc=0
( plan_die 'fatal with a code' 73 2>/dev/null ) || die_rc=$?
assert_eq 'plan_die honours an explicit code' "$die_rc" '73'
assert_eq 'plan_die prefixes the script name' \
    "$( ( plan_die 'shape' ) 2>&1 >/dev/null || true )" "${0##*/}: shape"

# ── plan_awk_trim ────────────────────────────────────────────────────────────
# Anchored on both ends: interior whitespace runs must survive.
assert_eq 'awk trim' \
    "$(printf '|  padded  cell  |\n' | awk -F'|' "$(plan_awk_trim) { print trim(\$2) }")" \
    'padded  cell'

# ── plan_require_* ───────────────────────────────────────────────────────────
require_rc=0
( plan_require_file "$temporary_root/absent.md" 2>/dev/null ) || require_rc=$?
assert_eq 'plan_require_file exit code' "$require_rc" '66'
require_rc=0
( plan_require_directory "$temporary_root/absent-dir" 2>/dev/null ) || require_rc=$?
assert_eq 'plan_require_directory exit code' "$require_rc" '66'
require_rc=0
( plan_refuse_existing "$mode_probe" 2>/dev/null ) || require_rc=$?
assert_eq 'plan_refuse_existing exit code' "$require_rc" '73'
plan_require_file "$mode_probe"
plan_require_directory "$temporary_root"
plan_refuse_existing "$temporary_root/absent.md"

# ── plan_require_bash ────────────────────────────────────────────────────────
plan_require_bash 3
bash_rc=0
( plan_require_bash 99 2>/dev/null ) || bash_rc=$?
assert_eq 'plan_require_bash exit code' "$bash_rc" '78'
assert_eq 'plan_require_bash hints at Homebrew' \
    "$( ( plan_require_bash 99 ) 2>&1 >/dev/null | grep -c 'brew install bash' )" '1'

printf 'Portable helper regression test passed.\n'
