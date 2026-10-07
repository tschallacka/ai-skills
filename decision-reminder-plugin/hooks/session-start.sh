#!/usr/bin/env bash
# MODE: PROD
# SessionStart hook: reminds the agent of open questions in the question
# register (DECISIONS.json) -- the one requirement of the whole question-
# register feature that is proactive rather than something an agent or
# person has to go looking for (a question stubbed and left open on a prior
# session must not be forgotten on the next one).
#
# Reads only open questions (decisions list --status open): an answered or
# closed question needs no reminder. Among those, any of urgent priority is
# named individually by id and title, since those are the ones most likely
# to be worth interrupting other work for; the rest are only counted.
#
# Non-blocking and advisory, the same as chat-interrupt-plugin's hook: it
# only ever adds additionalContext, never a permission decision. Unlike that
# hook, truly silent (no stdout at all) when there is nothing to say, rather
# than printing an empty JSON object -- a SessionStart hook with nothing to
# add needs no output, and the fixture test for this script asserts exactly
# that. Any failure to locate the register or the binary is just as silent.
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
[ -n "$open_lines" ] || exit 0

count="$(printf '%s\n' "$open_lines" | wc -l | tr -d ' ')"
plural=''
[ "$count" = 1 ] || plural='s'

text="$count open question$plural in the question register (DECISIONS.json). Non-blocking -- answer when convenient, via the decisions CLI or the decisions-mcp tools."

# Each open line is "ID [Priority/Status] Title (branch)" (decisions list's
# own format); an urgent one is named by id and title. Title extraction
# strips through the first "] " and the trailing " (...)"; a title that
# itself contains a parenthesized group is the one case this heuristic can
# misread, the same class of limitation tui-hint-plugin's own text matching
# already accepts.
urgent_lines="$(printf '%s\n' "$open_lines" | grep -E '^\S+ \[Urgent/' || true)"
if [ -n "$urgent_lines" ]; then
    while IFS= read -r line; do
        [ -n "$line" ] || continue
        id="${line%% *}"
        title="${line#*] }"
        title="${title% (*}"
        text="$text
Urgent: $id -- $title"
    done <<EOF
$urgent_lines
EOF
fi

# Control characters other than newline would make the JSON invalid.
text="$(printf '%s' "$text" | tr -d '\000-\010\013\014\016-\037')"
text="${text//\\/\\\\}"
text="${text//\"/\\\"}"
text="${text//$'\n'/\\n}"

printf '{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"%s"}}' "$text"
