#!/usr/bin/env bash
# MODE: DEV
# test-chat-interrupt-hook -- chat-interrupt-plugin's PreToolUse hook shows
# what the chat bridge queued for this Claude Code session, once, as valid
# JSON, and stays silent (and non-blocking) when nothing is queued (same
# convention as tui-hint-plugin/tests/test-tui-hint-matching.sh).
#
# The hook is what makes interrupts reach an agent without Claude Code's
# channels flag, so the claims are about its output: a reminder naming each
# notice, the spool emptied by reading it, only THIS session's spool read, and
# nothing at all otherwise. Delete the hook's `mv` and the "shown once" case
# fails; delete the session check and the "another session" case fails. It also
# covers the re-arm reminder (chat-mcp's own .active marker plus a stale or
# missing chat-spool-watch heartbeat) and lib.sh's own staleness check.
#
# Usage:
#   test-chat-interrupt-hook.sh

set -uo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
plugin_dir="$(cd "$tests_dir/.." && pwd)"
repo_root="$(cd "$plugin_dir/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$repo_root/planning/tests/lib-test.sh"
t_begin

hook="$plugin_dir/hooks/pre-tool-use.sh"
# shellcheck source=chat-interrupt-plugin/hooks/lib.sh
source "$plugin_dir/hooks/lib.sh"
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

# ---- lib.sh's own is_stale/backdate, tested directly -----------------------
stale_probe="$(mktemp "${TMPDIR:-/tmp}/chat-interrupt-stale-probe.XXXXXX")"
if chat_interrupt_hook_is_stale "$work/no-such-file" 60; then
    :
else
    t_fail 'a missing file must be reported as stale'
fi
: >"$stale_probe"
if chat_interrupt_hook_is_stale "$stale_probe" 60; then
    t_fail 'a file just written must not be reported as stale'
fi
chat_interrupt_hook_backdate "$stale_probe" 120
if chat_interrupt_hook_is_stale "$stale_probe" 60; then
    :
else
    t_fail 'a file backdated past the window must be reported as stale'
fi
rm -f "$stale_probe"

# ---- the re-arm reminder: chat-mcp's .active marker plus a stale or missing
#      chat-spool-watch heartbeat -------------------------------------------
rearm_spool="$work/interrupts/sess-rearm"
mkdir -p "$rearm_spool"

t_assert_eq 'no .active marker means no reminder, whatever the heartbeat' \
    "$(run_hook sess-rearm)" '{}'

: >"$rearm_spool/.active"
first_reminder="$(run_hook sess-rearm)"
t_assert_contains 'a missing heartbeat is reminded about' 'No chat-spool-watch is watching' "$first_reminder"
t_assert_contains 'the reminder names the command' 'chat-spool-watch' "$first_reminder"
t_assert_contains 'the reminder says it will not restart itself' 'does not restart itself' "$first_reminder"

t_assert_eq 'the reminder does not repeat inside its cooldown' \
    "$(run_hook sess-rearm)" '{}'

# A fresh heartbeat silences it even once the cooldown alone would allow one.
chat_interrupt_hook_backdate "$rearm_spool/.rearm-reminded" 999999
: >"$rearm_spool/.watcher"
t_assert_eq 'a fresh heartbeat means no reminder' "$(run_hook sess-rearm)" '{}'

# The heartbeat going stale again, past the cooldown, reminds again.
chat_interrupt_hook_backdate "$rearm_spool/.watcher" 999999
chat_interrupt_hook_backdate "$rearm_spool/.rearm-reminded" 999999
t_assert_contains 'a stale heartbeat past the cooldown reminds again' \
    'No chat-spool-watch is watching' "$(run_hook sess-rearm)"

# A real notice and the reminder can arrive together, in one reply.
chat_interrupt_hook_backdate "$rearm_spool/.rearm-reminded" 999999
printf '[10:05:00Z] #ops <ci> build failed\n' >"$rearm_spool/junkbox.log"
combined="$(run_hook sess-rearm)"
t_assert_contains 'the notice is still shown alongside the reminder' 'build failed' "$combined"
t_assert_contains 'and the reminder alongside the notice' 'No chat-spool-watch is watching' "$combined"

# Clearing .active (the agent removed its rules and timers) silences it even
# with a stale heartbeat and the cooldown elapsed.
rm -f "$rearm_spool/.active"
chat_interrupt_hook_backdate "$rearm_spool/.rearm-reminded" 999999
t_assert_eq 'nothing configured means no reminder' "$(run_hook sess-rearm)" '{}'

t_end 'test-chat-interrupt-hook'
