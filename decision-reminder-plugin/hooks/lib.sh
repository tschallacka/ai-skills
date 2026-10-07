#!/usr/bin/env bash
# MODE: PROD
# decision-reminder-plugin/hooks/lib.sh -- shared by hooks/session-start.sh.
#
# Three independent pieces of logic factored out so they can be tested
# directly (same convention as chat-interrupt-plugin/hooks/lib.sh and
# tui-hint-plugin/hooks/lib.sh): where the register lives, where the compiled
# `decisions` binary this hook shells out to lives, and turning a `decisions
# list` output line into the "<id> -- <title>" form both the open-urgent and
# decided sections of the reminder name questions by.

# decision_reminder_hook_register_path -- DECISIONS.json's path, the same
# fallback order the decisions binary's own resolve_path uses (src/decisions/
# src/main.rs): DECISIONS_JSON if set and non-empty, else the literal
# "DECISIONS.json" in the current directory -- which for a SessionStart hook
# is the project root Claude Code was started in, so no separate upward
# search is needed.
decision_reminder_hook_register_path() {
    if [ -n "${DECISIONS_JSON:-}" ]; then
        printf '%s\n' "$DECISIONS_JSON"
        return 0
    fi
    printf '%s\n' "DECISIONS.json"
}

# decision_reminder_hook_decisions_bin -- the compiled `decisions` binary's
# path, or a non-zero exit with nothing printed when it cannot be found.
# AI_SKILLS_BIN_ROOT wins when an install states it explicitly (T72's shared
# .env manifest does this); otherwise the shared install location every
# skill's compiled binaries live in
# (${XDG_CONFIG_HOME:-~/.config}/tsch-ai-skills/bin, src/installer/src/
# shared_bin.rs). Never PATH: nothing ever promised an ambient `decisions`
# would be there, the same reasoning tui_hint_rjq_bin already applies to rjq.
decision_reminder_hook_decisions_bin() {
    local root candidate
    for root in "${AI_SKILLS_BIN_ROOT:-}" "${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills/bin"; do
        [ -n "$root" ] || continue
        for candidate in "$root/decisions" "$root/decisions.exe"; do
            if [ -x "$candidate" ]; then
                printf '%s\n' "$candidate"
                return 0
            fi
        done
    done
    return 1
}

# decision_reminder_hook_named_lines <lines> -- one "<id> -- <title>" per
# input line, where each input line is "ID [Priority/Status] Title (branch)"
# (decisions list's own format). Title extraction strips through the first
# "] " and the trailing " (...)"; a title that itself contains a
# parenthesized group is the one case this heuristic can misread, the same
# class of limitation tui-hint-plugin's own text matching already accepts.
decision_reminder_hook_named_lines() {
    local lines="$1" line id title
    while IFS= read -r line; do
        [ -n "$line" ] || continue
        id="${line%% *}"
        title="${line#*] }"
        title="${title% (*}"
        printf '%s -- %s\n' "$id" "$title"
    done <<EOF
$lines
EOF
}
