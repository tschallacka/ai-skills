#!/usr/bin/env bash
# test-lib-core.sh — the core functions, each sourced on its own.
#
# The unit layer. Integration tests reach these through the compiled library and
# the entry points; this file sources one function file, stubs what it calls, and
# pins its own behaviour. Overlap with the integration layer is intended: when
# both fail, this one says which function is wrong.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
lib="$repo_root/planning/scripts/lib/core"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/lib-core.XXXXXX")"
trap 'rm -rf "$work"' EXIT

unit() { # <file>... -- <expression>
    local files=() f prelude=''
    while [ "$#" -gt 0 ] && [ "$1" != '--' ]; do files+=("$1"); shift; done
    shift
    for f in "${files[@]}"; do prelude="$prelude source '$lib/$f';"; done
    "$BASH" -c "set -uo pipefail; $prelude $1" 2>&1
}
unit_rc() { # <file>... -- <expression>  → exit code only
    local files=() f prelude='' rc=0
    while [ "$#" -gt 0 ] && [ "$1" != '--' ]; do files+=("$1"); shift; done
    shift
    for f in "${files[@]}"; do prelude="$prelude source '$lib/$f';"; done
    # `|| rc=$?`, because a refusal is the expected outcome in most cases here
    # and an unguarded call would abort this test under set -e with no message.
    "$BASH" -c "set -uo pipefail; $prelude $1" >/dev/null 2>&1 || rc=$?
    printf '%s' "$rc"
}

# ── plan_die: the exit code is the argument, defaulting to 64 ────────────────
t_assert_eq 'die defaults to 64' "$(unit_rc plan_die.sh 00-state.sh -- 'plan_die "boom"')" '64'
t_assert_eq 'die honours a given code' "$(unit_rc plan_die.sh 00-state.sh -- 'plan_die "boom" 73')" '73'
t_assert_contains 'die writes the message to stderr' 'boom' \
    "$(unit plan_die.sh 00-state.sh -- 'plan_die "boom" 2>&1 || true')"
# The registry is what stops a refusal leaving a temp file in the plan tree.
printf 'x\n' > "$work/leftover"
# `|| true` on the outer call: plan_die exits, so an inner `|| true` cannot
# catch it and the subshell's 65 would abort this test under set -e.
unit plan_die.sh 00-state.sh plan_register_temp_file.sh -- \
    "plan_register_temp_file '$work/leftover'; plan_die 'refused' 65" >/dev/null 2>&1 || true
[ ! -e "$work/leftover" ] || t_fail 'plan_die left a registered temp file behind'

# ── plan_fail and plan_warn: neither exits, one counts ───────────────────────
t_assert_eq 'fail increments the counter and returns' \
    "$(unit plan_fail.sh -- 'plan_error_count=0; plan_fail one 2>/dev/null; plan_fail two 2>/dev/null; printf %s "$plan_error_count"')" \
    '2'
t_assert_eq 'warn does not touch the counter' \
    "$(unit plan_warn.sh -- 'plan_error_count=0; plan_warn careful 2>/dev/null; printf %s "$plan_error_count"')" \
    '0'

# ── the guards: each refuses its own failure and passes its own success ──────
t_assert_eq 'require_directory accepts a directory' \
    "$(unit_rc plan_require_directory.sh plan_die.sh 00-state.sh -- "plan_require_directory '$work'")" '0'
# 66 is EX_NOINPUT: the input is not there, distinct from 64 for a usage error.
t_assert_eq 'require_directory refuses a missing path' \
    "$(unit_rc plan_require_directory.sh plan_die.sh 00-state.sh -- "plan_require_directory '$work/nope'")" '66'
printf 'content\n' > "$work/present"
t_assert_eq 'require_file accepts a file' \
    "$(unit_rc plan_require_file.sh plan_die.sh 00-state.sh -- "plan_require_file '$work/present'")" '0'
t_assert_eq 'require_file refuses a directory' \
    "$(unit_rc plan_require_file.sh plan_die.sh 00-state.sh -- "plan_require_file '$work'")" '66'
t_assert_eq 'refuse_existing refuses a path that exists' \
    "$(unit_rc plan_refuse_existing.sh plan_die.sh 00-state.sh -- "plan_refuse_existing '$work/present'")" '73'
t_assert_eq 'refuse_existing allows a free path' \
    "$(unit_rc plan_refuse_existing.sh plan_die.sh 00-state.sh -- "plan_refuse_existing '$work/free'")" '0'

# ── plan_require_safe_value: what counts as unsafe ───────────────────────────
t_assert_eq 'safe value accepts ordinary prose' \
    "$(unit_rc plan_require_safe_value.sh plan_die.sh 00-state.sh -- "plan_require_safe_value label 'An ordinary sentence.'")" '0'
# It guards the document, not the shell: a value is only ever passed as an
# argument, never evaluated, so what must be refused is what breaks a Markdown
# table or a one-line field. Command substitution text is legitimate prose here.
t_assert_eq 'safe value refuses a pipe, which would break a table row' \
    "$(unit_rc plan_require_safe_value.sh plan_die.sh 00-state.sh -- 'plan_require_safe_value label "a | b"')" '64'
t_assert_eq 'safe value refuses a newline' \
    "$(unit_rc plan_require_safe_value.sh plan_die.sh 00-state.sh -- 'plan_require_safe_value label "a
b"')" '64'
t_assert_eq 'safe value refuses an empty value' \
    "$(unit_rc plan_require_safe_value.sh plan_die.sh 00-state.sh -- 'plan_require_safe_value label ""')" '64'
t_assert_eq 'safe value accepts text that merely looks like a substitution' \
    "$(unit_rc plan_require_safe_value.sh plan_die.sh 00-state.sh -- 'plan_require_safe_value label "mentions \$(id) in prose"')" '0'

# ── plan_atomic_write: content, mode, and no leftover temp ──────────────────
printf 'first\n' > "$work/target"
chmod 640 "$work/target"
unit plan_atomic_write.sh plan_die.sh plan_track_tmp.sh plan_stat_probe.sh 00-state.sh -- \
    "printf 'second\n' | plan_atomic_write '$work/target'" >/dev/null 2>&1
t_assert_eq 'atomic write replaces the content' "$(cat "$work/target")" 'second'
# Read the mode through the probe rather than a raw stat: the GNU and BSD forms
# are what plan_stat_mode exists to hide, and PORTABILITY.md bans naming them.
t_assert_eq 'atomic write keeps the mode' \
    "$(unit plan_stat_probe.sh -- "plan_stat_mode '$work/target'")" '640'
t_assert_eq 'atomic write leaves no temp beside the target' \
    "$(find "$work" -name '.target.*' | wc -l | tr -d ' ')" '0'
t_assert_eq 'atomic write refuses a missing directory' \
    "$(unit_rc plan_atomic_write.sh plan_die.sh plan_track_tmp.sh plan_stat_probe.sh 00-state.sh -- \
        "printf x | plan_atomic_write '$work/nodir/file'")" '66'

# ── plan_hoist_plan_dir: the flag moves to a position, both spellings ───────
t_assert_eq 'hoist puts --plan-dir at position 1' \
    "$(unit plan_hoist_plan_dir.sh plan_die.sh 00-state.sh -- 'plan_hoist_plan_dir 1 --plan-dir /p one two')" \
    '/p one two '
t_assert_eq 'hoist accepts the equals spelling' \
    "$(unit plan_hoist_plan_dir.sh plan_die.sh 00-state.sh -- 'plan_hoist_plan_dir 1 --plan-dir=/p one')" \
    '/p one '
t_assert_eq 'hoist leaves a positional call untouched' \
    "$(unit plan_hoist_plan_dir.sh plan_die.sh 00-state.sh -- 'plan_hoist_plan_dir 1 /p one')" \
    '/p one '
t_assert_eq 'hoist can target a later position' \
    "$(unit plan_hoist_plan_dir.sh plan_die.sh 00-state.sh -- 'plan_hoist_plan_dir 2 --plan-dir /p sub one')" \
    'sub /p one '
t_assert_eq 'hoist refuses a flag with no value' \
    "$(unit_rc plan_hoist_plan_dir.sh plan_die.sh 00-state.sh -- 'plan_hoist_plan_dir 1 --plan-dir')" '64'

# ── plan_decode_escaped_newlines: the literal two characters ────────────────
# It prints with %s and no trailing newline, so the decoded value holds one
# newline between the two words rather than terminating a second line.
t_assert_eq 'decode turns backslash-n into a line break' \
    "$(unit plan_decode_escaped_newlines.sh -- 'plan_decode_escaped_newlines "a\\nb" | od -An -c | tr -s " "')" \
    ' a \n b'
t_assert_eq 'decode leaves text without escapes alone' \
    "$(unit plan_decode_escaped_newlines.sh -- 'plan_decode_escaped_newlines "plain"')" \
    'plain'

# ── plan_resolve_symlink: a chain, without readlink -f ──────────────────────
printf 'real\n' > "$work/real.md"
ln -sf "$work/real.md" "$work/link1.md"
ln -sf "$work/link1.md" "$work/link2.md"
t_assert_eq 'resolve follows a two-hop chain' \
    "$(unit plan_resolve_symlink.sh -- "plan_resolve_symlink '$work/link2.md'")" "$work/real.md"
t_assert_eq 'resolve returns a plain file unchanged' \
    "$(unit plan_resolve_symlink.sh -- "plan_resolve_symlink '$work/real.md'")" "$work/real.md"

# ── plan_stat_probe: one of the two forms is defined and both agree ─────────
t_assert_eq 'the probe defines a mode reader' \
    "$(unit plan_stat_probe.sh -- 'declare -F plan_stat_mode >/dev/null && printf yes')" 'yes'
t_assert_eq 'the mode reader prints octal digits' \
    "$(unit plan_stat_probe.sh -- "plan_stat_mode '$work/target'")" '640'

# ── planning_tmpdir: honours the override, defaults otherwise ──────────────
t_assert_eq 'tmpdir sits under TMPDIR' \
    "$(unit planning_tmpdir.sh -- 'TMPDIR=/custom/spot planning_tmpdir')" \
    '/custom/spot/planning-agent'
t_assert_eq 'tmpdir falls back to /tmp' \
    "$(unit planning_tmpdir.sh -- 'unset TMPDIR; planning_tmpdir')" \
    '/tmp/planning-agent'

# ── plan_duplicate_step_numbers: collisions only, never gaps ────────────────
mkdir -p "$work/plan/01-goal/steps"
printf 'x\n' > "$work/plan/01-goal/steps/01-step-one.md"
printf 'x\n' > "$work/plan/01-goal/steps/04-step-four.md"
t_assert_eq 'a gap is not a collision' \
    "$(unit plan_duplicate_step_numbers.sh -- "plan_duplicate_step_numbers '$work/plan'" | grep -c . || true)" '0'
printf 'x\n' > "$work/plan/01-goal/steps/04-step-also-four.md"
collision="$(unit plan_duplicate_step_numbers.sh -- "plan_duplicate_step_numbers '$work/plan'")"
t_assert_contains 'the goal is named' '01-goal' "$collision"
t_assert_contains 'the number is named' '04' "$collision"
t_assert_contains 'both colliding files are named' '04-step-also-four.md' "$collision"
t_assert_contains 'including the second' '04-step-four.md' "$collision"
# A companion shares its step's number by design and must not read as a clash.
printf 'x\n' > "$work/plan/01-goal/steps/01-step-one-testing.md"
t_assert_eq 'a testing companion is not a collision' \
    "$(unit plan_duplicate_step_numbers.sh -- "plan_duplicate_step_numbers '$work/plan'" | grep -c '01-goal 01' || true)" '0'

t_end
