#!/usr/bin/env bash
# MODE: DEV
# test-installer-agent-identity.sh — T122/T123: a Claude Code root cannot
# guarantee which agent is calling a skill-mode binary (measured: a
# subagent's environment there is byte-identical to its parent's), so an
# EXPLICIT request for skill mode on chat/ai-text-editor there is refused by
# name, a DEFAULT resolution is corrected to mcp instead, and either way the
# agent-identity-plugin that makes mcp mode's identity real is installed
# alongside a session-dependent skill.
set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
installer="$repo_root/install.sh"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/installer-agent-identity.XXXXXX")"
trap 'rm -rf "$work"' EXIT

install_into() { # <home> <target-subpath> [extra args...]
    local home="$1" target="$2"
    shift 2
    RUN_RC=0
    env -u XDG_CONFIG_HOME HOME="$home" "$BASH" "$installer" --target "$home/$target" \
        --yes --dev-build "$@" >"$work/out" 2>"$work/err" </dev/null || RUN_RC=$?
    RUN_OUT="$(cat "$work/out")"
    RUN_ERR="$(cat "$work/err")"
}

# ── an explicit skill-mode request for chat on a Claude Code root is refused ─
home="$work/explicit-refused"
install_into "$home" .claude/skills --skill chat --integration chat=skill
[ "$RUN_RC" -ne 0 ] || t_fail "an explicit chat=skill install on a Claude Code root must not exit 0, got $RUN_RC"
case "$RUN_ERR" in
    *'Refusing chat in skill mode'*) ;;
    *) t_fail "the refusal did not name chat by name: $RUN_ERR" ;;
esac
case "$RUN_ERR" in
    *'byte-identical'*) ;;
    *) t_fail "the refusal did not say why (byte-identical environment): $RUN_ERR" ;;
esac
[ ! -e "$home/.claude/skills/chat/SKILL.md" ] || t_fail "a refused skill-mode install still placed chat's skill files"

# ── a DEFAULT chat install on a Claude Code root becomes mcp, not skill ─────
home="$work/default-becomes-mcp"
install_into "$home" .claude/skills --skill chat
[ "$RUN_RC" -eq 0 ] || t_fail "a default chat install must succeed (as mcp), exited $RUN_RC: $RUN_ERR"
case "$RUN_ERR" in
    *'chat installs as mcp on'*) ;;
    *) t_fail "the default-to-mcp correction was not announced: $RUN_ERR" ;;
esac
found_chat_mcp=""
for candidate in "$home/.claude/skills/chat/bin"/*/chat-mcp*; do
    [ -f "$candidate" ] || continue
    found_chat_mcp="$candidate"
    break
done
[ -n "$found_chat_mcp" ] || t_fail "a default chat install on Claude Code did not place the mcp binary"

# ── the plugin is installed alongside a session-dependent skill on Claude Code
[ -f "$home/.claude/skills/agent-identity-plugin/.claude-plugin/plugin.json" ] \
    || t_fail "the agent-identity-plugin was not installed alongside chat on a Claude Code root"
[ -x "$home/.claude/skills/agent-identity-plugin/hooks/pre-tool-use.sh" ] \
    || t_fail "the plugin's pre-tool-use.sh was not installed executable"

# ── a non-Claude-Code root is unaffected: skill mode installs, no plugin ───
home="$work/codex-unaffected"
install_into "$home" .codex/skills --skill chat --integration chat=skill
[ "$RUN_RC" -eq 0 ] || t_fail "an explicit chat=skill install on a codex root must succeed, exited $RUN_RC: $RUN_ERR"
[ -f "$home/.codex/skills/chat/SKILL.md" ] \
    || t_fail "chat's own skill-mode files were not placed on a codex root"
[ ! -e "$home/.codex/skills/agent-identity-plugin" ] \
    || t_fail "the plugin was installed on a codex root, which has no SubagentStart hook to register"

# ── interactive-shell is unaffected by the refusal (no MCP alternative) ────
home="$work/interactive-shell-unaffected"
install_into "$home" .claude/skills --skill interactive-shell
[ "$RUN_RC" -eq 0 ] || t_fail "an interactive-shell install must not be refused (no mcp alternative exists), exited $RUN_RC: $RUN_ERR"
[ -f "$home/.claude/skills/interactive-shell/SKILL.md" ] \
    || t_fail "interactive-shell's own files were not placed"
[ -f "$home/.claude/skills/agent-identity-plugin/.claude-plugin/plugin.json" ] \
    || t_fail "the plugin's SOFT half (SubagentStart context) was not offered to interactive-shell"

[ "$(t_failures)" -eq 0 ] || exit 1
printf '%s\n' 'test-installer-agent-identity: PASS'
