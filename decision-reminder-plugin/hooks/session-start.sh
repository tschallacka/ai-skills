#!/usr/bin/env bash
# MODE: PROD
# SessionStart hook: reminds the agent of open questions in the question
# register (DECISIONS.json), and separately names every DECIDED question
# still awaiting implementation -- the one requirement of the whole
# question-register feature that is proactive rather than something an agent
# or person has to go looking for. An open question is the user's to answer;
# a decided one has already been answered and is the agent's own outstanding
# work, so it is named individually rather than only counted, regardless of
# priority, and the wording tells the agent to act rather than merely to
# notice.
#
# Reads open and decided questions separately (decisions list --status open
# / --status decided): an implemented, closed, dropped or obsolete question
# needs no reminder. Among open questions, only urgent ones are named
# individually, same as before; every decided question is named, since each
# one is itself a piece of outstanding work regardless of priority.
#
# Non-blocking and advisory, the same as chat-interrupt-plugin's hook: it
# only ever adds additionalContext, never a permission decision. Unlike that
# hook, truly silent (no stdout at all) when there is nothing to say for
# either kind -- the fixture test for this script asserts exactly that. Any
# failure to locate the register or the binary is just as silent.
set -uo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=decision-reminder-plugin/hooks/lib.sh
source "$script_dir/lib.sh"

register="$(decision_reminder_hook_register_path)"
[ -f "$register" ] || exit 0

bin="$(decision_reminder_hook_decisions_bin)" || exit 0

open_lines="$("$bin" list --status open --file "$register" 2>/dev/null)"
open_lines="$(printf '%s\n' "$open_lines" | sed '/^$/d')"

decided_lines="$("$bin" list --status decided --file "$register" 2>/dev/null)"
decided_lines="$(printf '%s\n' "$decided_lines" | sed '/^$/d')"

[ -n "$open_lines" ] || [ -n "$decided_lines" ] || exit 0

text=''

if [ -n "$open_lines" ]; then
    count="$(printf '%s\n' "$open_lines" | wc -l | tr -d ' ')"
    plural=''
    [ "$count" = 1 ] || plural='s'
    text="$count open question$plural in the question register (DECISIONS.json). Non-blocking -- answer when convenient, via the decisions CLI or the decisions-mcp tools."

    urgent_lines="$(printf '%s\n' "$open_lines" | grep -E '^\S+ \[Urgent/' || true)"
    if [ -n "$urgent_lines" ]; then
        while IFS= read -r line; do
            [ -n "$line" ] || continue
            text="$text
Urgent: $line"
        done <<EOF
$(decision_reminder_hook_named_lines "$urgent_lines")
EOF
    fi
fi

if [ -n "$decided_lines" ]; then
    count="$(printf '%s\n' "$decided_lines" | wc -l | tr -d ' ')"
    plural=''
    [ "$count" = 1 ] || plural='s'
    [ -z "$text" ] || text="$text
"
    text="${text}$count decided question$plural awaiting implementation -- the user already picked an option; this is the agent's own outstanding work, not just a record of the user's choice. Implement it now or at your next natural pause, then record it with \`decisions implement\` or the decisions-mcp implement tool."
    while IFS= read -r line; do
        [ -n "$line" ] || continue
        text="$text
$line"
    done <<EOF
$(decision_reminder_hook_named_lines "$decided_lines")
EOF
fi

# Control characters other than newline would make the JSON invalid.
text="$(printf '%s' "$text" | tr -d '\000-\010\013\014\016-\037')"
text="${text//\\/\\\\}"
text="${text//\"/\\\"}"
text="${text//$'\n'/\\n}"

printf '{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"%s"}}' "$text"
