#!/usr/bin/env bash
# MODE: DEV
# Supervision-frame test.
#
# Asserts the bounded supervision-frame emitter (planning/scripts/
# supervision-frame.sh) and the gated monitor reader (planning/scripts/
# monitor-read.sh):
#   - frames are bounded (over-budget write refused),
#   - each subagent's frame footer-overwrites (only the latest persists),
#   - grant-log entries carry case + command and never reasoning fields,
#   - the monitor reader is maintainer-only (fail-closed identity),
#   - a green frame is ~zero context and an escalated frame signals
#     pull-on-exception.

set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
# Report the failing expression: these assertions are bare `[ ... ]` under
# set -e, which otherwise exits 1 in silence.
t_trap_assertions


root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
frame_sh="$root/scripts/supervision-frame.sh"
monitor_sh="$root/scripts/monitor-read.sh"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
frame="$tmp/frame"
grants="$tmp/grants.log"

pass=0; fail=0
chk() {
    if [ "$1" = "$2" ]; then pass=$((pass+1)); echo "PASS: $3"; else fail=$((fail+1)); echo "FAIL: $3 (got $1, want $2)"; fi
}

# 1. Frame emittable and bounded.
frame_out="$("$BASH" "$frame_sh" write "$frame" --subagent reviewer-b --persona christoph --status ok --verdict "clean" 2>&1)"; chk $? 0 "write ok frame"
"$BASH" "$frame_sh" check "$frame" 2048 >/dev/null 2>&1; chk $? 0 "frame within budget"

# 2. Over-budget write refused.
over_rc=0
if FRAME_BUDGET=32 "$BASH" "$frame_sh" write "$frame" --subagent x --persona y --status ok --verdict "$(printf 'a%.0s' $(seq 1 100))" >/dev/null 2>&1; then
    over_rc=0
else
    over_rc=$?
fi
chk "$over_rc" 64 "over-budget frame refused"

# 3. Footer-overwrite: only the latest frame persists.
"$BASH" "$frame_sh" write "$frame" --subagent reviewer-b --persona christoph --status ok --verdict "first" >/dev/null
"$BASH" "$frame_sh" write "$frame" --subagent reviewer-b --persona christoph --status escalated --verdict "second" >/dev/null
[ "$(grep -c '^subagent:' "$frame")" -eq 1 ] && echo "PASS: footer-overwrite (one frame)" || { echo "FAIL: footer-overwrite"; fail=$((fail+1)); }
grep -q 'verdict: second' "$frame" && echo "PASS: latest frame wins" || { echo "FAIL: latest frame"; fail=$((fail+1)); }

# 4. Grant log: case + command, no reasoning fields.
"$BASH" "$frame_sh" grant "$grants" reviewer-b christian --case "needs path access" --command "ROLE_ID=maintainer role-context.sh --paths chris" >/dev/null
[ -f "$grants" ] || { echo "FAIL: grant log written"; fail=$((fail+1)); }
grep -q 'grant' "$grants" && grep -q 'needs path access' "$grants" && grep -q 'role-context.sh --paths chris' "$grants" \
    && echo "PASS: grant log has case + command" || { echo "FAIL: grant log fields"; fail=$((fail+1)); }
grep -qEi 'reasoning|because|since|therefore' "$grants" && { echo "FAIL: grant log leaked reasoning"; fail=$((fail+1)); } \
    || echo "PASS: grant log has no reasoning"

# 5. Monitor reader: maintainer-only identity (fail closed).
in="$tmp/in-frame"; cp "$frame" "$in"
rc_deny=0; if ROLE_ID=chris "$BASH" "$monitor_sh" show "$in" >/dev/null 2>&1; then rc_deny=0; else rc_deny=$?; fi
chk "$rc_deny" 64 "non-maintainer refused"
rc_nodesc=0; if "$BASH" "$monitor_sh" show "$in" >/dev/null 2>&1; then rc_nodesc=0; else rc_nodesc=$?; fi
chk "$rc_nodesc" 64 "no-ROLE_ID refused"
ROLE_ID=maintainer "$BASH" "$monitor_sh" show "$in" >/dev/null 2>&1; chk $? 0 "maintainer allowed"

# 6. Pull-on-exception signal.
verify_out="$(ROLE_ID=maintainer "$BASH" "$monitor_sh" verify "$in" 2>/dev/null || true)"
case "$verify_out" in
    *PULL-ON-EXCEPTION*) echo "PASS: escalated frame signals pull-on-exception" ;;
    *) echo "FAIL: pull-on-exception"; fail=$((fail+1)) ;;
esac

echo
echo "supervision-frame: $pass passed, $fail failed"
[ "$fail" -eq 0 ] && echo "test-supervision-frame: PASS" || exit 1
