#!/usr/bin/env bash
# MODE: DEV
# test-installer-multi-root-refusal.sh — a headless run with several detected
# skill roots and no --target refuses, naming each root, rather than silently
# narrowing to the first one (B163).
#
# The bug this pins: curl -fsSL .../install.sh | bash -s -- --all --yes on a
# machine with several agent roots printed "no interactive channel; using the
# first listed root" and installed into exactly one of them. The summary
# reported the narrowing, but nothing said the OTHER roots were left alone —
# an agent whose root is not first on the list silently never got the update.
#
# PATH is pinned to a minimal set of directories for every run here, not just
# HOME redirected: agent_target_available() also checks `command -v codex`
# (etc.), so a developer machine or CI image with claude/codex/opencode
# themselves on PATH detects roots this test never asked for and neither the
# "exactly two" nor the "exactly one" case would be reproducible. bash,
# coreutils and the other portable tools install.sh itself needs still
# resolve under /usr/bin and /bin on every platform this suite runs on.
set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
installer="$repo_root/install.sh"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/installer-multi-root.XXXXXX")"
trap 'rm -rf "$work"' EXIT

minimal_path="/usr/bin:/bin"
[ -d /usr/bin ] || minimal_path="/bin"
if ! command -v bash >/dev/null 2>&1 || ! PATH="$minimal_path" command -v bash >/dev/null 2>&1; then
    printf '%s\n' 'test-installer-multi-root-refusal: SKIP (bash is not on a minimal PATH here)'
    exit 0
fi

run_installer() { # <home> [extra args...]
    local home="$1"; shift
    env -u XDG_CONFIG_HOME HOME="$home" PATH="$minimal_path" \
        "$BASH" "$installer" --skill planning "$@" </dev/null
}

# ── two available roots (Universal, always; Codex, by directory marker) ────
# --yes, no tty: refuse and name both, rather than silently picking one.
home="$work/home-multi"
mkdir -p "$home/.codex"
rc=0
run_installer "$home" --yes >"$work/out" 2>&1 || rc=$?
t_assert_eq 'a headless run over multiple roots with no --target refuses' "$rc" '1'
t_assert_eq 'nothing was installed under the default root' \
    "$([ -e "$home/.agents" ] && printf yes || printf no)" 'no'
t_assert_eq 'nothing was installed under the codex root' \
    "$([ -e "$home/.codex/skills" ] && printf yes || printf no)" 'no'
case "$(cat "$work/out")" in
    *'2 skill roots are available'*) : ;;
    *) t_fail "the refusal did not name how many roots were found: $(cat "$work/out")" ;;
esac
case "$(cat "$work/out")" in
    *"--target $home/.agents/skills"*) : ;;
    *) t_fail "the refusal did not name the default root: $(cat "$work/out")" ;;
esac
case "$(cat "$work/out")" in
    *"--target $home/.codex/skills"*) : ;;
    *) t_fail "the refusal did not name the codex root: $(cat "$work/out")" ;;
esac

# ── the workaround the refusal names actually works, once per root ─────────
rc=0
run_installer "$home" --target "$home/.agents/skills" --yes \
    >"$work/out2" 2>&1 || rc=$?
t_assert_eq 'the named workaround (--target) succeeds' "$rc" '0'
t_assert_eq 'and actually installs' \
    "$([ -f "$home/.agents/skills/planning/SKILL.md" ] && printf yes || printf no)" 'yes'

# ── exactly one available root: the original default behaviour is unchanged ─
home_one="$work/home-one"
rc=0
run_installer "$home_one" --yes >"$work/out3" 2>&1 || rc=$?
t_assert_eq 'a single available root still uses the old default, unrefused' "$rc" '0'
t_assert_eq 'and installs there' \
    "$([ -f "$home_one/.agents/skills/planning/SKILL.md" ] && printf yes || printf no)" 'yes'

t_end 'test-installer-multi-root-refusal'
