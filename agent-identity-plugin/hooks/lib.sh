#!/usr/bin/env bash
# MODE: PROD
# agent-identity-plugin/hooks/lib.sh - shared by both hook scripts, and
# mirrored byte-for-byte by register_dir()/register_path() in
# src/agent-session-key/src/lib.rs. Both sides must compute the identical
# path from nothing but the environment and a session id, since the writer
# (this hook, a child of the coding harness) and the reader (an MCP server,
# also a child of the same harness) share no other channel.
#
# AI_SKILLS_AGENT_IDENTITY_DIR overrides the directory outright, for a
# machine where XDG_STATE_HOME/HOME are not writable or not what a test
# wants isolated.
agent_identity_register_dir() {
    if [ -n "${AI_SKILLS_AGENT_IDENTITY_DIR:-}" ]; then
        printf '%s\n' "$AI_SKILLS_AGENT_IDENTITY_DIR"
        return
    fi
    printf '%s/ai-skills/agent-identity\n' "${XDG_STATE_HOME:-$HOME/.local/state}"
}

# One file per Claude Code session, named by the session id both the hook
# payload (session_id) and the MCP server's own environment
# (CLAUDE_CODE_SESSION_ID) carry -- so the two processes agree on a path
# without a session ID ever crossing any channel but "the same variable,
# read twice."
agent_identity_register_path() {
    local session_id="$1"
    printf '%s/%s.jsonl\n' "$(agent_identity_register_dir)" "$session_id"
}

# A register left behind by a session that ended keeps no purpose and would
# grow the directory forever; prune anything not written to in a week. Cheap
# enough to run on every hook invocation rather than needing its own cron.
agent_identity_prune_stale() {
    local dir
    dir="$(agent_identity_register_dir)"
    [ -d "$dir" ] || return 0
    find "$dir" -maxdepth 1 -name '*.jsonl' -mtime +7 -delete 2>/dev/null || true
}
