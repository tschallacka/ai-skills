# MODE: DEV
# PACKAGE: PROD
# ---------------------------------------------------------------
# 6b. Full-screen skill picker: state, text metrics, requirement model
# ---------------------------------------------------------------
# The interactive picker that replaces the numbered menu of section 6 when fd 3
# is a terminal. Its three parts are split by concern and read in order: this one
# owns the state and the arithmetic, 6c draws a frame, 6d reads the keyboard and
# is the seam select_skills() calls.
#
# Everything here is prefixed iui_ and shares section 6's primitives rather than
# repeating them: ART, detect_color_mode, fg_sgr, color_for and eye_row_for are
# defined there, so the sprite and the palette exist once in this file.
#
# Banners in this part, in order:
#   State
#   Palette segments
#   Glyph sets and the plain-text fold
#   The requirement table — the data seam
#   The global, per-tool dependency cache
#   Layout arithmetic

# ─────────────────────────────────────────────────────────────────────────────
# State
# ─────────────────────────────────────────────────────────────────────────────

# PORTABILITY(assoc-array): the skill table and the dependency cache both want a
# map; index-parallel arrays plus a linear lookup are the bash 3.2 shape of one.
IUI_SKILL_NAMES=()
IUI_SKILL_DETAILS=()
IUI_INFO_SCROLL=0
IUI_INFO_ROWS=0
IUI_INFO_MAX_SCROLL=0
IUI_SKILL_DESCS=()
IUI_SKILL_INSTALLED=()
IUI_SKILL_HAVE=()
IUI_SKILL_WANT=()
IUI_SKILL_SEL=()

# The global dependency cache. Keyed by TOOL, never by skill, so reverifying
# `rjq` from one skill's info box refreshes every skill that also needs `rjq`.
IUI_DEP_TOOLS=()
IUI_DEP_STATES=()

IUI_CURSOR=0
IUI_SCROLL=0
IUI_FOCUS=list
IUI_COLS=80
IUI_ROWS=24
IUI_GLYPHS=blocks
IUI_POSITION=0
IUI_DONE=0
IUI_RC=130
IUI_EYE=front
IUI_HEAD_ON=0
IUI_HEAD_SCALE=0
IUI_HEAD_H=0
IUI_MESSAGE=()
IUI_STTY_SAVED=""
IUI_TERM_ACTIVE=0

# ─────────────────────────────────────────────────────────────────────────────
# Palette segments
# ─────────────────────────────────────────────────────────────────────────────

# Minecraft block palette. iui_seg() wraps text in a role's colour; in the
# `none` colour mode every wrap is the identity, which is what keeps the ASCII
# fallback free of escape bytes.
iui_seg() {
    local role="$1" text="$2" rgb='' attr=''
    [ -n "$COLOR_MODE" ] || detect_color_mode
    case "$role" in
        grass) rgb='91;135;49' ;;
        dirt) rgb='139;90;43' ;;
        gold) rgb='252;238;75'; attr='1;' ;;
        redstone) rgb='217;58;43' ;;
        diamond) rgb='74;237;217' ;;
        obsidian) rgb='21;12;28' ;;
        # Body text. Named explicitly rather than left to the fallback below: at
        # 127;127;127 it measured 4.2:1 against a #1e1e1e terminal, which reads as
        # dark grey and is under the 4.5:1 needed for body text. 205;207;212 is
        # 11:1 on the same ground, with a faint cool bias so it belongs to the
        # palette instead of being pure grey.
        stone) rgb='205;207;212' ;;
        # The focused pane's label. Reverse video rather than another hue: the
        # only previous cue was [BRACKETS] versus spaces, which is invisible at
        # a glance and vanishes entirely in the no-colour mode.
        focus) rgb='252;238;75'; attr='1;7;' ;;
        # Secondary text that should recede without becoming unreadable.
        slate) rgb='150;154;163' ;;
        # Deliberately left as mid grey: a role nobody named should look wrong
        # rather than quietly inheriting body-text contrast.
        *) rgb='127;127;127' ;;
    esac
    if [ "$COLOR_MODE" = "none" ]; then
        IUI_SEG="$text"
        return
    fi
    fg_sgr "$rgb" "$attr"
    IUI_SEG="$FG_SGR$text"$'\033'"[0m"
}

# ─────────────────────────────────────────────────────────────────────────────
# Glyph sets and the plain-text fold
# ─────────────────────────────────────────────────────────────────────────────

# The glyph vocabulary is closed: these nine are the only non-ASCII bytes a frame
# may hold, and iui_plain() folds exactly them back to one byte each.
# ---- quoted: glyph -> ASCII fold ----
# ▀ =    ▄ =    ▌ |
# ▛ +    ▜ +    ▙ +    ▟ +
# ─ -    █ #
# ---- end quoted ----
iui_set_glyphs() {
    IUI_GLYPHS="${1:-blocks}"
    if [ "$IUI_GLYPHS" = "ascii" ]; then
        IUI_G_TOP='='; IUI_G_BOT='='; IUI_G_V='|'
        IUI_G_TL='+'; IUI_G_TR='+'; IUI_G_BL='+'; IUI_G_BR='+'
        IUI_G_TJ='+'; IUI_G_BJ='+'
        IUI_G_RULE='-'; IUI_G_FILL='#'
        return
    fi
    IUI_G_TOP='▀'; IUI_G_BOT='▄'; IUI_G_V='▌'
    IUI_G_TL='▛'; IUI_G_TR='▜'; IUI_G_BL='▙'; IUI_G_BR='▟'
    # Where the pane divider meets the top and bottom borders. Without these the
    # border drew a plain horizontal at the divider column, so the vertical line
    # below it looked disconnected from the frame.
    IUI_G_TJ='▛'; IUI_G_BJ='▙'
    IUI_G_RULE='─'; IUI_G_FILL='█'
}

# Strips SGR sequences and folds the glyph set, leaving one ASCII byte per
# display cell. Pure parameter expansion — no forks, so it is cheap enough to
# call on every rendered line as a self-check.
iui_plain() {
    local s="$1" pre rest
    while [ "${s#*$'\033'\[}" != "$s" ]; do
        pre="${s%%$'\033'\[*}"
        rest="${s#*$'\033'\[}"
        rest="${rest#*m}"
        s="$pre$rest"
    done
    s="${s//▀/=}"; s="${s//▄/=}"; s="${s//▌/|}"
    s="${s//▛/+}"; s="${s//▜/+}"; s="${s//▙/+}"; s="${s//▟/+}"
    s="${s//─/-}"; s="${s//█/#}"
    IUI_PLAIN="$s"
}

iui_repeat() {
    local glyph="$1" count="$2" out='' i
    for ((i = 0; i < count; i++)); do
        out="$out$glyph"
    done
    IUI_REPEAT="$out"
}

# PORTABILITY(bytes-vs-characters): the pad/truncate arithmetic is in display
# cells and the text it measures is ASCII, so ${#t} is both the byte and the
# cell count. Never hand this a glyph — glyphs are placed by count, not measured.
# A 5x7 pixel font, as data rather than a case statement: bash 3.2 has no
# associative arrays, and forty case arms would breach the function-length cap.
# One entry per character, `CHAR:row|row|...` with seven rows of five bits.
#
# The splash already drew mmh-mhh in this shape at two columns per pixel
# (pixel_message). The hints reuse the shape at one column per pixel, which is
# the "tad bit smaller" that keeps a whole hint on one line.
IUI_PIXEL_FONT=(
    '_:00000|00000|00000|00000|00000|00000|00000'
    '-:00000|00000|00000|11111|00000|00000|00000'
    '.:00000|00000|00000|00000|00000|00100|00100'
    '/:00001|00010|00010|00100|01000|01000|10000'
    '0:01110|10001|10011|10101|11001|10001|01110'
    '1:00100|01100|00100|00100|00100|00100|01110'
    '2:01110|10001|00001|00010|00100|01000|11111'
    '3:11110|00001|00001|01110|00001|00001|11110'
    '4:00010|00110|01010|10010|11111|00010|00010'
    '5:11111|10000|11110|00001|00001|10001|01110'
    '6:00110|01000|10000|11110|10001|10001|01110'
    '7:11111|00001|00010|00100|01000|01000|01000'
    '8:01110|10001|10001|01110|10001|10001|01110'
    '9:01110|10001|10001|01111|00001|00010|01100'
    '::00000|00100|00000|00000|00000|00100|00000'
    'A:01110|10001|10001|11111|10001|10001|10001'
    'B:11110|10001|10001|11110|10001|10001|11110'
    'C:01110|10001|10000|10000|10000|10001|01110'
    'D:11110|10001|10001|10001|10001|10001|11110'
    'E:11111|10000|10000|11110|10000|10000|11111'
    'F:11111|10000|10000|11110|10000|10000|10000'
    'G:01110|10001|10000|10111|10001|10001|01110'
    'H:10001|10001|10001|11111|10001|10001|10001'
    'I:11111|00100|00100|00100|00100|00100|11111'
    'J:00111|00010|00010|00010|00010|10010|01100'
    'K:10001|10010|10100|11000|10100|10010|10001'
    'L:10000|10000|10000|10000|10000|10000|11111'
    'M:10001|11011|10101|10101|10001|10001|10001'
    'N:10001|11001|10101|10011|10001|10001|10001'
    'O:01110|10001|10001|10001|10001|10001|01110'
    'P:11110|10001|10001|11110|10000|10000|10000'
    'Q:01110|10001|10001|10001|10101|10011|01111'
    'R:11110|10001|10001|11110|10100|10010|10001'
    'S:01111|10000|10000|01110|00001|00001|11110'
    'T:11111|00100|00100|00100|00100|00100|00100'
    'U:10001|10001|10001|10001|10001|10001|01110'
    'V:10001|10001|10001|10001|10001|01010|00100'
    'W:10001|10001|10001|10101|10101|11011|10001'
    'X:10001|01010|00100|00100|00100|01010|10001'
    'Y:10001|10001|01010|00100|00100|00100|00100'
    'Z:11111|00001|00010|00100|01000|10000|11111'
)

# One pixel row of one character. Unknown characters render blank rather than
# failing: a hint is decoration, and a missing glyph must not take the frame down.
iui_pixel_rows() {
    local want="$1" entry
    [ "$want" != ' ' ] || want='_'
    IUI_PIXEL_ROWS=''
    for entry in "${IUI_PIXEL_FONT[@]}"; do
        if [ "${entry%%:*}" = "$want" ]; then
            IUI_PIXEL_ROWS="${entry#*:}"
            return 0
        fi
    done
    IUI_PIXEL_ROWS='00000|00000|00000|00000|00000|00000|00000'
}

# Row `row` (0-6) of `text` rendered in the pixel font, padded to `cols`.
iui_big_line() {
    local text="$1" row="$2" cols="$3" i char out='' bits
    local -a rows
    for ((i = 0; i < ${#text}; i++)); do
        char="${text:$i:1}"
        iui_pixel_rows "$char"
        IFS='|' read -r -a rows <<< "$IUI_PIXEL_ROWS"
        bits="${rows[$row]}"
        bits="${bits//1/$IUI_G_FILL}"
        bits="${bits// /_}"
        bits="${bits//0/ }"
        out="$out$bits "
    done
    iui_pad "$out" "$cols"
    IUI_BIG_LINE="$IUI_PAD"
}

iui_pad() {
    local t="$1" w="$2"
    [ "$w" -ge 0 ] || w=0
    if [ "${#t}" -gt "$w" ]; then
        if [ "$w" -ge 1 ]; then t="${t:0:$((w - 1))}~"; else t=''; fi
    fi
    printf -v IUI_PAD '%s%*s' "$t" "$((w - ${#t}))" ''
}

# Wraps to whole words where it can and hyphenates where it cannot, so a token
# longer than the pane continues on the next line as `term-` / `inator` rather
# than being cut without a mark. `consume` is tracked separately from the emitted
# line because the hyphen is added by this function and is not part of the source
# text: consuming ${#line} would swallow the character the hyphen replaced.
iui_wrap() {
    local text="$1" width="$2" line split consume
    IUI_WRAP_LINES=()
    [ "$width" -ge 8 ] || width=8
    while [ -n "$text" ]; do
        line="$text"
        consume="${#text}"
        if [ "${#line}" -gt "$width" ]; then
            line="${text:0:width}"
            split="${line% *}"
            if [ -n "$split" ] && [ "$split" != "$line" ]; then
                line="$split"
                consume="${#line}"
            else
                # One unbroken token wider than the pane: break it and say so.
                consume=$((width - 1))
                line="${text:0:consume}-"
            fi
        fi
        IUI_WRAP_LINES+=("$line")
        text="${text:consume}"
        text="${text# }"
    done
    [ "${#IUI_WRAP_LINES[@]}" -gt 0 ] || IUI_WRAP_LINES=('')
}

# ─────────────────────────────────────────────────────────────────────────────
# The requirement table — the data seam
# ─────────────────────────────────────────────────────────────────────────────

# One row per (skill, tool) requirement. iui_req_add() is the only ingest point
# and iui_load_requirements() the only producer, so wiring a generated table in
# means replacing one function; nothing in the renderer reads a skill list.
IUI_REQ_SKILL=()
IUI_REQ_TOOL=()
IUI_REQ_COND=()
IUI_REQ_STRENGTH=()
IUI_REQ_WHY=()

# iui_req_add <skill-index> <tool> <condition> hard|soft <what-is-lost>
# The condition is `*`, an OS glob, or OS:ARCH — memlimit is Darwin:arm64 only
# and must not show up as required on an Intel Mac.
iui_req_add() {
    IUI_REQ_SKILL+=("$1")
    IUI_REQ_TOOL+=("$2")
    IUI_REQ_COND+=("$3")
    IUI_REQ_STRENGTH+=("$4")
    IUI_REQ_WHY+=("$5")
}

iui_req_reset() {
    IUI_REQ_SKILL=(); IUI_REQ_TOOL=(); IUI_REQ_COND=()
    IUI_REQ_STRENGTH=(); IUI_REQ_WHY=()
}

iui_platform() {
    [ -n "${IUI_UNAME_S:-}" ] || IUI_UNAME_S="$(uname -s)"
    [ -n "${IUI_UNAME_M:-}" ] || IUI_UNAME_M="$(uname -m)"
}

iui_cond_applies() {
    local cond="$1" os arch
    [ "$cond" = '*' ] && return 0
    iui_platform
    os="${cond%%:*}"
    # Unquoted patterns on purpose: MINGW*/CYGWIN* are globs.
    # shellcheck disable=SC2254
    case "$IUI_UNAME_S" in $os) : ;; *) return 1 ;; esac
    [ "$cond" = "$os" ] && return 0
    arch="${cond#*:}"
    # shellcheck disable=SC2254
    case "$IUI_UNAME_M" in $arch) return 0 ;; esac
    return 1
}

iui_req_applies() {
    [ "${IUI_REQ_SKILL[$1]}" = "$2" ] || return 1
    iui_cond_applies "${IUI_REQ_COND[$1]}"
}

# The one producer of the table, from section 4's generated tables and not a
# skill's requires.tsv: select_skills() runs before download_source(), so under
# `curl … | bash` no skill directory exists yet. Conditions are applied there.
iui_load_requirements() {
    local i skill tool
    iui_req_reset
    for ((i = 0; i < ${#IUI_SKILL_NAMES[@]}; i++)); do
        skill="${IUI_SKILL_NAMES[$i]}"
        while IFS= read -r tool; do
            [ -n "$tool" ] || continue
            iui_req_add "$i" "$tool" '*' \
                "$(runtime_requirement_strength "$skill" "$tool")" \
                "$(runtime_requirement_why "$skill" "$tool")"
        done < <(runtime_requirements "$skill")
    done
}

# ─────────────────────────────────────────────────────────────────────────────
# The global, per-tool dependency cache
# ─────────────────────────────────────────────────────────────────────────────

# Keyed by tool, never by skill: every read goes through iui_dep_state and every
# write through iui_dep_set, so one record serves every skill needing that tool.
iui_dep_index() {
    local wanted="$1" i
    IUI_DEP_INDEX=-1
    for ((i = 0; i < ${#IUI_DEP_TOOLS[@]}; i++)); do
        if [ "${IUI_DEP_TOOLS[$i]}" = "$wanted" ]; then
            IUI_DEP_INDEX="$i"
            return
        fi
    done
}

iui_dep_set() {
    iui_dep_index "$1"
    if [ "$IUI_DEP_INDEX" -lt 0 ]; then
        IUI_DEP_TOOLS+=("$1")
        IUI_DEP_STATES+=("$2")
        return
    fi
    IUI_DEP_STATES[$IUI_DEP_INDEX]="$2"
}

iui_dep_state() {
    iui_dep_index "$1"
    if [ "$IUI_DEP_INDEX" -lt 0 ]; then
        iui_dep_set "$1" unknown
        IUI_DEP_STATE=unknown
        return
    fi
    IUI_DEP_STATE="${IUI_DEP_STATES[$IUI_DEP_INDEX]}"
}

# The generated runtime_tool_verify() from section 4 is the authority for a
# single tool, and runtime_requirement_met() extends it to any-of groups, so
# the picker and verify_runtime_tools() can never disagree. Overridable so a
# test can inject a probe result without installing tools.
iui_dep_probe() {
    case $1 in
        @*) runtime_requirement_met "$1" ;;
        *) runtime_tool_verify "$1" ;;
    esac
}

iui_dep_verify() {
    if iui_dep_probe "$1"; then
        iui_dep_set "$1" ok
    else
        iui_dep_set "$1" missing
    fi
}

# Reverify every applicable tool of one skill. The writes land in the per-tool
# cache, so the refresh is global by construction.
iui_dep_reverify_skill() {
    local index="$1" i
    for ((i = 0; i < ${#IUI_REQ_SKILL[@]}; i++)); do
        iui_req_applies "$i" "$index" || continue
        iui_dep_verify "${IUI_REQ_TOOL[$i]}"
    done
}

# blocked beats degraded: an unmet hard requirement stops the install, an unmet
# soft one only costs capability. IUI_SKILL_BLOCKER names the first offender.
iui_skill_state() {
    local index="$1" i
    IUI_SKILL_STATE=ok
    IUI_SKILL_BLOCKER=''
    for ((i = 0; i < ${#IUI_REQ_SKILL[@]}; i++)); do
        iui_req_applies "$i" "$index" || continue
        iui_dep_state "${IUI_REQ_TOOL[$i]}"
        [ "$IUI_DEP_STATE" = "missing" ] || continue
        if [ "${IUI_REQ_STRENGTH[$i]}" = "hard" ]; then
            IUI_SKILL_STATE=blocked
            IUI_SKILL_BLOCKER="${IUI_REQ_TOOL[$i]}"
            return
        fi
        IUI_SKILL_STATE=degraded
        [ -n "$IUI_SKILL_BLOCKER" ] || IUI_SKILL_BLOCKER="${IUI_REQ_TOOL[$i]}"
    done
}

# ─────────────────────────────────────────────────────────────────────────────
# The published release, for the "latest release" row
# ─────────────────────────────────────────────────────────────────────────────

# Asked once per run and cached: the picker redraws on every keypress, so a
# per-render fetch would be one HTTP request per arrow key.
#
# raw.githubusercontent rather than the GitHub API: raw needs no token and has no
# 60-per-hour unauthenticated rate limit, and the repository is public. The cost
# is a CDN cache of a few minutes, so a version pushed seconds ago can still read
# as the old one -- which is why this row says "latest release" and not "HEAD".
IUI_RELEASE_URL="https://raw.githubusercontent.com/tschallacka/ai-skills/master/package.json"
IUI_RELEASE_VERSION=''
IUI_RELEASE_CHECKED=0
iui_release_version() {
    [ "$IUI_RELEASE_CHECKED" -eq 0 ] || return 0
    IUI_RELEASE_CHECKED=1
    # IUI_NO_NETWORK=1 is how a test pins the offline path.
    [ "${IUI_NO_NETWORK:-0}" -eq 0 ] || return 0
    command -v curl >/dev/null 2>&1 || return 0
    # A picker that hangs on a dead network is worse than one that cannot answer,
    # so the timeout is short and a failure leaves the answer empty rather than
    # reporting a version nobody fetched.
    IUI_RELEASE_VERSION="$(curl -fsSL --max-time 3 "$IUI_RELEASE_URL" 2>/dev/null \
        | awk -F'"' '/"version"/ { print $4; exit }')"
    return 0
}

# ─────────────────────────────────────────────────────────────────────────────
# Layout arithmetic
# ─────────────────────────────────────────────────────────────────────────────

# Rows: 1 title, 1 top border, IUI_BODY_ROWS body, 1 bottom border, 1 hint.
# Columns: 1 vertical + IUI_LEFT_W + 1 divider + IUI_RIGHT_W + 1 vertical.
# IUI_NARROW is the degrade path: one pane at a time, whichever has focus.
iui_layout() {
    IUI_BODY_ROWS=$((IUI_ROWS - 4))
    [ "$IUI_BODY_ROWS" -ge 1 ] || IUI_BODY_ROWS=1
    IUI_NARROW=0
    if [ "$IUI_COLS" -lt 56 ]; then
        IUI_NARROW=1
        IUI_LEFT_W=$((IUI_COLS - 2))
        [ "$IUI_LEFT_W" -ge 8 ] || IUI_LEFT_W=8
        IUI_RIGHT_W="$IUI_LEFT_W"
        iui_head_geometry
        return
    fi
    iui_list_width
    IUI_RIGHT_W=$((IUI_COLS - IUI_LEFT_W - 3))
    iui_head_geometry
}

# The list is sized to its content, not to a fraction of the terminal: a row is
# cursor(1) + checkbox(3) + space(1) + name + state tag(6), so the longest skill
# name decides the width and nothing truncates. A third of the terminal was the
# earlier rule and it capped at 34 columns, which cut
# post-implementation-review (26 chars, needing 37) on every screen size.
IUI_LIST_MIN_W=22
IUI_LIST_MAX_W=46
IUI_DETAIL_MIN_W=30
iui_list_width() {
    local i longest=0 length
    for ((i = 0; i < ${#IUI_SKILL_NAMES[@]}; i++)); do
        length="${#IUI_SKILL_NAMES[$i]}"
        [ "$length" -le "$longest" ] || longest="$length"
    done
    IUI_LEFT_W=$((longest + 11))
    [ "$IUI_LEFT_W" -ge "$IUI_LIST_MIN_W" ] || IUI_LEFT_W="$IUI_LIST_MIN_W"
    [ "$IUI_LEFT_W" -le "$IUI_LIST_MAX_W" ] || IUI_LEFT_W="$IUI_LIST_MAX_W"
    # The detail pane is the reason the picker exists; it wins a narrow screen.
    local ceiling=$((IUI_COLS - 3 - IUI_DETAIL_MIN_W))
    [ "$IUI_LEFT_W" -le "$ceiling" ] || IUI_LEFT_W="$ceiling"
    [ "$IUI_LEFT_W" -ge 8 ] || IUI_LEFT_W=8
}

# The sprite costs scale*32 columns and scale*16 rows, and is dropped rather than
# shrunk when it does not fit: with no colour it would paint blank, and in the
# narrow layout or under IUI_HEAD_MIN_TEXT_ROWS it would starve the info text.
# Where the sprite goes depends on how much room there is, and there are three
# answers rather than two:
#
#   right-top    a tall, wide terminal. The sprite sits at the top of the detail
#                pane and the empty space beside it -- which was blank before --
#                carries the hints.
#   left-bottom  an ordinary terminal. The sprite sits at the bottom of the list
#                pane, in the rows the list does not use, alone and with nothing
#                animating beside it. The detail pane keeps its full height.
#   none         no colour, a narrow layout, or not enough rows to spare.
#
# The deciding measurement is rows: the detail text must keep a real floor after
# the sprite has taken its band, or the hints would be bought with the content a
# reader came for.
IUI_HEAD_MIN_LIST_ROWS=6
IUI_HEAD_DETAIL_FLOOR=30
# The hints are drawn in the pixel font at six columns per character, so the
# space beside the sprite has to be wide enough for the longest of them. 34 was
# the figure for plain text and would have truncated every hint into nonsense.
IUI_HINT_MIN_COLS=108
iui_head_geometry() {
    IUI_HEAD_ON=0
    IUI_HEAD_SCALE=0
    IUI_HEAD_H=0
    IUI_HINT_ROWS=0
    IUI_HEAD_PLACE="none"
    IUI_LIST_ROWS="$IUI_BODY_ROWS"
    [ "$IUI_NARROW" -eq 0 ] || return 0
    [ -n "$COLOR_MODE" ] || detect_color_mode
    [ "$COLOR_MODE" != "none" ] || return 0
    # One size, always: 32x16. The sprite cannot shrink below it, and growing it
    # bought nothing while costing the detail pane its floor -- at 200x60 a
    # doubled sprite pushed the whole layout back into the small-screen case.
    IUI_HEAD_SCALE=1
    IUI_HEAD_H=16
    [ "$IUI_LEFT_W" -ge 32 ] || [ "$IUI_RIGHT_W" -ge $((32 + IUI_HINT_MIN_COLS)) ] || return 0
    IUI_HEAD_ON=1
    if [ $((IUI_BODY_ROWS - IUI_HEAD_H)) -ge "$IUI_HEAD_DETAIL_FLOOR" ] \
        && [ $((IUI_RIGHT_W - IUI_HEAD_SCALE * 32)) -ge "$IUI_HINT_MIN_COLS" ]; then
        IUI_HEAD_PLACE="right-top"
        IUI_HINT_ROWS="$IUI_HEAD_H"
        return 0
    fi
    if [ $((IUI_BODY_ROWS - IUI_HEAD_H - 1)) -ge "$IUI_HEAD_MIN_LIST_ROWS" ]; then
        IUI_HEAD_PLACE="left-bottom"
        IUI_LIST_ROWS=$((IUI_BODY_ROWS - IUI_HEAD_H - 1))
        return 0
    fi
    IUI_HEAD_ON=0
    IUI_HEAD_H=0
    return 0
}

iui_clamp_scroll() {
    local count rows
    count="${#IUI_SKILL_NAMES[@]}"
    rows="$IUI_LIST_ROWS"
    [ "$IUI_CURSOR" -ge 0 ] || IUI_CURSOR=0
    [ "$IUI_CURSOR" -lt "$count" ] || IUI_CURSOR=$((count - 1))
    [ "$IUI_CURSOR" -ge 0 ] || IUI_CURSOR=0
    [ "$IUI_SCROLL" -le "$IUI_CURSOR" ] || IUI_SCROLL="$IUI_CURSOR"
    if [ "$IUI_CURSOR" -ge $((IUI_SCROLL + rows)) ]; then
        IUI_SCROLL=$((IUI_CURSOR - rows + 1))
    fi
    if [ "$count" -le "$rows" ]; then
        IUI_SCROLL=0
    elif [ "$IUI_SCROLL" -gt $((count - rows)) ]; then
        IUI_SCROLL=$((count - rows))
    fi
    [ "$IUI_SCROLL" -ge 0 ] || IUI_SCROLL=0
}

# The info pane scrolls independently of the list. Without this the pane was
# fixed: tabbing to it highlighted it, but Up/Down still moved the skill cursor,
# so anything past the visible rows -- the whole detail block -- was unreachable.
iui_clamp_info_scroll() {
    IUI_INFO_ROWS=$((IUI_BODY_ROWS - IUI_HEAD_H))
    [ "$IUI_INFO_ROWS" -ge 1 ] || IUI_INFO_ROWS=1
    IUI_INFO_MAX_SCROLL=$(( ${#IUI_INFO_TEXT[@]} - IUI_INFO_ROWS ))
    [ "$IUI_INFO_MAX_SCROLL" -ge 0 ] || IUI_INFO_MAX_SCROLL=0
    [ "$IUI_INFO_SCROLL" -le "$IUI_INFO_MAX_SCROLL" ] || IUI_INFO_SCROLL="$IUI_INFO_MAX_SCROLL"
    [ "$IUI_INFO_SCROLL" -ge 0 ] || IUI_INFO_SCROLL=0
}

iui_measure() {
    local columns="${COLUMNS:-80}" lines="${LINES:-24}"
    if command -v tput >/dev/null 2>&1; then
        columns="$(tput cols 2>/dev/null || echo "$columns")"
        lines="$(tput lines 2>/dev/null || echo "$lines")"
    fi
    case "$columns" in ''|*[!0-9]*) columns=80 ;; esac
    case "$lines" in ''|*[!0-9]*) lines=24 ;; esac
    IUI_COLS="$columns"
    IUI_ROWS="$lines"
    [ "$IUI_COLS" -ge 20 ] || IUI_COLS=20
    [ "$IUI_ROWS" -ge 6 ] || IUI_ROWS=6
}
