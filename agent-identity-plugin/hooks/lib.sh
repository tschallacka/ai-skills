#!/usr/bin/env bash
# MODE: PROD
# agent-identity-plugin/hooks/lib.sh -- shared by hooks/subagent-start.sh.
#
# rjq is this repo's real JSON tool. install_shared_rjq (installer/src/
# 20-runtime-tools.sh) copies it to a fixed, install-root-independent path
# as part of installing this plugin -- deliberately not the user's global
# PATH, since none of chat/ai-text-editor/interactive-shell ever promised
# rjq would be there themselves. agent_identity_rjq_bin resolves that path
# (B319: install.sh's own PATH-prepend does not outlive its process, so a
# hook running independently later cannot assume an ambient `rjq`).
agent_identity_rjq_bin() {
    local bin="${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills/bin/rjq"
    if [ -x "$bin" ]; then
        printf '%s\n' "$bin"
        return 0
    fi
    command -v rjq
}

# Builds the additionalContext string for a genuine subagent start, or fails
# with no output when agent_id is empty. Defensive rather than load-bearing:
# SubagentStart should not fire for the main agent at all, but empty input
# must never manufacture a message that claims an identity nobody has.
#
# What it tells the subagent is scoped to what each tool can ACTUALLY honor
# today (T122's own investigation, 2026-09-12):
#   - ai-text-editor-mcp already accepts a per-call agent/session override
#     (ADAPTER_ARGUMENTS in src/ai-text-editor-mcp/src/lib.rs) -- reconnects
#     to that agent's own workspace.
#   - interactive-shell already reads --agent/AGENT_ID as a fallback
#     identity for naming its socket.
#   - chat-mcp does NOT support a per-call override yet: it holds one
#     connection/nick for its whole process lifetime. Saying otherwise here
#     would be actively wrong, so this says plainly that chat still shares
#     the parent's nick for now (see the follow-up TODO filed alongside
#     T122's own closure).
agent_identity_context() { # <agent_id> <agent_type>
    local agent_id="$1" agent_type="${2:-unknown}"
    [ -n "$agent_id" ] || return 1
    printf 'AGENT_ID=%s AGENT_TYPE=%s\n\nThis identity is yours alone for this run -- your parent and any sibling subagent have their own. Use it so their tool state and yours cannot be silently shared:\n\n- ai-text-editor: pass agent: "%s" (or session: "%s") as an argument on every mcp__ai-text-editor__* call, so your tabs stay separate from theirs.\n- interactive-shell: pass --agent %s (or export AGENT_ID=%s once in your own shell) on every interactive-shell / interactive-shell-input call, so your terminal socket stays separate from theirs.\n- chat: chat-mcp does not yet support a per-call identity override -- your chat/wait calls still share your parent'"'"'s nick for now.\n' \
        "$agent_id" "$agent_type" "$agent_id" "$agent_id" "$agent_id" "$agent_id"
}
