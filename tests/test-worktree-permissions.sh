#!/usr/bin/env bash
# MODE: DEV
# test-worktree-permissions.sh — the installer grants agents read/write on the
# worktree root, and grants it to every install rather than only a planning one.
#
# Three properties, each of which was broken when this was written (B234):
#
#   1. The worktree root is covered at all. The only grant the installer made
#      was planning's three paths, so a checkout under the documented worktrees
#      root was unpermitted and cost a prompt per file.
#   2. Write is granted, not just Edit. Edit covers changing a file that already
#      exists; creating one needs Write, and creating files is most of what
#      working in a fresh checkout consists of. This is the half that reads as
#      "read/write is granted" while still prompting.
#   3. The grant does not depend on the planning skill being selected, because
#      any agent may be asked to take a worktree. planning_permission_step sits
#      inside main's `contains planning` branch; this step must not.
#
# And one property that is a security boundary rather than a convenience: the
# rules name the worktrees root and NOTHING above it. The tempting layout put
# worktrees under tsch-ai-skills/, which also holds the chat server's
# server.key, the editor's private session registry and, on a shared install,
# the installed binaries — so a rule reaching a parent of the worktrees root
# would hand an agent write access to a private key and to the binaries it is
# itself running. The assertion below fails if any granted pattern is broad
# enough to cover tsch-ai-skills.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$repo_root/planning/tests/lib-test.sh"
t_begin

if ! command -v rjq >/dev/null 2>&1; then
    echo "test-worktree-permissions.sh: rjq is not on PATH; run ./setup-dev-env.sh" >&2
    echo "UNCONFIGURED"
    exit 0
fi

work="$(mktemp -d "${TMPDIR:-/tmp}/worktree-perms.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# The permission writers are sourced directly rather than driven through a whole
# install: what is under test is which rules they emit, and a full run would
# need agent roots, a skill copy and interactive confirmation to reach them.
#
# BOTH config paths are redirected through the writers' own environment
# overrides -- CLAUDE_CONFIGFILE and OPENCODE_CONFIGFILE -- and NOT by stubbing
# opencode_configfile(). Stubbing it does not work and fails dangerously: the
# stub has to be defined before this source line, and sourcing then REDEFINES
# it with the real implementation, so the writer resolves the developer's own
# ~/.config/opencode/opencode.json and edits it for real. That happened while
# this test was being written; it added three dead /tmp patterns to a live
# config. The overrides below are read at call time by the real functions, so
# there is nothing to shadow.
export CLAUDE_CONFIGFILE="$work/claude.json"
export OPENCODE_CONFIGFILE="$work/opencode.json"

# Only the interactive and backup edges are stubbed, and each is defined AFTER
# the source so nothing can quietly replace it.
# shellcheck disable=SC1090
source "$repo_root/installer/src/70-permissions.sh"
die() { echo "$*" >&2; exit 1; }
confirm() { return 0; }
backup_file() { :; }

# A belt-and-braces guard on the redirection itself. If either path ever
# resolves outside the scratch directory, stop before writing: the failure mode
# this protects against is editing the developer's real agent config, which no
# assertion would catch because the test's own fixtures would still look right.
resolved_opencode="$(opencode_configfile)"
case "$resolved_opencode" in
    "$work"/*) ;;
    *) echo "REFUSING TO RUN: opencode config resolved to $resolved_opencode, outside $work" >&2; exit 1 ;;
esac
case "$CLAUDE_CONFIGFILE" in
    "$work"/*) ;;
    *) echo "REFUSING TO RUN: claude config resolved to $CLAUDE_CONFIGFILE, outside $work" >&2; exit 1 ;;
esac

worktrees="$work/config/tsch-ai-worktrees"

# --- claude ------------------------------------------------------------------

printf '{\n  "permissions": {\n    "allow": ["Read(/already/there/**)"]\n  }\n}\n' > "$work/claude.json"
claude_worktrees_permissions "$worktrees" >/dev/null

allow="$(rjq -r '.permissions.allow[]' "$work/claude.json")"

# t_record() records a FINDING, which t_end counts as a failure -- it is not a
# "this passed" call. So the passing path here does nothing at all, and only
# t_fail is reached on a real problem.
#
# Membership is tested with `case` against a newline-padded copy rather than
# `grep -qx`: PORTABILITY.md bans a `printf | grep -q` pipeline (pipefail-grep-q)
# because grep exits at the first match, the writer takes SIGPIPE, and pipefail
# then fails the whole line. The padding makes each comparison whole-line, which
# is what -x was there for.
padded="
$allow
"
has_rule() { # <exact rule text>
    case "$padded" in
        *"
$1
"*) return 0 ;;
    esac
    return 1
}

for verb in Read Edit Write; do
    has_rule "$verb($worktrees/**)" \
        || t_fail "claude grants $verb on the worktree root: no $verb($worktrees/**) in $(printf '%s' "$allow" | tr '\n' ' ')"
done

has_rule "Bash($worktrees/**:*)" \
    || t_fail "claude may run the checkout's own scripts: no Bash($worktrees/**:*) in $(printf '%s' "$allow" | tr '\n' ' ')"

# A pre-existing entry is preserved, not replaced: the writer merges.
t_assert_contains "claude keeps entries it did not add" \
    'Read(/already/there/**)' "$allow"

# Idempotent: a second run adds nothing and does not duplicate.
claude_worktrees_permissions "$worktrees" >/dev/null
t_assert_eq "claude grant is idempotent" \
    "$(rjq -r '.permissions.allow | length' "$work/claude.json")" \
    "$(printf '%s\n' "$allow" | wc -l | tr -d ' ')"

# THE SECURITY BOUNDARY. Every granted pattern must be confined to the
# worktrees root. A pattern naming any ancestor would cover tsch-ai-skills,
# whose tree holds server.key and the installed binaries.
skills_root="$work/config/tsch-ai-skills"
mkdir -p "$skills_root"
overbroad=""
while IFS= read -r rule; do
    case "$rule" in
        *"$worktrees"*) continue ;;
        *"$work/config/"*|*"$work/**"*) overbroad="$overbroad $rule" ;;
    esac
done <<RULES
$allow
RULES
t_assert_eq "no granted pattern reaches outside the worktree root" \
    "$overbroad" ""

# --- opencode ----------------------------------------------------------------

printf '{\n  "$schema": "https://opencode.ai/config.json"\n}\n' > "$work/opencode.json"
opencode_worktrees_permissions "$worktrees" >/dev/null

for tool in read edit write bash; do
    got="$(rjq -r --arg t "$tool" --arg p "$worktrees/**" \
        '.permission[$t][$p] // "MISSING"' "$work/opencode.json")"
    t_assert_eq "opencode allows $tool on the worktree root" "$got" allow
done

# --- the step runs for every install ----------------------------------------

# B234's third property. planning_permission_step is called inside main's
# `contains planning` branch; worktrees_permission_step must be called outside
# it, or a non-planning install grants nothing. Asserted against the generated
# install.sh, which is what a user actually runs.
main_tail="$(awk '/^    if contains planning /,0' "$repo_root/install.sh")"
printf '%s' "$main_tail" \
    | awk '/^    fi$/{seen=1; next} seen && /worktrees_permission_step/{found=1} END{exit !found}' \
    || t_fail "worktrees_permission_step runs outside the planning branch: install.sh calls it only within, or not at all"

t_end
