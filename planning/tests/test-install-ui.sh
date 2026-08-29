#!/usr/bin/env bash
# MODE: DEV
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
# The release row probes the network. A test must not: the answer would vary with
# connectivity, and a 3-second timeout per call would be paid on every assertion
# that renders a frame. Pinned off for the whole file; the release assertions set
# IUI_RELEASE_VERSION themselves.
IUI_NO_NETWORK=1

t_begin

export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ui="$repo_root/install-ui.sh"

note_fail() { printf 'install-ui: %s\n' "$1" >&2; t_record "$1"; }
note_pass() { printf 'PASS %s\n' "$1"; }

[ -f "$ui" ] || { note_fail "missing $ui"; exit 1; }
# shellcheck source=/dev/null
. "$ui"

# Section 4's generated tables are not sourced here, so the requirement
# helpers the renderer gained with any-of groups stand in as identity: these
# tests exercise plain tools, whose label is the tool itself.
runtime_requirement_label() { printf '%s' "$1"; }
runtime_requirement_install_hint() { printf '  install %s\n' "$1"; }

has_non_ascii() {
    [ -n "$(printf '%s' "$1" | tr -d ' -~')" ]
}

# ── 1. Width discipline ──────────────────────────────────────────────────────
check_widths() {
    local label="$1" width="$2" height="$3"
    shift 3
    local out line n=0 bad=0
    out="$("$BASH" "$ui" --render --width "$width" --height "$height" "$@")"
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
frame="$("$BASH" "$ui" --render --width 80 --height 8 --cursor 5 --color none --glyphs ascii)"
case "$frame" in
    *'>[ ] install-ui'*) ;;
    *) note_fail 'the cursor row is not visible after scrolling' ;;
esac
case "$frame" in
    *'] planning'*) note_fail 'the list did not scroll: row 0 is still drawn' ;;
esac
[ "$(t_failures)" -eq 0 ] && note_pass 'the list scrolls and keeps the cursor visible'

# ── 3. Checkbox and the three ways to toggle ─────────────────────────────────
frame="$("$BASH" "$ui" --render --width 80 --height 24 --color none --glyphs ascii)"
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
list_frame="$("$BASH" "$ui" --render --width 80 --height 24 --color none --glyphs ascii --focus list)"
info_frame="$("$BASH" "$ui" --render --width 80 --height 24 --color none --glyphs ascii --focus info)"
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
# Skills 0 and 4 both need rjq, so reverifying from one must change the other.
# The probe is overridden rather than uninstalling rjq.
iui_load_demo
iui_req_reset
IUI_DEP_TOOLS=(); IUI_DEP_STATES=()
iui_req_add 0 rjq '*' hard 'plan validation refuses to run'
iui_req_add 2 memlimit '*' soft 'memory cap not enforced; CPU limiting still works'
iui_req_add 4 rjq '*' hard 'the finding tables cannot be assembled'
iui_dep_probe() { return 1; }
iui_dep_reverify_skill 0
iui_dep_state rjq
[ "$IUI_DEP_STATE" = "missing" ] || note_fail "reverify left rjq as '$IUI_DEP_STATE'"
iui_skill_state 4
[ "$IUI_SKILL_STATE" = "blocked" ] \
    || note_fail 'reverifying rjq from skill 0 did not reach skill 4, so the cache is per-skill'
[ "${#IUI_DEP_TOOLS[@]}" -eq 1 ] \
    || note_fail "the cache holds ${#IUI_DEP_TOOLS[@]} records; rjq must not be stored twice"
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
    case "$line" in *'rjq'*'missing'*) found=1 ;; esac
done
[ "$found" -eq 1 ] || note_fail "skill 4's info pane still shows rjq as met"
note_pass 'reverifying one tool refreshes every skill that shares it'

# ── 6b. hard blocks, soft only degrades ──────────────────────────────────────
# Fixture from 6: rjq (hard) and memlimit (soft) both missing, so skill 4 is
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
    *rjq*) : ;;
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
blocked_re='install *blocked \(rjq missing\)'
[[ "$frame" =~ $blocked_re ]] \
    || note_fail 'the info pane must spell out the blocking tool'
tool_row_re='rjq  *hard  *missing'
[[ "$frame" =~ $tool_row_re ]] \
    || note_fail 'the info pane must show the tool, its strength and its state'
IUI_CURSOR=2
frame="$(iui_render_frame)"
# The verdict may wrap: at 80 columns the list takes its content width (37) and
# the detail pane gets 40, so this phrase no longer fits on one line. An earlier
# version of this assertion required it unwrapped, which pinned a pane width
# rather than the contract. What must hold is that a degraded skill still reads
# as installable and still names the tool, wrapped or not.
degraded_re='install *allowed, degraded'
[[ "$frame" =~ $degraded_re ]] \
    || note_fail 'a degraded skill must still read as installable'
case "$frame" in
    *memlimit*|*memlim-*) ;;
    *) note_fail 'a degraded verdict must name the tool that degraded it' ;;
esac
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
mono="$("$BASH" "$ui" --render --width 80 --height 40 --color none --glyphs ascii)"
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
# The sprite lives at the bottom of the LEFT pane now, in the space the list does
# not use, so it costs the detail pane nothing. Before this it sat in the detail
# pane and grew to fill it: measured at 200x60 it took 48 of 60 rows and left the
# skill's own information 8.
[ "$IUI_HEAD_H" -gt 0 ] \
    || note_fail 'the sprite must have a height when it is on'
[ $((IUI_HEAD_SCALE * 32)) -le "$IUI_LEFT_W" ] \
    || note_fail "the sprite must fit the left pane: $((IUI_HEAD_SCALE * 32)) of $IUI_LEFT_W"
# The two placements have different invariants, so each is checked in its own
# right rather than by one arithmetic that happens to hold for both.
case "$IUI_HEAD_PLACE" in
    left-bottom)
        [ $((IUI_LIST_ROWS + 1 + IUI_HEAD_H)) -eq "$IUI_BODY_ROWS" ] \
            || note_fail "left-bottom sections must fill the body: list $IUI_LIST_ROWS + sep 1 + sprite $IUI_HEAD_H against $IUI_BODY_ROWS"
        [ "$IUI_LIST_ROWS" -ge "$IUI_HEAD_MIN_LIST_ROWS" ] \
            || note_fail "the list keeps its floor: $IUI_LIST_ROWS against $IUI_HEAD_MIN_LIST_ROWS"
        ;;
    right-top)
        [ "$IUI_LIST_ROWS" -eq "$IUI_BODY_ROWS" ] \
            || note_fail 'with the sprite in the detail pane the list keeps every row'
        [ $((IUI_BODY_ROWS - IUI_HEAD_H)) -ge "$IUI_HEAD_DETAIL_FLOOR" ] \
            || note_fail "the detail text keeps its floor: $((IUI_BODY_ROWS - IUI_HEAD_H)) against $IUI_HEAD_DETAIL_FLOOR"
        [ $((IUI_RIGHT_W - IUI_HEAD_SCALE * 32)) -ge "$IUI_HINT_MIN_COLS" ] \
            || note_fail 'the hints must have room beside the sprite'
        ;;
    *) note_fail "expected a placement at this size, got '$IUI_HEAD_PLACE'" ;;
esac

# An ordinary 40-row terminal puts the sprite bottom-left, alone: no hints beside
# it, and the detail pane keeps every row it had.
IUI_ROWS=40
iui_layout
[ "$IUI_HEAD_PLACE" = left-bottom ] \
    || note_fail "a 40-row terminal must place the sprite bottom-left; got '$IUI_HEAD_PLACE'"
# A tall terminal moves it beside the hints, in space that was blank before.
IUI_ROWS=60
iui_layout
[ "$IUI_HEAD_PLACE" = right-top ] \
    || note_fail "a 60-row terminal must place the sprite beside the hints; got '$IUI_HEAD_PLACE'"
# Pixel text costs six columns per character, so a hint that fits as plain text
# can still overflow here. Checked against the space beside the sprite.
hint_index=0
while [ "$hint_index" -lt "${#IUI_HINTS[@]}" ]; do
    [ $(( (${#IUI_HINTS[$hint_index]} + 1) * 6 )) -le $((IUI_RIGHT_W - IUI_HEAD_SCALE * 32)) ] \
        || note_fail "hint '${IUI_HINTS[$hint_index]}' needs $(( (${#IUI_HINTS[$hint_index]} + 1) * 6 )) columns of pixel text, pane has $((IUI_RIGHT_W - IUI_HEAD_SCALE * 32))"
    hint_index=$((hint_index + 1))
done
IUI_ROWS=20
iui_layout
[ "$IUI_HEAD_ON" -eq 0 ] \
    || note_fail 'the sprite must be dropped when the list would lose its floor'
note_pass 'the ASCII/no-colour fallback is clean and drops the sprite'

# ── 6c. The release row, its states and their colours ────────────────────────
# Red is for something wrong. This row read redstone for every answer that was
# not literally "yes", so a skill that was simply not installed looked like a
# fault. And the comparison itself was against the local checkout's commit,
# unknowable under `curl … | bash`; it is now against the published release.
IUI_NO_NETWORK=1
IUI_RELEASE_CHECKED=0
IUI_RELEASE_VERSION=''
IUI_SKILL_INSTALLED[0]=yes
IUI_SKILL_HAVE_PKG[0]=1.4.2
release_text="$(iui_uptodate_text 0)"
case "$release_text" in
    'unknown ('*) ;;
    *) note_fail "offline must say it could not reach the release, got '$release_text'" ;;
esac
[ "$IUI_RELEASE_VERSION" = '' ] \
    || note_fail 'the offline path must not invent a version'

# One definition: IUI_STUB_PROBE=1 turns it into the no-op the second
# group needs (a redefined stub trips SC2218 on shellcheck 0.9.0, where uses
# before the LAST definition still count as use-before-def).
iui_release_version() {
    [ "${IUI_STUB_PROBE:-0}" = 1 ] && return 0
    IUI_RELEASE_VERSION=9.9.9
    IUI_RELEASE_CHECKED=1
}
# Probed once per run, not once per redraw: the picker renders on every keypress.
IUI_RELEASE_CHECKED=0
iui_release_version
first_state="$IUI_RELEASE_CHECKED"
iui_release_version
[ "$first_state" = 1 ] && [ "$IUI_RELEASE_CHECKED" = 1 ] \
    || note_fail 'the release probe must latch after the first call'

# The row must trigger the probe itself. Stubbed rather than fetched: if
# iui_uptodate_text stops calling it, the version stays empty and every row reads
# "unknown" forever -- silently, since offline looks identical.
IUI_RELEASE_VERSION=''
IUI_RELEASE_CHECKED=0
IUI_SKILL_INSTALLED[0]=yes
IUI_SKILL_HAVE_PKG[0]=1.0.0
case "$(iui_uptodate_text 0)" in
    *'-> 9.9.9'*) ;;
    *) note_fail "the release row must ask for the version itself, got '$(iui_uptodate_text 0)'" ;;
esac
# Replaced with a no-op rather than re-sourced: sourcing 35-ui-model.sh again
# reinitialises every IUI_ global, so the skill arrays this file set up are wiped
# and iui_uptodate_text reads unbound values. Every assertion below sets
# IUI_RELEASE_VERSION itself, so a probe that does nothing is exactly right.
IUI_STUB_PROBE=1

# The three answers and the role each gets. Each sets every input it depends on:
# inheriting state from the block above is how this test passed standalone and
# failed in the suite, reading a version an earlier assertion had left behind.
IUI_RELEASE_VERSION=1.4.2
IUI_RELEASE_CHECKED=1
IUI_SKILL_INSTALLED[0]=yes
IUI_SKILL_HAVE_PKG[0]=1.4.2
[ "$(iui_uptodate_text 0)" = 'yes' ] \
    || note_fail "matching versions must read yes, got '$(iui_uptodate_text 0)'"
IUI_SKILL_HAVE_PKG[0]=1.4.1
case "$(iui_uptodate_text 0)" in
    'no (1.4.1 -> 1.4.2)') ;;
    *) note_fail "a newer release must name both versions, got '$(iui_uptodate_text 0)'" ;;
esac
IUI_SKILL_INSTALLED[0]=no
[ "$(iui_uptodate_text 0)" = 'not installed' ] \
    || note_fail "an uninstalled skill must say so, got '$(iui_uptodate_text 0)'"

# Body text must not fall through to the palette default, which measured 4.2:1
# against a dark terminal and reads as dark grey.
iui_seg stone x
case "$IUI_SEG" in
    *'205;207;212'*) ;;
    *) note_fail 'stone must be the named high-contrast body colour' ;;
esac

# The row's colour, not just its words: redstone here reads as a fault.
release_role_of() {
    local i
    iui_info_lines 60
    for ((i = 0; i < ${#IUI_INFO_TEXT[@]}; i++)); do
        case "${IUI_INFO_TEXT[$i]}" in
            *'latest release'*) printf '%s' "${IUI_INFO_ROLE[$i]}"; return ;;
        esac
    done
    printf 'not-found'
}
IUI_CURSOR=0
IUI_SKILL_INSTALLED[0]=no
[ "$(release_role_of)" != redstone ] \
    || note_fail 'an uninstalled skill must not paint the release row red'
IUI_SKILL_INSTALLED[0]=yes
IUI_SKILL_HAVE_PKG[0]=1.4.1
IUI_RELEASE_VERSION=1.4.2
IUI_RELEASE_CHECKED=1
[ "$(release_role_of)" != redstone ] \
    || note_fail 'an available update must not paint the release row red'
note_pass 'the release row compares against the published release and reserves red'

# ── 6b1. Nothing on the exit path may block ──────────────────────────────────
# Two defects met here and left the terminal unusable: a lone Escape blocked
# forever waiting for a continuation byte it never gets, and the input drain
# blocked for 256 bytes that never came -- after the restore sequences had been
# emitted, so the tty stayed raw with no prompt. Both are bounded now, and both
# are checked against an fd that never sends a byte and never closes, because an
# EOF would rescue an unbounded read and hide the defect.
escape_probe="$scratch/escape-probe.sh"
cat > "$escape_probe" <<'PROBE'
set -uo pipefail
cd "$1"
for f in installer/src/05-config.sh installer/src/30-render.sh installer/src/35-ui-model.sh \
         installer/src/36-ui-render.sh installer/src/37-ui-input.sh; do
    # shellcheck disable=SC1090
    source "$f" 2>/dev/null || true
done
case "$2" in
    escape)
        exec 3< <(printf '\033'; sleep 8)
        iui_read_key
        printf '%s' "$IUI_KEY"
        ;;
    drain)
        exec 3< <(sleep 8)
        IUI_STTY_SAVED='pretend-saved'
        # stty reports success and changes nothing -- exactly the case that hung
        # a real terminal. Without this the guard returns early on a pipe and the
        # read is never reached, so the assertion would prove nothing.
        stty() { return 0; }
        iui_drain_input
        printf 'returned'
        ;;
esac
PROBE
escape_result="$("$BASH" "$escape_probe" "$repo_root" escape 2>/dev/null &
                 probe_pid=$!
                 ( sleep 5; kill -9 "$probe_pid" 2>/dev/null ) >/dev/null 2>&1 &
                 wait "$probe_pid" 2>/dev/null)"
[ "$escape_result" = ESC ] \
    || note_fail "a lone Escape must resolve to ESC without blocking; got '$escape_result'"
drain_result="$("$BASH" "$escape_probe" "$repo_root" drain 2>/dev/null &
                probe_pid=$!
                ( sleep 5; kill -9 "$probe_pid" 2>/dev/null ) >/dev/null 2>&1 &
                wait "$probe_pid" 2>/dev/null)"
[ "$drain_result" = returned ] \
    || note_fail "the input drain must return without blocking; got '$drain_result'"
note_pass 'a lone Escape and the input drain are both bounded'

# ── 6b2. Every eye state in the cycle is reachable ────────────────────────────
# iui_advance_eye searched the cycle for the current state and took the next one.
# `front` appears more than once, so the search always matched the first and the
# sequence collapsed to front/right/front/right: `left` was never reached, on any
# terminal, ever. Index-based advance fixes it; this asserts the property rather
# than the implementation.
eye_seen_left=no
eye_seen_right=no
eye_seen_front=no
IUI_EYE_INDEX=0
for eye_tick in $(seq 1 "${#IUI_EYE_FRAMES[@]}"); do
    iui_advance_eye
    case "$IUI_EYE" in
        left) eye_seen_left=yes ;;
        right) eye_seen_right=yes ;;
        front) eye_seen_front=yes ;;
        *) note_fail "unknown eye state in the cycle: $IUI_EYE" ;;
    esac
done
[ "$eye_seen_left" = yes ] || note_fail 'the eye cycle never looks left'
[ "$eye_seen_right" = yes ] || note_fail 'the eye cycle never looks right'
[ "$eye_seen_front" = yes ] || note_fail 'the eye cycle never returns to centre'

# The dwell is what makes the movement read as deliberate: without several still
# frames first, the sprite twitches one second after the frame is drawn.
eye_lead=0
for eye_tick in 0 1 2 3 4 5 6 7; do
    [ "${IUI_EYE_FRAMES[$eye_tick]}" = front ] || break
    eye_lead=$((eye_lead + 1))
done
[ "$eye_lead" -ge 3 ] \
    || note_fail "the cycle must open with a still dwell; got $eye_lead frame(s)"
IUI_EYE_INDEX=0
IUI_EYE=front
note_pass 'the eye cycle reaches every state and opens with a dwell'

# ── 6d. The detail pane scrolls, and the frame lines up ───────────────────────
# Tabbing to the detail pane highlighted it but Up/Down still moved the skill
# cursor, so every line of the detail block below the fold was unreachable.
IUI_COLS=110
IUI_ROWS=24
# The pane must have more content than rows, and this test supplies it rather
# than relying on the demo fixture carrying a long enough detail block.
IUI_SKILL_DETAILS[0]='One paragraph of detail that occupies a line.
A second paragraph, so the block is taller than a couple of rows.
A third, because the pane is twenty rows and the status section takes some.
A fourth to be sure there is something below the fold to scroll to.
A fifth, since the whole point is that it was unreachable before.
A sixth paragraph, past any plausible pane height for this fixture.
A seventh, so the assertion below cannot pass by accident.
An eighth paragraph closing the block.'
iui_layout
iui_info_lines "$IUI_RIGHT_W"
iui_clamp_info_scroll
[ "$IUI_INFO_MAX_SCROLL" -gt 0 ] \
    || note_fail "the fixture must have more detail than rows to test scrolling"
IUI_FOCUS=list
IUI_INFO_SCROLL=0
cursor_before="$IUI_CURSOR"
iui_handle_key DOWN
[ "$IUI_CURSOR" -ne "$cursor_before" ] \
    || note_fail 'with the list focused, Down must move the skill cursor'
[ "$IUI_INFO_SCROLL" -eq 0 ] \
    || note_fail 'moving the skill cursor must reset the detail scroll'
IUI_FOCUS=info
cursor_before="$IUI_CURSOR"
iui_handle_key DOWN
iui_handle_key DOWN
[ "$IUI_INFO_SCROLL" -eq 2 ] \
    || note_fail "with the detail focused, Down must scroll it; got $IUI_INFO_SCROLL"
[ "$IUI_CURSOR" -eq "$cursor_before" ] \
    || note_fail 'scrolling the detail pane must not move the skill cursor'
iui_handle_key END
[ "$IUI_INFO_SCROLL" -eq "$IUI_INFO_MAX_SCROLL" ] \
    || note_fail 'End must reach the bottom of the detail pane'
iui_clamp_info_scroll
[ "$IUI_INFO_SCROLL" -le "$IUI_INFO_MAX_SCROLL" ] \
    || note_fail 'the detail scroll must never pass its own end'
IUI_FOCUS=list
IUI_INFO_SCROLL=0
IUI_CURSOR=0

# The focused pane must be marked by something other than brackets, which are
# invisible at a glance and vanish in the no-colour mode.
iui_header_seg SKILLS 20 1
focused_seg="$IUI_HEADER_SEG"
iui_header_seg SKILLS 20 0
[ "$focused_seg" != "$IUI_HEADER_SEG" ] \
    || note_fail 'a focused pane header must render differently from an unfocused one'

note_pass 'the detail pane scrolls under focus and the focused pane is marked'

# ── 7a. Wrapping breaks words, and marks the break when it cannot ────────────
# A token wider than the pane used to be cut with no mark, so a reader could not
# tell a break from the end of a word.
iui_wrap "the terminator wraps here" 12
[ "${IUI_WRAP_LINES[0]}" = "the" ] \
    || note_fail "expected a word break, got '${IUI_WRAP_LINES[0]}'"
case "${IUI_WRAP_LINES[*]}" in
    *-*) note_fail "text that fits on word boundaries must not be hyphenated: ${IUI_WRAP_LINES[*]}" ;;
esac
iui_wrap "supercalifragilisticexpialidocious" 12
[ "${IUI_WRAP_LINES[0]}" = "supercalifr-" ] \
    || note_fail "an unbreakable token must continue with a hyphen, got '${IUI_WRAP_LINES[0]}'"
for wrapped_line in "${IUI_WRAP_LINES[@]}"; do
    [ "${#wrapped_line}" -le 12 ] \
        || note_fail "a wrapped line ran past the pane: '$wrapped_line' is ${#wrapped_line} of 12"
done
# The hyphen replaces a character rather than being inserted beside it, so no
# character of the source may be lost or repeated across the joined lines.
rejoined=""
for wrapped_line in "${IUI_WRAP_LINES[@]}"; do
    rejoined="$rejoined${wrapped_line%-}"
done
[ "$rejoined" = "supercalifragilisticexpialidocious" ] \
    || note_fail "hyphenation lost or duplicated characters: '$rejoined'"
note_pass 'wrapping prefers word boundaries and marks a mid-word break'

# ── 7b. The list fits its longest name ───────────────────────────────────────
# A row is cursor(1) + checkbox(3) + space(1) + name + state tag(6). The width
# was a third of the terminal capped at 34, which truncated
# post-implementation-review (26 chars, needing 37) at every terminal size.
iui_longest_name_width() {
    local i longest=0 length
    for ((i = 0; i < ${#IUI_SKILL_NAMES[@]}; i++)); do
        length="${#IUI_SKILL_NAMES[$i]}"
        [ "$length" -le "$longest" ] || longest="$length"
    done
    printf '%s' $((longest + 11))
}
for size in "80 24" "120 40" "200 60" "300 100"; do
    set -- $size
    IUI_COLS="$1"
    IUI_ROWS="$2"
    iui_layout
    needed="$(iui_longest_name_width)"
    if [ "$IUI_NARROW" -eq 0 ] && [ "$((IUI_COLS - 3 - IUI_DETAIL_MIN_W))" -ge "$needed" ]; then
        [ "$IUI_LEFT_W" -ge "$needed" ] \
            || note_fail "at ${1}x${2} the list is $IUI_LEFT_W columns, needs $needed for its longest name"
        [ "$IUI_RIGHT_W" -ge "$IUI_DETAIL_MIN_W" ] \
            || note_fail "at ${1}x${2} the detail pane is $IUI_RIGHT_W columns, floor is $IUI_DETAIL_MIN_W"
    fi
done
note_pass 'the list is sized to its longest skill name, so nothing truncates'

# ── 8. Only the eye rows animate ─────────────────────────────────────────────
"$BASH" "$ui" --render --width 100 --height 40 --color truecolor --eye front > "$scratch/front"
"$BASH" "$ui" --render --width 100 --height 40 --color truecolor --eye left > "$scratch/left"
# awk rather than diff: `diff` exits 1 on a difference, which set -e treats as
# a test crash, and its line-format long options are not portable.
changed="$(awk 'NR==FNR { a[FNR]=$0; next } a[FNR] != $0 { printf "%d ", FNR }' \
    "$scratch/front" "$scratch/left")"
# Derived, not hardcoded: the sprite moved from the top of the detail pane to the
# bottom of the list pane, and a literal row number silently encodes wherever it
# happened to sit. What matters is that exactly the two eye rows change.
IUI_COLS=100
IUI_ROWS=40
iui_layout
eye_top=$((3 + IUI_LIST_ROWS + 1 + IUI_HINT_ROWS))
want_eyes="$((eye_top + 8 * IUI_HEAD_SCALE)) $((eye_top + 9 * IUI_HEAD_SCALE)) "
[ "$changed" = "$want_eyes" ] \
    || note_fail "the eye states differ on lines [$changed], want [$want_eyes]"
note_pass 'an eye-state change touches only the two sprite eye rows'

# ── 9. Terminal restoration on abort ─────────────────────────────────────────
restored="$(IUI_NO_STTY=1 "$BASH" -c '
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
    restored="$(IUI_NO_STTY=1 "$BASH" -c '
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
# stubbed to fail for rjq, fd 3 closed, and cleanup() standing in for the
# installer's own EXIT trap.
seam_probe() {
    ( set -euo pipefail
      # shellcheck source=/dev/null
      . "$ui"
      runtime_tool_verify() { [ "$1" != rjq ]; }
      runtime_requirement_label() { printf '%s' "$1"; }
      runtime_requirement_install_hint() { printf '  install %s\n' "$1"; }
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
    *'planning-sel=1 other-sel=1'*) : ;;
    *) note_fail 'the default selection must include planning with bundled rjq' ;;
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
