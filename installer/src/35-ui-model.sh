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
IUI_SKILL_DESCS=()
IUI_SKILL_INSTALLED=()
IUI_SKILL_HAVE=()
IUI_SKILL_WANT=()
IUI_SKILL_SEL=()

# The global dependency cache. Keyed by TOOL, never by skill, so reverifying
# `jq` from one skill's info box refreshes every skill that also needs `jq`.
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
        IUI_G_RULE='-'; IUI_G_FILL='#'
        return
    fi
    IUI_G_TOP='▀'; IUI_G_BOT='▄'; IUI_G_V='▌'
    IUI_G_TL='▛'; IUI_G_TR='▜'; IUI_G_BL='▙'; IUI_G_BR='▟'
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
iui_pad() {
    local t="$1" w="$2"
    [ "$w" -ge 0 ] || w=0
    if [ "${#t}" -gt "$w" ]; then
        if [ "$w" -ge 1 ]; then t="${t:0:$((w - 1))}~"; else t=''; fi
    fi
    printf -v IUI_PAD '%s%*s' "$t" "$((w - ${#t}))" ''
}

iui_wrap() {
    local text="$1" width="$2" line split
    IUI_WRAP_LINES=()
    [ "$width" -ge 8 ] || width=8
    while [ -n "$text" ]; do
        line="$text"
        if [ "${#line}" -gt "$width" ]; then
            line="${text:0:width}"
            split="${line% *}"
            [ -n "$split" ] && [ "$split" != "$line" ] && line="$split"
        fi
        IUI_WRAP_LINES+=("$line")
        text="${text:${#line}}"
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
    IUI_LEFT_W=$((IUI_COLS / 3))
    [ "$IUI_LEFT_W" -le 34 ] || IUI_LEFT_W=34
    [ "$IUI_LEFT_W" -ge 22 ] || IUI_LEFT_W=22
    IUI_RIGHT_W=$((IUI_COLS - IUI_LEFT_W - 3))
    iui_head_geometry
}

# The sprite costs scale*32 columns and scale*16 rows, and is dropped rather than
# shrunk when it does not fit: with no colour it would paint blank, and in the
# narrow layout or under IUI_HEAD_MIN_TEXT_ROWS it would starve the info text.
IUI_HEAD_MIN_TEXT_ROWS=6
iui_head_geometry() {
    IUI_HEAD_ON=0
    IUI_HEAD_SCALE=0
    IUI_HEAD_H=0
    [ "$IUI_NARROW" -eq 0 ] || return 0
    [ -n "$COLOR_MODE" ] || detect_color_mode
    [ "$COLOR_MODE" != "none" ] || return 0
    local s
    for s in 1 2 3; do
        [ $((s * 32)) -le "$IUI_RIGHT_W" ] || break
        [ $((IUI_BODY_ROWS - s * 16)) -ge "$IUI_HEAD_MIN_TEXT_ROWS" ] || break
        IUI_HEAD_SCALE="$s"
    done
    [ "$IUI_HEAD_SCALE" -ge 1 ] || return 0
    IUI_HEAD_ON=1
    IUI_HEAD_H=$((IUI_HEAD_SCALE * 16))
}

iui_clamp_scroll() {
    local count="${#IUI_SKILL_NAMES[@]}" rows="$IUI_BODY_ROWS"
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
