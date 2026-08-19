#!/usr/bin/env bash
# test-install-ui — the installer's full-screen picker stays inside the terminal
# and keeps its contract when nobody is watching it.
#
# Usage: test-install-ui.sh
#
# Every assertion runs headless: install-ui.sh's --render mode draws one frame to
# stdout for a fixed state, and the key/mouse handlers are driven directly or fed
# synthetic bytes on fd 3. The groups, in order:
#   1. Line width, at 60/80/200 and a narrow 40 that must degrade to one pane.
#   2. The list scrolls when the skills outgrow the rows, cursor still visible.
#   3. The checkbox reflects selection; Enter, Space and a click all toggle.
#   4. Tab moves focus, and the focused pane is marked without relying on colour.
#   5. The info pane's actions are only marked usable when it has focus.
#   6. Reverifying one tool updates every skill sharing it (the global cache).
#   6b. hard requirements block the install, soft ones only degrade it, and the
#      three states are distinguishable with and without colour.
#   7. The ASCII fallback has no escape or non-ASCII byte, and drops the sprite.
#   8. Only the two eye rows change between two eye states.
#   9. The restoration sequences are emitted on INT, TERM and a plain exit.
#   10. The installer seam: a blocked skill is never preselected, fd 3 without a
#      tty declines with 69, and install.sh's own EXIT trap survives the picker.
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ui="$repo_root/install-ui.sh"

note_fail() { printf 'install-ui: %s\n' "$1" >&2; t_record "$1"; }
note_pass() { printf 'PASS %s\n' "$1"; }

[ -f "$ui" ] || { note_fail "missing $ui"; exit 1; }
# shellcheck source=/dev/null
. "$ui"

has_non_ascii() {
    [ -n "$(printf '%s' "$1" | tr -d ' -~')" ]
}

# ── 1. Width discipline ──────────────────────────────────────────────────────
check_widths() {
    local label="$1" width="$2" height="$3"
    shift 3
    local out line n=0 bad=0
    out="$(bash "$ui" --render --width "$width" --height "$height" "$@")"
    while IFS= read -r line; do
        n=$((n + 1))
        iui_plain "$line"
        if [ "${#IUI_PLAIN}" -ne "$width" ]; then
            note_fail "$label line $n is ${#IUI_PLAIN} cells, want $width"
            bad=1
        fi
        if has_non_ascii "$IUI_PLAIN"; then
            note_fail "$label line $n holds a glyph outside the documented set"
            bad=1
        fi
    done <<< "$out"
    if [ "$n" -ne "$height" ]; then
        note_fail "$label produced $n lines, want $height"
        bad=1
    fi
    [ "$bad" -eq 0 ] && note_pass "$label fits ${width}x${height}"
    return 0
}

for w in 60 80 200; do
    check_widths "wide/colour" "$w" 40 --color truecolor
    check_widths "wide/mono" "$w" 24 --color none --glyphs ascii
    check_widths "wide/info-focus" "$w" 40 --color truecolor --focus info
done
check_widths "narrow/list" 40 24 --color truecolor
check_widths "narrow/info" 40 24 --color truecolor --focus info
check_widths "narrow/ascii" 40 12 --color none --glyphs ascii --focus info

# The 40-column case must genuinely degrade to one pane, not squeeze two.
IUI_COLS=40
IUI_ROWS=24
iui_layout
[ "$IUI_NARROW" -eq 1 ] || note_fail 'a 40-column terminal must use the single-pane degrade path'
[ "$IUI_NARROW" -eq 1 ] && note_pass '40 columns degrades to one pane'

# ── 2. Scrolling keeps the cursor visible ────────────────────────────────────
iui_load_demo
IUI_COLS=80
IUI_ROWS=8
IUI_CURSOR=5
IUI_SCROLL=0
iui_layout
iui_clamp_scroll
[ "$IUI_BODY_ROWS" -eq 4 ] || note_fail "expected 4 body rows at 8 lines, got $IUI_BODY_ROWS"
[ "$IUI_SCROLL" -eq 2 ] || note_fail "expected scroll 2 with cursor 5 in 4 rows, got $IUI_SCROLL"
[ "$IUI_CURSOR" -ge "$IUI_SCROLL" ] \
    && [ "$IUI_CURSOR" -lt $((IUI_SCROLL + IUI_BODY_ROWS)) ] \
    || note_fail 'the cursor scrolled out of the visible window'
frame="$(bash "$ui" --render --width 80 --height 8 --cursor 5 --color none --glyphs ascii)"
case "$frame" in
    *'>[ ] install-ui'*) ;;
    *) note_fail 'the cursor row is not visible after scrolling' ;;
esac
case "$frame" in
    *'] planning'*) note_fail 'the list did not scroll: row 0 is still drawn' ;;
esac
[ "$(t_failures)" -eq 0 ] && note_pass 'the list scrolls and keeps the cursor visible'

# ── 3. Checkbox and the three ways to toggle ─────────────────────────────────
frame="$(bash "$ui" --render --width 80 --height 24 --color none --glyphs ascii)"
case "$frame" in
    *'>[#] planning'*) ;;
    *) note_fail 'a selected skill must render a filled checkbox' ;;
esac
case "$frame" in
    *' [ ] project-spec'*) ;;
    *) note_fail 'an unselected skill must render an empty checkbox' ;;
esac
note_pass 'the checkbox reflects selection'

iui_load_demo
IUI_COLS=80
IUI_ROWS=24
IUI_CURSOR=0
IUI_SCROLL=0
iui_layout
iui_handle_key ENTER
[ "${IUI_SKILL_SEL[0]}" -eq 0 ] || note_fail 'Enter did not toggle the cursor row off'
iui_handle_key SPACE
[ "${IUI_SKILL_SEL[0]}" -eq 1 ] || note_fail 'Space did not toggle the cursor row on'
IUI_MOUSE_BTN=0
IUI_MOUSE_COL=4
IUI_MOUSE_ROW=4
IUI_MOUSE_RELEASE=0
iui_handle_key MOUSE
[ "$IUI_CURSOR" -eq 1 ] || note_fail "a click must move the cursor, got $IUI_CURSOR"
[ "${IUI_SKILL_SEL[1]}" -eq 1 ] || note_fail 'a click did not toggle the row it landed on'
note_pass 'Enter, Space and a click all toggle the checkbox'

# A wheel event and a drag are parsed and then deliberately ignored.
IUI_MOUSE_BTN=64
iui_handle_key MOUSE
[ "$IUI_CURSOR" -eq 1 ] || note_fail 'a wheel event must be ignored, not move the cursor'
IUI_MOUSE_BTN=32
iui_handle_key MOUSE
[ "$IUI_CURSOR" -eq 1 ] || note_fail 'a drag must be ignored, not move the cursor'
note_pass 'wheel and drag events are ignored'

# The escape and SGR-mouse parsers, driven with synthetic bytes on fd 3.
scratch="$(mktemp -d "${TMPDIR:-/tmp}/install-ui-test.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT
printf '\033[B\033[A \033[<0;30;5M\033[Zq' > "$scratch/keys"
exec 3<"$scratch/keys"
expected=(DOWN UP SPACE MOUSE SHIFTTAB q EOF)
for want in "${expected[@]}"; do
    iui_read_key
    [ "$IUI_KEY" = "$want" ] || note_fail "key parser read '$IUI_KEY', want '$want'"
done
[ "$IUI_MOUSE_COL" = "30" ] && [ "$IUI_MOUSE_ROW" = "5" ] \
    || note_fail "SGR mouse parsed col=$IUI_MOUSE_COL row=$IUI_MOUSE_ROW, want 30/5"
exec 3<&-
note_pass 'the fd-3 escape and SGR-mouse parsers decode a synthetic stream'

# ── 4. Focus ─────────────────────────────────────────────────────────────────
list_frame="$(bash "$ui" --render --width 80 --height 24 --color none --glyphs ascii --focus list)"
info_frame="$(bash "$ui" --render --width 80 --height 24 --color none --glyphs ascii --focus info)"
case "$list_frame" in
    *'[SKILLS]'*) ;;
    *) note_fail 'the focused list pane must be marked [SKILLS]' ;;
esac
case "$list_frame" in
    *' DETAILS '*) ;;
    *) note_fail 'the unfocused info pane must not be bracketed' ;;
esac
case "$info_frame" in
    *'[DETAILS]'*) ;;
    *) note_fail 'the focused info pane must be marked [DETAILS]' ;;
esac
case "$info_frame" in
    *' SKILLS '*) ;;
    *) note_fail 'the unfocused list pane must not be bracketed' ;;
esac
IUI_FOCUS=list
iui_handle_key $'\t'
[ "$IUI_FOCUS" = "info" ] || note_fail 'Tab did not move focus to the info pane'
iui_handle_key $'\t'
[ "$IUI_FOCUS" = "list" ] || note_fail 'Tab did not move focus back to the list'
iui_handle_key SHIFTTAB
[ "$IUI_FOCUS" = "info" ] || note_fail 'Shift-Tab did not move focus'
note_pass 'Tab moves focus and the focused pane is marked differently'

# ── 5. The actions are usable only under focus ───────────────────────────────
case "$info_frame" in
    *'> d  help me install dependencies'*) ;;
    *) note_fail 'a focused info pane must mark its actions usable' ;;
esac
case "$info_frame" in
    *'> r  reverify dependencies'*) ;;
    *) note_fail 'the reverify action is missing from the focused info pane' ;;
esac
case "$list_frame" in
    *'- d  help me install dependencies'*) ;;
    *) note_fail 'an unfocused info pane must still list its actions, marked unusable' ;;
esac
note_pass 'the info actions become usable only when the pane has focus'

# ── 6. The dependency cache is global, keyed per tool ────────────────────────
# Skills 0 and 4 both need jq, so reverifying from one must change the other.
# The probe is overridden rather than uninstalling jq.
iui_load_demo
iui_req_reset
IUI_DEP_TOOLS=(); IUI_DEP_STATES=()
iui_req_add 0 jq '*' hard 'plan validation refuses to run'
iui_req_add 2 memlimit '*' soft 'memory cap not enforced; CPU limiting still works'
iui_req_add 4 jq '*' hard 'the finding tables cannot be assembled'
iui_dep_probe() { return 1; }
iui_dep_reverify_skill 0
iui_dep_state jq
[ "$IUI_DEP_STATE" = "missing" ] || note_fail "reverify left jq as '$IUI_DEP_STATE'"
iui_skill_state 4
[ "$IUI_SKILL_STATE" = "blocked" ] \
    || note_fail 'reverifying jq from skill 0 did not reach skill 4, so the cache is per-skill'
[ "${#IUI_DEP_TOOLS[@]}" -eq 1 ] \
    || note_fail "the cache holds ${#IUI_DEP_TOOLS[@]} records; jq must not be stored twice"
iui_dep_reverify_skill 2
IUI_CURSOR=4
IUI_COLS=80
IUI_ROWS=40
COLOR_MODE=none
iui_set_glyphs ascii
iui_layout
iui_info_lines "$IUI_RIGHT_W"
found=0
for line in "${IUI_INFO_TEXT[@]}"; do
    case "$line" in *'jq'*'missing'*) found=1 ;; esac
done
[ "$found" -eq 1 ] || note_fail "skill 4's info pane still shows jq as met"
note_pass 'reverifying one tool refreshes every skill that shares it'

# ── 6b. hard blocks, soft only degrades ──────────────────────────────────────
# Fixture from 6: jq (hard) and memlimit (soft) both missing, so skill 4 is
# blocked, skill 2 is degraded and skill 1 is fine.
iui_skill_state 1
[ "$IUI_SKILL_STATE" = "ok" ] || note_fail "a skill with no requirements is '$IUI_SKILL_STATE'"
iui_skill_state 2
[ "$IUI_SKILL_STATE" = "degraded" ] \
    || note_fail "an unmet soft requirement gave '$IUI_SKILL_STATE', want degraded"
[ "$IUI_SKILL_BLOCKER" = "memlimit" ] || note_fail 'the degraded skill does not name its tool'
iui_skill_state 4
[ "$IUI_SKILL_STATE" = "blocked" ] \
    || note_fail "an unmet hard requirement gave '$IUI_SKILL_STATE', want blocked"
[ "$(iui_state_role blocked)" = "redstone" ] \
    && [ "$(iui_state_role degraded)" = "gold" ] \
    && [ "$(iui_state_role ok)" = "diamond" ] \
    || note_fail 'the three states must map to three different palette roles'

# Selecting a blocked skill is refused with a reason; deselecting always works.
IUI_SKILL_SEL[4]=0
IUI_MESSAGE=()
IUI_CURSOR=4
iui_toggle 4
[ "${IUI_SKILL_SEL[4]}" -eq 0 ] || note_fail 'a blocked skill must not become selected'
[ "${#IUI_MESSAGE[@]}" -gt 0 ] || note_fail 'refusing a blocked skill must say why'
case "${IUI_MESSAGE[0]}" in
    *jq*) : ;;
    *) note_fail 'the refusal must name the missing tool' ;;
esac
IUI_SKILL_SEL[2]=0
iui_toggle 2
[ "${IUI_SKILL_SEL[2]}" -eq 1 ] || note_fail 'a merely degraded skill must stay selectable'
iui_handle_key a
[ "${IUI_SKILL_SEL[4]}" -eq 0 ] || note_fail 'select-all must skip a blocked skill'
[ "${IUI_SKILL_SEL[1]}" -eq 1 ] || note_fail 'select-all must still select an installable skill'
note_pass 'hard requirements block, soft requirements only degrade'

# The three states must be distinguishable in the frame itself, colour or not.
# Rendered in-process so the injected fixture is the state being drawn.
for mode in 'none ascii' 'truecolor blocks'; do
    COLOR_MODE="${mode%% *}"
    iui_set_glyphs "${mode##* }"
    IUI_CURSOR=0
    for pair in '4: block' '2:  warn' '1:    ok'; do
        iui_list_cell "${pair%%:*}" 26
        iui_plain "$IUI_LIST_CELL"
        [ "${#IUI_PLAIN}" -eq 26 ] || note_fail "a list cell is ${#IUI_PLAIN} cells, want 26"
        [ "${IUI_PLAIN: -6}" = "${pair#*:}" ] \
            || note_fail "row ${pair%%:*} in $COLOR_MODE ends '${IUI_PLAIN: -6}', want '${pair#*:}'"
    done
done
COLOR_MODE=none
iui_set_glyphs ascii
IUI_CURSOR=4
frame="$(iui_render_frame)"
# PORTABILITY(pipefail-grep-q): the padding is variable, so these three stay
# regexes and are matched in-process rather than through a pipe.
blocked_re='install *blocked \(jq missing\)'
[[ "$frame" =~ $blocked_re ]] \
    || note_fail 'the info pane must spell out the blocking tool'
tool_row_re='jq  *hard  *missing'
[[ "$frame" =~ $tool_row_re ]] \
    || note_fail 'the info pane must show the tool, its strength and its state'
IUI_CURSOR=2
frame="$(iui_render_frame)"
degraded_re='install *allowed, degraded \(memlimit'
[[ "$frame" =~ $degraded_re ]] \
    || note_fail 'a degraded skill must still read as installable'
case "$frame" in
    *'memory cap not enforced'*) ;;
    *) note_fail 'a soft requirement must say what capability is lost' ;;
esac
note_pass 'all three requirement states are distinguishable with and without colour'

# A condition carries architecture as well as OS, so memlimit is not required on
# an Intel Mac.
IUI_UNAME_S=Darwin
IUI_UNAME_M=arm64
iui_cond_applies 'Darwin:arm64' || note_fail 'Darwin:arm64 must apply on an arm64 Mac'
IUI_UNAME_M=x86_64
iui_cond_applies 'Darwin:arm64' && note_fail 'Darwin:arm64 must not apply on an Intel Mac'
IUI_UNAME_S=Linux
iui_cond_applies 'Darwin' && note_fail 'a Darwin condition must not apply on Linux'
iui_cond_applies '*' || note_fail 'the * condition must always apply'
IUI_UNAME_S=''
IUI_UNAME_M=''
note_pass 'a requirement condition matches on OS and architecture'

# ── 7. ASCII fallback ────────────────────────────────────────────────────────
mono="$(bash "$ui" --render --width 80 --height 40 --color none --glyphs ascii)"
case "$mono" in
    *$'\033'*) note_fail 'the none colour mode emitted an escape sequence' ;;
esac
has_non_ascii "$mono" && note_fail 'the ASCII fallback emitted a non-ASCII byte'
case "$mono" in
    *'################################'*) note_fail 'the sprite must be dropped in the none colour mode, not drawn blank' ;;
esac
COLOR_MODE=none
IUI_COLS=200
IUI_ROWS=60
iui_layout
[ "$IUI_HEAD_ON" -eq 0 ] || note_fail 'the sprite must not be reserved space it cannot paint'
COLOR_MODE=truecolor
iui_layout
[ "$IUI_HEAD_ON" -eq 1 ] || note_fail 'the sprite must appear when there is colour and room'
[ "$IUI_HEAD_SCALE" -ge 2 ] || note_fail "expected scale >= 2 at 200x60, got $IUI_HEAD_SCALE"
IUI_ROWS=24
iui_layout
[ "$IUI_HEAD_ON" -eq 0 ] \
    || note_fail 'the sprite must be dropped when it would starve the info text'
note_pass 'the ASCII/no-colour fallback is clean and drops the sprite'

# ── 8. Only the eye rows animate ─────────────────────────────────────────────
bash "$ui" --render --width 100 --height 40 --color truecolor --eye front > "$scratch/front"
bash "$ui" --render --width 100 --height 40 --color truecolor --eye left > "$scratch/left"
# awk rather than diff: `diff` exits 1 on a difference, which set -e treats as
# a test crash, and its line-format long options are not portable.
changed="$(awk 'NR==FNR { a[FNR]=$0; next } a[FNR] != $0 { printf "%d ", FNR }' \
    "$scratch/front" "$scratch/left")"
[ "$changed" = "11 12 " ] \
    || note_fail "the eye states differ on lines [$changed], want 11 and 12"
note_pass 'an eye-state change touches only the two sprite eye rows'

# ── 9. Terminal restoration on abort ─────────────────────────────────────────
restored="$(IUI_NO_STTY=1 bash -c '
    set -euo pipefail
    . "'"$ui"'"
    iui_install_traps
    iui_term_enter
    printf "HALF-DRAWN FRAME"
    kill -INT $$
' 2>/dev/null || true)"
for want in '?1049h' '?1049l' '?25h' '?1006l' '?1000l'; do
    case "$restored" in
        *"$want"*) : ;;
        *) note_fail "an abort mid-render did not emit $want" ;;
    esac
done
case "$restored" in
    *'HALF-DRAWN FRAME'*'?1049l'*) : ;;
    *) note_fail 'the restore sequences must follow the partial frame, not precede it' ;;
esac
note_pass 'aborting mid-render restores the screen buffer, cursor and mouse mode'

# The same must hold for a plain exit (the EXIT trap) and for SIGTERM.
for signal in EXIT TERM; do
    case "$signal" in
        EXIT) abort='exit 1' ;;
        *) abort='kill -TERM $$' ;;
    esac
    restored="$(IUI_NO_STTY=1 bash -c '
        set -euo pipefail
        . "'"$ui"'"
        iui_install_traps
        iui_term_enter
        printf "HALF-DRAWN FRAME"
        '"$abort"'
    ' 2>/dev/null || true)"
    case "$restored" in
        *'?1049l'*) : ;;
        *) note_fail "the $signal path did not leave the alternate screen" ;;
    esac
done
note_pass 'the EXIT and TERM paths restore the terminal too'

# A caller with no tty on fd 3 must be refused, not half-started.
rc=0
# shellcheck source=/dev/null
( exec 3<&-; . "$ui"; iui_run ) >/dev/null 2>&1 || rc="$?"
[ "$rc" -eq 69 ] || note_fail "iui_run without a tty on fd 3 returned $rc, want 69"
note_pass 'iui_run declines with 69 when fd 3 is not a tty'

# ── 10. The installer seam ───────────────────────────────────────────────────
# iui_select_skills() is the one function install.sh's select_skills() calls, so
# it is driven the way the installer drives it: the generated verify table
# stubbed to fail for jq, fd 3 closed, and cleanup() standing in for the
# installer's own EXIT trap.
seam_probe() {
    ( set -euo pipefail
      # shellcheck source=/dev/null
      . "$ui"
      runtime_tool_verify() { [ "$1" != jq ]; }
      SOURCE_VERSION=""
      TARGET_SELECTION=""
      SELECTED_SKILLS=()
      cleanup() { printf 'cleanup-ran\n'; }
      trap cleanup EXIT
      iui_load_installer_skills
      printf 'planning-sel=%s other-sel=%s\n' "${IUI_SKILL_SEL[0]}" "${IUI_SKILL_SEL[1]}"
      seam_rc=0
      exec 3<&-
      iui_select_skills || seam_rc="$?"
      printf 'rc=%s\n' "$seam_rc"
      # `trap -p` reports nothing from a subshell on bash 3.2, so the trap is
      # proved by its effect: were the chained handler still installed, exiting
      # here would run iui_term_leave as well as cleanup.
      IUI_TERM_ACTIVE=1
      iui_term_leave() { printf 'term-leave-ran\n'; } )
}

seam="$(seam_probe 2>&1 || true)"
case "$seam" in
    *'planning-sel=0 other-sel=1'*) : ;;
    *) note_fail 'the default selection must skip a skill whose hard requirement is missing' ;;
esac
case "$seam" in
    *'rc=69'*) : ;;
    *) note_fail 'iui_select_skills must return 69 so select_skills falls back to the numbered menu' ;;
esac
case "$seam" in
    *'cleanup-ran'*) : ;;
    *) note_fail "install.sh's cleanup() no longer runs on exit after the picker" ;;
esac
case "$seam" in
    *'term-leave-ran'*) note_fail "the picker left its own EXIT trap installed over cleanup" ;;
esac
note_pass 'the installer seam declines without a tty and leaves cleanup() installed'

[ "$(t_failures)" -eq 0 ] || exit 1
printf 'install-ui: all assertions passed\n'
