#!/usr/bin/env bash
# MODE: DEV
# test-installer-dependencies — the per-skill dependency model: a hard
# requirement blocks one skill and nothing else, the summary says so, and the
# replay command it prints actually works.
#
# Usage: test-installer-dependencies.sh
#
# rjq is made absent by running the installer under a PATH of symlinks to every
# executable on the real PATH except rjq, so the shipped runtime_tool_verify() is
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

# A PATH without rjq. Symlinking rather than copying keeps it cheap, and the
# per-directory glob keeps it to what this machine actually has.
jqless_bin="$temporary_root/bin"
mkdir -p "$jqless_bin"
build_jqless_path() {
    local dir entry
    while IFS= read -r dir; do
        [ -d "$dir" ] || continue
        for entry in "$dir"/*; do
            [ -x "$entry" ] || continue
            [ "${entry##*/}" = rjq ] && continue
            ln -sf "$entry" "$jqless_bin/${entry##*/}" 2>/dev/null || true
        done
    done < <(printf '%s\n' "$PATH" | tr ':' '\n')
}
build_jqless_path
if PATH="$jqless_bin" command -v rjq >/dev/null 2>&1; then
    printf 'installer-deps: UNCONFIGURED (rjq could not be hidden from PATH)\n' >&2
    exit 0
fi

# $1 is the target root; the rest are installer arguments. Leaves the run in
# RUN_RC / RUN_OUT / RUN_ERR.
run_without_jq() {
    local out="$temporary_root/out" err="$temporary_root/err"
    set +e
    PATH="$jqless_bin" AI_SKILLS_NO_SPLASH=1 "$BASH" "$installer" "$@" \
        >"$out" 2>"$err" </dev/null
    RUN_RC=$?
    set -e
    RUN_OUT="$(cat "$out")"
    RUN_ERR="$(cat "$err")"
}

# ── --all: the bundled rjq works even when PATH has no rjq ───────────────────
target="$temporary_root/all"
mkdir -p "$target"
run_without_jq --all --target "$target" --yes
[ "$RUN_RC" -eq 0 ] || note_fail 'the bundled rjq did not make the run succeed'
[ -e "$target/planning" ] || note_fail 'the planning skill was not written'
for ready in project-specificies resource-limited-testing brainstorm post-implementation-review; do
    [ -f "$target/$ready/SKILL.md" ] \
        || note_fail "a hard requirement of planning blocked $ready as well"
done
case "$RUN_OUT" in
    *"Installed: $target/planning"*) ;;
    *) note_fail "the summary did not report what was installed: $RUN_OUT" ;;
esac
# The summary is one contiguous block on stdout; the diagnostics stay on stderr.
case "$RUN_ERR" in
    *'== Summary =='*) note_fail 'the summary leaked onto stderr' ;;
esac

# ── One requested skill succeeds without an external jq installation ─────────
single="$temporary_root/single"
mkdir -p "$single"
run_without_jq --skill planning --target "$single" --yes
[ "$RUN_RC" -eq 0 ] || note_fail 'the bundled rjq did not install the requested skill'
[ -f "$single/planning/SKILL.md" ] || note_fail 'the requested skill was not written'

[ "$(t_failures)" -eq 0 ] || exit 1
printf '%s\n' 'test-installer-dependencies: PASS'
