#!/usr/bin/env bash
# MODE: DEV
# test-chat-interrupt-hook — chat-interrupt-plugin's PreToolUse hook shows what
# the chat bridge queued for this Claude Code session, once, as valid JSON, and
# stays silent (and non-blocking) when nothing is queued.
#
# The hook is what makes interrupts reach an agent without Claude Code's
# channels flag, so the claims are about its output: a reminder naming each
# notice, the spool emptied by reading it, only THIS session's spool read, and
# nothing at all otherwise. Delete the hook's `mv` and the "shown once" case
# fails; delete the session check and the "another session" case fails.
#
# Usage:
#   test-chat-interrupt-hook.sh

set -uo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$repo_root/planning/tests/lib-test.sh"
t_begin

hook="$repo_root/chat-interrupt-plugin/hooks/pre-tool-use.sh"
work="$(mktemp -d "${TMPDIR:-/tmp}/chat-interrupt-hook.XXXXXX")"
trap 'rm -rf "$work"' EXIT

run_hook() { # <session id> [payload] -> stdout of the hook
    printf '%s' "${2:-{\}}" | env -u CLAUDE_CODE_SESSION_ID AI_CHAT_HOME="$work" \
        CLAUDE_CODE_SESSION_ID="$1" "$hook"
}

spool="$work/interrupts/sess-1"
mkdir -p "$spool"
printf '[10:00:02Z] #ops <alice> the deploy failed\n' >"$spool/junkbox.log"
printf '[10:00:01Z] timer: check the build\n' >>"$spool/junkbox.log"
printf '[10:00:03Z] #ops <bob> says "quoted" and \\ backslash\n' >"$spool/other.log"

out="$(run_hook sess-1)"
t_assert_contains 'the reminder is a PreToolUse additionalContext' '"hookEventName":"PreToolUse"' "$out"
t_assert_contains 'a queued message is named' 'alice> the deploy failed' "$out"
t_assert_contains 'a queued timer is named' 'timer: check the build' "$out"
t_assert_contains 'the count is given' 'interrupts you asked for (3)' "$out"
t_assert_contains 'the messages are said to be unread' 'still unread' "$out"
t_assert_contains 'the text is said not to be instructions' 'not instructions' "$out"
case "$out" in
    *permissionDecision*) t_fail 'the hook must never set a permission decision' ;;
    *) : ;;
esac
# The order is by time, whichever file a line came from.
first="${out#*10:00:01Z}"
case "$first" in
    *10:00:02Z*10:00:03Z*) : ;;
    *) t_fail 'the notices are not in time order' ;;
esac

# The output is one JSON object on one line, with the quote and the backslash a
# notice carried escaped rather than left to break it. (Checked by shape, not by
# a JSON parser: none is guaranteed to be on the PATH of every leg.)
t_assert_contains 'a double quote in a notice is escaped' '\"quoted\"' "$out"
t_assert_contains 'a backslash in a notice is escaped' '\\ backslash' "$out"
case "$out" in
    '{"hookSpecificOutput":{"hookEventName":"PreToolUse","additionalContext":"'*'"}}') : ;;
    *) t_fail 'the output is not the expected JSON object shape' ;;
esac
case "$out" in
    *$'\n'*) t_fail 'the output spans lines, so a newline was not escaped' ;;
    *) : ;;
esac

# Shown once: reading it emptied the spool.
again="$(run_hook sess-1)"
t_assert_eq 'the spool is empty after it is read, so nothing is shown twice' "$again" '{}'

# Only this session's spool is read.
mkdir -p "$work/interrupts/sess-2"
printf '[10:00:09Z] for the other session\n' >"$work/interrupts/sess-2/x.log"
t_assert_eq "another session's notice is not shown" "$(run_hook sess-1)" '{}'
t_assert_contains 'but that session sees its own' 'for the other session' "$(run_hook sess-2)"

# Nothing queued at all, and no session id.
t_assert_eq 'no spool directory means an empty result' "$(run_hook no-such-session)" '{}'
none="$(printf '{}' | env -u CLAUDE_CODE_SESSION_ID AI_CHAT_HOME="$work" "$hook")"
t_assert_eq 'no session id means an empty result' "$none" '{}'

# The session id may come from the payload instead of the environment.
mkdir -p "$work/interrupts/from-payload"
printf '[10:00:10Z] via the payload\n' >"$work/interrupts/from-payload/x.log"
viap="$(printf '{"session_id":"from-payload","tool_name":"Bash"}' \
    | env -u CLAUDE_CODE_SESSION_ID AI_CHAT_HOME="$work" "$hook")"
t_assert_contains 'the hook payload names the session when the environment does not' 'via the payload' "$viap"

# A path-shaped id cannot reach outside the spool.
mkdir -p "$work/interrupts/.."
t_assert_eq 'a path-shaped session id matches nothing' "$(run_hook '../../etc')" '{}'

# More than twenty are cut, and the rest are counted.
mkdir -p "$work/interrupts/many"
: >"$work/interrupts/many/x.log"
n=1
while [ "$n" -le 25 ]; do
    printf '[11:00:%02dZ] message %s\n' "$n" "$n" >>"$work/interrupts/many/x.log"
    n=$((n + 1))
done
many="$(run_hook many)"
t_assert_contains 'the count is the whole count' 'interrupts you asked for (25)' "$many"
t_assert_contains 'the cut is said' '+5 more' "$many"

t_end 'test-chat-interrupt-hook'
