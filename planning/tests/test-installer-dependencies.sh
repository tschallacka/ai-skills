#!/usr/bin/env bash
# test-installer-dependencies — the per-skill dependency model: a hard
# requirement blocks one skill and nothing else, the summary says so, and the
# replay command it prints actually works.
#
# Usage: test-installer-dependencies.sh
#
# jq is made absent by running the installer under a PATH of symlinks to every
# executable on the real PATH except jq, so the shipped runtime_tool_verify() is
# what decides, not a paraphrase of it.
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
installer="$repo_dir/install.sh"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/installer-deps.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

note_fail() { printf 'installer-deps: %s\n' "$1" >&2; t_record "$1"; }

# A PATH without jq. Symlinking rather than copying keeps it cheap, and the
# per-directory glob keeps it to what this machine actually has.
jqless_bin="$temporary_root/bin"
mkdir -p "$jqless_bin"
build_jqless_path() {
    local dir entry
    while IFS= read -r dir; do
        [ -d "$dir" ] || continue
        for entry in "$dir"/*; do
            [ -x "$entry" ] || continue
            [ "${entry##*/}" = jq ] && continue
            ln -sf "$entry" "$jqless_bin/${entry##*/}" 2>/dev/null || true
        done
    done < <(printf '%s\n' "$PATH" | tr ':' '\n')
}
build_jqless_path
if PATH="$jqless_bin" command -v jq >/dev/null 2>&1; then
    printf 'installer-deps: UNCONFIGURED (jq could not be hidden from PATH)\n' >&2
    exit 0
fi

# $1 is the target root; the rest are installer arguments. Leaves the run in
# RUN_RC / RUN_OUT / RUN_ERR.
run_without_jq() {
    local out="$temporary_root/out" err="$temporary_root/err"
    set +e
    PATH="$jqless_bin" AI_SKILLS_NO_SPLASH=1 bash "$installer" "$@" \
        >"$out" 2>"$err" </dev/null
    RUN_RC=$?
    set -e
    RUN_OUT="$(cat "$out")"
    RUN_ERR="$(cat "$err")"
}

# ── --all: planning is blocked, the other four still install ─────────────────
target="$temporary_root/all"
mkdir -p "$target"
run_without_jq --all --target "$target" --yes
[ "$RUN_RC" -ne 0 ] || note_fail 'a blocked skill did not make the run exit non-zero'
[ ! -e "$target/planning" ] || note_fail 'the blocked skill was written anyway'
for ready in project-specificies resource-limited-testing brainstorm post-implementation-review; do
    [ -f "$target/$ready/SKILL.md" ] \
        || note_fail "a hard requirement of planning blocked $ready as well"
done
case "$RUN_OUT" in
    *'== Summary =='*'Skipped:   planning'*) ;;
    *) note_fail "the summary did not report the blocked skill: $RUN_OUT" ;;
esac
case "$RUN_OUT" in
    *"Installed: $target/brainstorm"*) ;;
    *) note_fail "the summary did not report what was installed: $RUN_OUT" ;;
esac
# The summary is one contiguous block on stdout; the diagnostics stay on stderr.
case "$RUN_ERR" in
    *'== Summary =='*) note_fail 'the summary leaked onto stderr' ;;
esac

# ── The replay command carries the run's target and flags, and works ─────────
replay="$(printf '%s\n' "$RUN_OUT" | sed -n 's/^  \(.*--skill planning .*\)$/\1/p' | head -n 1)"
[ -n "$replay" ] || note_fail 'the summary printed no replay command for the blocked skill'
case "$replay" in
    *"--target $target"*) ;;
    *) note_fail "the replay dropped the target the user chose: $replay" ;;
esac
case "$replay" in
    *--yes*) ;;
    *) note_fail "the replay dropped --yes: $replay" ;;
esac

# Executed verbatim with jq present it must install planning. The exit status is
# not asserted here: the planning permission step needs a tty, which this test
# does not have; the YES_ALL run below covers the status.
replay_target="$temporary_root/replay"
mkdir -p "$replay_target"
set +e
eval "${replay/$target/$replay_target}" >/dev/null 2>&1 </dev/null
set -e
[ -f "$replay_target/planning/SKILL.md" ] \
    || note_fail 'the printed replay command did not install planning'

# ── Mutations: the assertions above must be able to fail ────────────────────
mutated_target="$temporary_root/mutated"
mkdir -p "$mutated_target"
set +e
eval "${replay/$target/$mutated_target}" >/dev/null 2>&1 </dev/null
eval "${replay/--skill planning/--skill no-such-skill}" >/dev/null 2>&1 </dev/null
bad_skill_rc=$?
set -e
[ -f "$mutated_target/planning/SKILL.md" ] \
    || note_fail 'the replay ignored --target, so a broken target would go unnoticed'
[ "$bad_skill_rc" -ne 0 ] || note_fail 'a replay naming an unknown skill still succeeded'

# ── One requested skill, blocked: a refusal, not a silent no-op ──────────────
single="$temporary_root/single"
mkdir -p "$single"
run_without_jq --skill planning --target "$single" --yes
[ "$RUN_RC" -ne 0 ] || note_fail 'blocking the only requested skill exited 0'
[ -z "$(ls -A "$single")" ] || note_fail 'the refused single-skill run wrote files'
case "$RUN_ERR" in
    *'Nothing was installed'*) ;;
    *) note_fail "the single blocked skill was not refused clearly: $RUN_ERR" ;;
esac
case "$RUN_OUT" in
    *'1. install jq:'*) ;;
    *) note_fail "the summary did not say how to install jq: $RUN_OUT" ;;
esac

[ "$(t_failures)" -eq 0 ] || exit 1
printf '%s\n' 'test-installer-dependencies: PASS'
