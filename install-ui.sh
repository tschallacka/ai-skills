#!/usr/bin/env bash
# install-ui — full-screen keyboard-and-mouse skill picker for install.sh.
#
# Standalone and sourceable: it defines iui_* functions only, plus a --demo
# entry point and a headless --render mode that draws one frame to stdout so the
# layout can be asserted without a tty. Nothing here is wired into install.sh
# yet; iui_splash_hook() marks the point where install.sh's show_splash() runs
# first and hands the terminal over.
#
# Usage:
#   install-ui.sh --demo
#   install-ui.sh --render [--width N] [--height N] [--cursor N]
#                          [--focus list|info] [--eye front|left|right]
#                          [--color truecolor|256|8|none] [--glyphs blocks|ascii]
#   install-ui.sh --help
#
# Exit codes: 0 confirmed, 64 bad usage, 69 fd 3 is not a tty (the caller must
# fall back to the plain numeric menu), 130 aborted by the user.

set -euo pipefail

# No `export LC_ALL=C` at file scope: this file is sourceable and must not change
# the caller's locale. It byte-counts only ASCII, so widths are locale-free.

# ─────────────────────────────────────────────────────────────────────────────
# 1. State
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
COLOR_MODE="${COLOR_MODE:-}"

# 16 rows of 16 six-hex-digit pixels, each doubled horizontally so the sprite is
# square. Rows 8 and 9 are the eyes and are substituted per frame.
# TODO: fold IUI_ART and the iui_ colour/sprite helpers back into install.sh at integration.
IUI_ART=(
    'f2cf38 f2cf38 fdc100 fdc100 fcf246 fcf246 e8b11a e8b11a fcdb28 fcdb28 fcd228 fcd228 fdfd5e fcfd5f fcf347 fcf347'
    'f2cf38 f2cf38 fdc100 fdc100 fcf246 fcf246 e8b11a e8b11a fcdb28 fcdb28 fcd228 fcd228 fbfb5d fdfd5e fcf347 fcf347'
    'e8be38 e8be38 fcd84b fcd84b fdc100 fdc100 fcdb28 fcdb28 fcdb28 fcdb28 e8b11a e8b11a fddc51 fddc51 fdbb37 fdbb37'
    'e8be38 e8be38 fcd84b fcd84b fdc100 fdc100 fcdb28 fcdb28 fcdb28 fcdb28 e8b11a e8b11a fcdc51 fcdc51 fdbb37 fdbb37'
    'c37f18 c37f18 fdc127 fdc127 e6a621 e6a621 fcd22b fcd22b fdc127 fdc127 e6a621 e6a621 e6a621 e6a621 c37f18 c37f18'
    'c37f18 c37f18 fdc127 fdc127 e6a621 e6a621 fcd22b fcd22b fdc127 fdc127 e6a621 e6a621 e6a621 e6a621 c37f18 c37f18'
    'd8a521 d8a521 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 c37f18 c37f18'
    'd8a521 d8a521 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 c37f18 c37f18'
    'c27f18 c27f18 fbfbfb fbfbfb 009c00 009c00 c27417 c27417 dd8100 df8200 009c00 009c00 fbfbfb fbfbfb d68601 d68601'
    'c27f18 c27f18 6c3100 6c3100 c27f18 c27f18 67522d 67522d 67522d 67522d 883300 883300 c27f18 c27f18 6c3100 6c3100'
    '623b00 623b00 321400 321400 3a2910 3a2910 67522d 67522d 67522d 67522d 3a2910 3a2910 6c3100 6c3100 210000 210000'
    '623b00 623b00 321400 321400 3a2910 3a2910 67522d 67522d 67522d 67522d 3a2910 3a2910 6c3100 6c3100 210000 210000'
    '280c02 280c02 300d0a 300d0a 240a00 240a00 67522d 67522d 67522d 67522d 3e0907 3e0907 300d0a 300d0a 210000 210000'
    '280c02 280c02 300d0a 300d0a 240a00 240a00 67522d 67522d 67522d 67522d 3e0907 3e0907 300d0a 300d0a 210000 210000'
    '300d0a 300d0a 280c02 280c02 240a00 240a00 67522d 67522d 67522d 67522d 210000 210000 3e0907 3e0907 240a00 240a00'
    '300d0a 300d0a 280c02 280c02 240a00 240a00 67522d 67522d 67522d 67522d 210000 210000 3e0907 3e0907 240a00 240a00'
)

# ─────────────────────────────────────────────────────────────────────────────
# 2. Terminal capability and palette
# ─────────────────────────────────────────────────────────────────────────────

# 24-bit SGR is not universal, so probe once and degrade to 256, then 8, then no
# colour at all.
iui_detect_color_mode() {
    local colors=0
    if command -v tput >/dev/null 2>&1; then
        colors="$(tput colors 2>/dev/null || echo 0)"
    fi
    case "$colors" in
        ''|*[!0-9]*) colors=0 ;;
    esac
    case "${COLORTERM:-}" in
        truecolor|24bit)
            COLOR_MODE=truecolor
            return
            ;;
    esac
    if [ "$colors" -ge 16777216 ]; then
        COLOR_MODE=truecolor
    elif [ "$colors" -ge 256 ]; then
        COLOR_MODE=256
    elif [ "$colors" -ge 8 ]; then
        COLOR_MODE=8
    else
        COLOR_MODE=none
    fi
}

iui_fg_sgr() {
    local rgb="$1" attr="${2:-}" r g b rest
    [ -n "$COLOR_MODE" ] || iui_detect_color_mode
    case "$COLOR_MODE" in
        truecolor)
            printf -v IUI_FG_SGR '\033[%s38;2;%sm' "$attr" "$rgb"
            return
            ;;
        none)
            IUI_FG_SGR=''
            return
            ;;
    esac
    r="${rgb%%;*}"
    rest="${rgb#*;}"
    g="${rest%%;*}"
    b="${rest##*;}"
    if [ "$COLOR_MODE" = "256" ]; then
        printf -v IUI_FG_SGR '\033[%s38;5;%dm' "$attr" \
            "$(( 16 + 36 * (r * 5 / 255) + 6 * (g * 5 / 255) + (b * 5 / 255) ))"
    else
        printf -v IUI_FG_SGR '\033[%s3%dm' "$attr" \
            "$(( (r >= 128) + 2 * (g >= 128) + 4 * (b >= 128) ))"
    fi
}

iui_color_for() {
    IUI_COLOR=''
    if [ "${#1}" -eq 6 ]; then
        IUI_COLOR="$((16#${1:0:2}));$((16#${1:2:2}));$((16#${1:4:2}))"
    fi
}

# Minecraft block palette. iui_seg() wraps text in a role's colour; in the
# `none` colour mode every wrap is the identity, which is what keeps the ASCII
# fallback free of escape bytes.
iui_seg() {
    local role="$1" text="$2" rgb='' attr=''
    [ -n "$COLOR_MODE" ] || iui_detect_color_mode
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
    iui_fg_sgr "$rgb" "$attr"
    IUI_SEG="$IUI_FG_SGR$text"$'\033'"[0m"
}

# ─────────────────────────────────────────────────────────────────────────────
# 3. Glyph sets and the plain-text fold
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
# 4. The requirement table — the data seam
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

# ─────────────────────────────────────────────────────────────────────────────
# 5. The global, per-tool dependency cache
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

# Overridable so a test can inject a probe result without installing tools.
iui_dep_probe() {
    command -v "$1" >/dev/null 2>&1
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

# Per-platform install hint for one tool, into IUI_HINT_LINES. Condensed from
# install.sh's runtime_tool_install_hint(); duplicated pending integration.
iui_dep_hint() {
    local tool="$1"
    IUI_HINT_LINES=()
    if [ "$tool" = "memlimit" ]; then
        IUI_HINT_LINES+=('curl -LsSf https://github.com/pingiun/memlimit/releases/latest/download/memlimit-installer.sh | sh')
        IUI_HINT_LINES+=('(MIT, by Jelle Besseling; Apple Silicon only)')
        return
    fi
    case "$(uname -s)" in
        Darwin)
            if command -v brew >/dev/null 2>&1; then IUI_HINT_LINES+=("brew install $tool")
            elif command -v port >/dev/null 2>&1; then IUI_HINT_LINES+=("sudo port install $tool")
            else IUI_HINT_LINES+=("install Homebrew (https://brew.sh) then: brew install $tool"); fi
            ;;
        Linux)
            if command -v apt-get >/dev/null 2>&1; then IUI_HINT_LINES+=("sudo apt-get install -y $tool")
            elif command -v dnf >/dev/null 2>&1; then IUI_HINT_LINES+=("sudo dnf install -y $tool")
            elif command -v pacman >/dev/null 2>&1; then IUI_HINT_LINES+=("sudo pacman -S --noconfirm $tool")
            elif command -v apk >/dev/null 2>&1; then IUI_HINT_LINES+=("sudo apk add $tool")
            else IUI_HINT_LINES+=("install $tool with your package manager"); fi
            ;;
        MINGW*|MSYS*|CYGWIN*)
            if command -v winget >/dev/null 2>&1; then IUI_HINT_LINES+=("winget install $tool")
            elif command -v choco >/dev/null 2>&1; then IUI_HINT_LINES+=("choco install $tool")
            else IUI_HINT_LINES+=("install $tool with your package manager"); fi
            ;;
        *) IUI_HINT_LINES+=("install $tool with your package manager") ;;
    esac
}

# ─────────────────────────────────────────────────────────────────────────────
# 6. Layout arithmetic
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
    [ -n "$COLOR_MODE" ] || iui_detect_color_mode
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

# ─────────────────────────────────────────────────────────────────────────────
# 7. The animated head
# ─────────────────────────────────────────────────────────────────────────────

# Sprite rows 8 and 9 carry the same pixels in every eye state, so one row per
# state is enough for both.
iui_eye_row() {
    case "$1" in
        left)
            IUI_EYE_ROW='c27f18 c27f18 009c00 009c00 fbfbfb fbfbfb c27417 c27417 df8200 df8200 009c00 009c00 fbfbfb fbfbfb d68601 d68601'
            ;;
        right)
            IUI_EYE_ROW='c27f18 c27f18 fbfbfb fbfbfb 009c00 009c00 c27417 c27417 df8200 df8200 fbfbfb fbfbfb 009c00 009c00 d68601 d68601'
            ;;
        *)
            IUI_EYE_ROW='c27f18 c27f18 fbfbfb fbfbfb 009c00 009c00 c27417 c27417 df8200 df8200 009c00 009c00 fbfbfb fbfbfb d68601 d68601'
            ;;
    esac
}

IUI_EYE_CYCLE='front right front left'
iui_advance_eye() {
    local first='' seen=0 state next=''
    for state in $IUI_EYE_CYCLE; do
        [ -n "$first" ] || first="$state"
        if [ "$seen" -eq 1 ]; then next="$state"; break; fi
        [ "$state" = "$IUI_EYE" ] && seen=1
    done
    [ -n "$next" ] || next="$first"
    IUI_EYE="$next"
}

# One sprite row as a string of exactly $3 display cells: 16 pixels of
# scale*2 fill glyphs each, then padding out to the pane width.
iui_head_line() {
    local art_y="$1" scale="$2" width="$3" eye="$4"
    local row blocks out='' pad x
    local -a pixels
    row="${IUI_ART[$art_y]}"
    if [ "$art_y" -eq 8 ] || [ "$art_y" -eq 9 ]; then
        iui_eye_row "$eye"
        row="$IUI_EYE_ROW"
    fi
    iui_repeat "$IUI_G_FILL" $((scale * 2))
    blocks="$IUI_REPEAT"
    IFS=' ' read -r -a pixels <<< "$row"
    printf -v pad '%*s' $((scale * 2)) ''
    for ((x = 0; x < ${#pixels[@]}; x++)); do
        iui_color_for "${pixels[$x]}"
        if [ -n "$IUI_COLOR" ]; then
            iui_fg_sgr "$IUI_COLOR"
            out="$out$IUI_FG_SGR$blocks"$'\033'"[0m"
        else
            out="$out$pad"
        fi
    done
    printf -v pad '%*s' $((width - scale * 32)) ''
    IUI_HEAD_LINE="$out$pad"
}

# The eyes are the only thing that changes without user input, so their redraw
# is separate from the full frame: two sprite rows, absolutely positioned, and
# nothing else is touched. That is why iui_head_line() is width-parameterised.
iui_redraw_eyes() {
    [ "$IUI_HEAD_ON" -eq 1 ] || return 0
    [ "$IUI_POSITION" -eq 1 ] || return 0
    local scale="$IUI_HEAD_SCALE" col=$((IUI_LEFT_W + 3)) art_y r row
    for art_y in 8 9; do
        iui_head_line "$art_y" "$scale" $((scale * 32)) "$IUI_EYE"
        for ((r = 0; r < scale; r++)); do
            row=$((3 + art_y * scale + r))
            printf '\033[%d;%dH%s' "$row" "$col" "$IUI_HEAD_LINE"
        done
    done
    printf '\033[%d;1H' "$IUI_ROWS"
}

# ─────────────────────────────────────────────────────────────────────────────
# 8. Frame rendering
# ─────────────────────────────────────────────────────────────────────────────

# One renderer serves both modes: IUI_POSITION=1 prefixes an absolute cursor
# move (interactive), IUI_POSITION=0 emits plain lines (headless, assertable).
iui_out_line() {
    if [ "$IUI_POSITION" -eq 1 ]; then
        printf '\033[%d;1H\033[K%s' "$1" "$2"
    else
        printf '%s\n' "$2"
    fi
}

# A header segment of exactly $2 cells: one edge glyph, the label, then filler.
# A focused pane's label is bracketed, so the focus is assertable without colour.
iui_header_seg() {
    local label="$1" width="$2" focused="$3" text head
    if [ "$focused" -eq 1 ]; then text="[$label]"; else text=" $label "; fi
    [ "${#text}" -le $((width - 2)) ] || text="${text:0:$((width - 2))}"
    iui_repeat "$IUI_G_TOP" 1
    iui_seg gold "$text"
    head="$IUI_REPEAT$IUI_SEG"
    iui_repeat "$IUI_G_TOP" $((width - 1 - ${#text}))
    iui_seg dirt "$IUI_REPEAT"
    IUI_HEADER_SEG="$head$IUI_SEG"
}

# The row carries the requirement verdict as a word as well as a colour, so the
# three states stay apart in 8-colour mode and in the ASCII fallback. The tag is
# dropped below 16 cells, where there is no room for a name beside it.
iui_list_cell() {
    local index="$1" width="$2" role box cursor tag='' tag_w=0
    if [ "${IUI_SKILL_SEL[$index]}" -eq 1 ]; then box="[$IUI_G_FILL]"; else box='[ ]'; fi
    if [ "$index" -eq "$IUI_CURSOR" ]; then cursor='>'; else cursor=' '; fi
    role=stone
    [ "${IUI_SKILL_SEL[$index]}" -eq 1 ] && role=grass
    [ "$index" -eq "$IUI_CURSOR" ] && role=gold
    iui_skill_state "$index"
    if [ "$width" -ge 16 ]; then
        tag_w=6
        case "$IUI_SKILL_STATE" in
            blocked) tag=' block' ;;
            degraded) tag='  warn' ;;
            *) tag='    ok' ;;
        esac
    fi
    iui_pad " ${IUI_SKILL_NAMES[$index]}" $((width - 4 - tag_w))
    iui_seg "$role" "$cursor$box$IUI_PAD"
    IUI_LIST_CELL="$IUI_SEG"
    [ "$tag_w" -eq 0 ] && return
    iui_seg "$(iui_state_role "$IUI_SKILL_STATE")" "$tag"
    IUI_LIST_CELL="$IUI_LIST_CELL$IUI_SEG"
}

# Builds the info text into IUI_INFO_TEXT (padded cells), IUI_INFO_ROLE and
# IUI_INFO_TAG, index-parallel. IUI_INFO_TAG marks the two action lines so a
# mouse click can be mapped back to an action after the frame is drawn.
iui_info_lines() {
    local width="$1" index="$IUI_CURSOR" tool line state up
    IUI_INFO_TEXT=(); IUI_INFO_ROLE=(); IUI_INFO_TAG=()
    iui_repeat "$IUI_G_RULE" "$width"
    IUI_INFO_TEXT+=("$IUI_REPEAT"); IUI_INFO_ROLE+=(dirt); IUI_INFO_TAG+=(rule)
    iui_info_push gold "${IUI_SKILL_NAMES[$index]}" name "$width"
    iui_wrap "${IUI_SKILL_DESCS[$index]}" "$width"
    for line in "${IUI_WRAP_LINES[@]}"; do
        iui_info_push stone "$line" body "$width"
    done
    iui_info_push diamond 'DEPENDENCIES' body "$width"
    iui_info_requirements "$index" "$width"
    iui_info_push diamond 'STATUS' body "$width"
    iui_skill_state "$index"
    printf -v line '  %-14s %s' 'install' "$(iui_install_verdict)"
    iui_info_push "$(iui_state_role "$IUI_SKILL_STATE")" "$line" body "$width"
    printf -v line '  %-14s %s' 'installed' "${IUI_SKILL_INSTALLED[$index]}"
    iui_info_push stone "$line" body "$width"
    up="$(iui_uptodate_text "$index")"
    printf -v line '  %-14s %s' 'up to date' "$up"
    state=redstone
    [ "$up" = "yes" ] && state=diamond
    iui_info_push "$state" "$line" body "$width"
    iui_info_actions "$width"
    iui_info_message "$width"
}

# Tool, strength and state on one line; for an unmet requirement the next line
# says what is lost, which is the part that matters for a soft one.
iui_info_requirements() {
    local index="$1" width="$2" i line role any=0 wrapped
    for ((i = 0; i < ${#IUI_REQ_SKILL[@]}; i++)); do
        iui_req_applies "$i" "$index" || continue
        any=1
        iui_dep_state "${IUI_REQ_TOOL[$i]}"
        role=stone
        [ "$IUI_DEP_STATE" = "ok" ] && role=diamond
        if [ "$IUI_DEP_STATE" = "missing" ]; then
            role=gold
            [ "${IUI_REQ_STRENGTH[$i]}" = "hard" ] && role=redstone
        fi
        printf -v line '  %-14s %-5s %s' \
            "${IUI_REQ_TOOL[$i]}" "${IUI_REQ_STRENGTH[$i]}" "$IUI_DEP_STATE"
        iui_info_push "$role" "$line" body "$width"
        [ "$IUI_DEP_STATE" = "missing" ] || continue
        iui_wrap "${IUI_REQ_WHY[$i]}" $((width - 4))
        for wrapped in "${IUI_WRAP_LINES[@]}"; do
            iui_info_push "$role" "    $wrapped" body "$width"
        done
    done
    [ "$any" -eq 1 ] || iui_info_push stone '  (none)' body "$width"
}

iui_state_role() {
    case "$1" in
        blocked) printf 'redstone' ;;
        degraded) printf 'gold' ;;
        *) printf 'diamond' ;;
    esac
}

iui_install_verdict() {
    case "$IUI_SKILL_STATE" in
        blocked) printf 'blocked (%s missing)' "$IUI_SKILL_BLOCKER" ;;
        degraded) printf 'allowed, degraded (%s missing)' "$IUI_SKILL_BLOCKER" ;;
        *) printf 'allowed' ;;
    esac
}

iui_uptodate_text() {
    local index="$1"
    if [ "${IUI_SKILL_INSTALLED[$index]}" != "yes" ]; then
        printf 'not installed'
    elif [ "${IUI_SKILL_HAVE[$index]}" = "${IUI_SKILL_WANT[$index]}" ]; then
        printf 'yes'
    else
        printf 'no (%s -> %s)' "${IUI_SKILL_HAVE[$index]}" "${IUI_SKILL_WANT[$index]}"
    fi
}

iui_info_push() {
    local role="$1" text="$2" tag="$3" width="$4"
    iui_pad "$text" "$width"
    IUI_INFO_TEXT+=("$IUI_PAD"); IUI_INFO_ROLE+=("$role"); IUI_INFO_TAG+=("$tag")
}

# The actions are always listed; they are only *usable* when the info pane has
# focus, and the leading marker says which state they are in.
iui_info_actions() {
    local width="$1" marker role
    if [ "$IUI_FOCUS" = "info" ]; then marker='>'; role=gold; else marker='-'; role=stone; fi
    iui_info_push diamond 'ACTIONS' body "$width"
    iui_info_push "$role" " $marker d  help me install dependencies" act-dep "$width"
    iui_info_push "$role" " $marker r  reverify dependencies" act-verify "$width"
}

iui_info_message() {
    local width="$1" line wrapped
    [ "${#IUI_MESSAGE[@]}" -gt 0 ] || return 0
    for line in "${IUI_MESSAGE[@]}"; do
        iui_wrap "$line" "$width"
        for wrapped in "${IUI_WRAP_LINES[@]}"; do
            iui_info_push gold "$wrapped" body "$width"
        done
    done
}

iui_title_bar() {
    local text
    printf -v text ' %s  %d/%d selected ' 'TSCHALLACKA SKILL SHOP' \
        "$(iui_selected_count)" "${#IUI_SKILL_NAMES[@]}"
    iui_pad "$text" "$IUI_COLS"
    iui_seg gold "$IUI_PAD"
    iui_out_line 1 "$IUI_SEG"
}

iui_hint_bar() {
    iui_pad ' Up/Dn move  Enter/Space toggle  click toggle  Tab focus  a all  n none  i install  q quit' "$IUI_COLS"
    iui_seg stone "$IUI_PAD"
    iui_out_line "$IUI_ROWS" "$IUI_SEG"
}

iui_selected_count() {
    local i n=0
    for ((i = 0; i < ${#IUI_SKILL_SEL[@]}; i++)); do
        [ "${IUI_SKILL_SEL[$i]}" -eq 1 ] && n=$((n + 1))
    done
    printf '%d' "$n"
}

iui_render_frame() {
    iui_layout
    iui_clamp_scroll
    IUI_ACTION_ROW_DEP=0
    IUI_ACTION_ROW_VERIFY=0
    [ "$IUI_POSITION" -eq 1 ] && printf '\033[H'
    iui_title_bar
    if [ "$IUI_NARROW" -eq 1 ]; then
        iui_render_narrow
    else
        iui_render_wide
    fi
    iui_hint_bar
    [ "$IUI_POSITION" -eq 1 ] && printf '\033[%d;1H' "$IUI_ROWS"
    return 0
}

iui_top_border() {
    local left right tl td tr focus_list=0 focus_info=0
    [ "$IUI_FOCUS" = "list" ] && focus_list=1
    [ "$IUI_FOCUS" = "info" ] && focus_info=1
    iui_header_seg SKILLS "$IUI_LEFT_W" "$focus_list"
    left="$IUI_HEADER_SEG"
    iui_header_seg DETAILS "$IUI_RIGHT_W" "$focus_info"
    right="$IUI_HEADER_SEG"
    iui_seg dirt "$IUI_G_TL"; tl="$IUI_SEG"
    iui_seg dirt "$IUI_G_TOP"; td="$IUI_SEG"
    iui_seg dirt "$IUI_G_TR"; tr="$IUI_SEG"
    iui_out_line 2 "$tl$left$td$right$tr"
}

iui_bottom_border() {
    local bl bm br
    iui_repeat "$IUI_G_BOT" $((IUI_COLS - 3))
    iui_seg dirt "$IUI_G_BL"; bl="$IUI_SEG"
    iui_seg dirt "$IUI_REPEAT$IUI_G_BOT"; bm="$IUI_SEG"
    iui_seg dirt "$IUI_G_BR"; br="$IUI_SEG"
    iui_out_line $((IUI_ROWS - 1)) "$bl$bm$br"
}

# The right cell is the sprite for its first IUI_HEAD_H rows and info text after
# that, so the head and the text share one row budget and neither can overflow.
iui_right_cell() {
    local body="$1" row="$2"
    if [ "$IUI_HEAD_ON" -eq 1 ] && [ "$body" -lt "$IUI_HEAD_H" ]; then
        iui_head_line $((body / IUI_HEAD_SCALE)) "$IUI_HEAD_SCALE" "$IUI_RIGHT_W" "$IUI_EYE"
        IUI_INFO_CELL="$IUI_HEAD_LINE"
        return
    fi
    iui_info_cell $((body - IUI_HEAD_H)) "$row"
}

iui_render_wide() {
    local row body index v
    iui_top_border
    iui_seg dirt "$IUI_G_V"; v="$IUI_SEG"
    iui_info_lines "$IUI_RIGHT_W"
    for ((body = 0; body < IUI_BODY_ROWS; body++)); do
        row=$((body + 3))
        index=$((IUI_SCROLL + body))
        if [ "$index" -lt "${#IUI_SKILL_NAMES[@]}" ]; then
            iui_list_cell "$index" "$IUI_LEFT_W"
        else
            iui_pad '' "$IUI_LEFT_W"; iui_seg stone "$IUI_PAD"; IUI_LIST_CELL="$IUI_SEG"
        fi
        iui_right_cell "$body" "$row"
        iui_out_line "$row" "$v$IUI_LIST_CELL$v$IUI_INFO_CELL$v"
    done
    iui_bottom_border
}

iui_info_cell() {
    local i="$1" row="$2"
    if [ "$i" -lt 0 ] || [ "$i" -ge "${#IUI_INFO_TEXT[@]}" ]; then
        iui_pad '' "$IUI_RIGHT_W"
        iui_seg stone "$IUI_PAD"
        IUI_INFO_CELL="$IUI_SEG"
        return
    fi
    case "${IUI_INFO_TAG[$i]}" in
        act-dep) IUI_ACTION_ROW_DEP="$row" ;;
        act-verify) IUI_ACTION_ROW_VERIFY="$row" ;;
    esac
    iui_seg "${IUI_INFO_ROLE[$i]}" "${IUI_INFO_TEXT[$i]}"
    IUI_INFO_CELL="$IUI_SEG"
}

# Degrade path for a terminal too narrow for two panes: one pane at a time,
# chosen by focus, in the same row budget, and no sprite. Nothing overflows and
# nothing is half-drawn — a plain box beats a broken one.
iui_render_narrow() {
    local row body index label v
    label=SKILLS
    [ "$IUI_FOCUS" = "info" ] && label=DETAILS
    iui_header_seg "$label" "$IUI_LEFT_W" 1
    iui_seg dirt "$IUI_G_TL"; local tl="$IUI_SEG"
    iui_seg dirt "$IUI_G_TR"; local tr="$IUI_SEG"
    iui_out_line 2 "$tl$IUI_HEADER_SEG$tr"
    iui_seg dirt "$IUI_G_V"; v="$IUI_SEG"
    IUI_RIGHT_W="$IUI_LEFT_W"
    [ "$IUI_FOCUS" = "info" ] && iui_info_lines "$IUI_LEFT_W"
    for ((body = 0; body < IUI_BODY_ROWS; body++)); do
        row=$((body + 3))
        index=$((IUI_SCROLL + body))
        if [ "$IUI_FOCUS" = "info" ]; then
            iui_info_cell "$body" "$row"
            iui_out_line "$row" "$v$IUI_INFO_CELL$v"
            continue
        fi
        if [ "$index" -lt "${#IUI_SKILL_NAMES[@]}" ]; then
            iui_list_cell "$index" "$IUI_LEFT_W"
        else
            iui_pad '' "$IUI_LEFT_W"; iui_seg stone "$IUI_PAD"; IUI_LIST_CELL="$IUI_SEG"
        fi
        iui_out_line "$row" "$v$IUI_LIST_CELL$v"
    done
    iui_repeat "$IUI_G_BOT" "$IUI_LEFT_W"
    iui_seg dirt "$IUI_G_BL$IUI_REPEAT$IUI_G_BR"
    iui_out_line $((IUI_ROWS - 1)) "$IUI_SEG"
}

# ─────────────────────────────────────────────────────────────────────────────
# 9. Input — keys and SGR mouse, read from fd 3
# ─────────────────────────────────────────────────────────────────────────────

# Keys come from fd 3 (open on /dev/tty, so prompts survive `curl … | bash`) and
# escapes go to stdout. A bare ESC blocks for one more byte; `q` is the quit key.
iui_read_byte() {
    IUI_BYTE=''
    IFS= read -r -n 1 -u 3 IUI_BYTE || return 1
    [ -n "$IUI_BYTE" ] || IUI_BYTE=$'\n'
    return 0
}

# Returns 2 on timeout (an idle animation tick) and 1 on EOF (quit).
# PORTABILITY(read-timeout-integer): 1s is the finest tick available here, so the
# eyes blink on whole seconds and ESC cannot be disambiguated by timing.
iui_read_first_byte() {
    local rc=0
    IUI_BYTE=''
    IFS= read -r -n 1 -t 1 -u 3 IUI_BYTE || rc="$?"
    [ "$rc" -le 128 ] || return 2
    [ "$rc" -eq 0 ] || return 1
    [ -n "$IUI_BYTE" ] || IUI_BYTE=$'\n'
    return 0
}

# ---- quoted: SGR mouse report ----
# \033[<{btn};{col};{row}M   press
# \033[<{btn};{col};{row}m   release
# ---- end quoted ----
# Wheel (btn 64/65) and drag (btn with the 32 bit set) are parsed and then
# ignored — nothing in this UI should answer a wheel or a drag.
iui_read_mouse() {
    local buf='' rest
    IUI_MOUSE_RELEASE=0
    while iui_read_byte; do
        case "$IUI_BYTE" in
            M) break ;;
            m) IUI_MOUSE_RELEASE=1; break ;;
            *) buf="$buf$IUI_BYTE" ;;
        esac
        [ "${#buf}" -lt 24 ] || break
    done
    IUI_MOUSE_BTN="${buf%%;*}"
    rest="${buf#*;}"
    IUI_MOUSE_COL="${rest%%;*}"
    IUI_MOUSE_ROW="${rest##*;}"
    case "$IUI_MOUSE_BTN" in ''|*[!0-9]*) IUI_MOUSE_BTN=99 ;; esac
    case "$IUI_MOUSE_COL" in ''|*[!0-9]*) IUI_MOUSE_COL=0 ;; esac
    case "$IUI_MOUSE_ROW" in ''|*[!0-9]*) IUI_MOUSE_ROW=0 ;; esac
}

iui_read_key() {
    local rc=0
    iui_read_first_byte || rc="$?"
    if [ "$rc" -eq 2 ]; then IUI_KEY=TICK; return 0; fi
    if [ "$rc" -ne 0 ]; then IUI_KEY=EOF; return 0; fi
    case "$IUI_BYTE" in
        $'\033') iui_read_escape ;;
        $'\n'|$'\r') IUI_KEY=ENTER ;;
        ' ') IUI_KEY=SPACE ;;
        $'\003') IUI_KEY=q ;;
        *) IUI_KEY="$IUI_BYTE" ;;
    esac
    return 0
}

iui_read_escape() {
    iui_read_byte || { IUI_KEY=ESC; return 0; }
    case "$IUI_BYTE" in
        '[') : ;;
        'O') iui_read_byte || { IUI_KEY=ESC; return 0; }; iui_key_from_final; return 0 ;;
        *) IUI_KEY=ESC; return 0 ;;
    esac
    iui_read_byte || { IUI_KEY=ESC; return 0; }
    case "$IUI_BYTE" in
        '<') iui_read_mouse; IUI_KEY=MOUSE ;;
        [0-9]) iui_read_tilde "$IUI_BYTE" ;;
        *) iui_key_from_final ;;
    esac
    return 0
}

iui_key_from_final() {
    case "$IUI_BYTE" in
        A) IUI_KEY=UP ;;
        B) IUI_KEY=DOWN ;;
        C) IUI_KEY=RIGHT ;;
        D) IUI_KEY=LEFT ;;
        H) IUI_KEY=HOME ;;
        F) IUI_KEY=END ;;
        Z) IUI_KEY=SHIFTTAB ;;
        *) IUI_KEY=UNKNOWN ;;
    esac
}

iui_read_tilde() {
    local digits="$1"
    while iui_read_byte; do
        case "$IUI_BYTE" in
            [0-9]|';') digits="$digits$IUI_BYTE" ;;
            *) break ;;
        esac
        [ "${#digits}" -lt 12 ] || break
    done
    case "$digits" in
        1|7) IUI_KEY=HOME ;;
        4|8) IUI_KEY=END ;;
        5) IUI_KEY=PGUP ;;
        6) IUI_KEY=PGDN ;;
        *) IUI_KEY=UNKNOWN ;;
    esac
}

# ─────────────────────────────────────────────────────────────────────────────
# 10. Terminal enter/leave
# ─────────────────────────────────────────────────────────────────────────────

# Install the trap before the first escape byte is written; iui_term_leave() is
# idempotent. IUI_NO_STTY=1 skips the stty calls so a test can drive this without
# owning a tty.
iui_term_enter() {
    IUI_STTY_SAVED=""
    if [ "${IUI_NO_STTY:-0}" -ne 1 ]; then
        IUI_STTY_SAVED="$(stty -g <&3 2>/dev/null || true)"
        stty raw -echo <&3 2>/dev/null || true
    fi
    IUI_TERM_ACTIVE=1
    # ---- quoted: enter sequences ----
    # \033[?1049h  alternate screen
    # \033[?25l    hide cursor
    # \033[?1000h  mouse tracking
    # \033[?1006h  SGR mouse encoding
    # ---- end quoted ----
    printf '\033[?1049h\033[?25l\033[?1000h\033[?1006h\033[2J\033[H'
}

iui_term_leave() {
    [ "$IUI_TERM_ACTIVE" -eq 1 ] || return 0
    IUI_TERM_ACTIVE=0
    # Every enable from iui_term_enter, disabled in reverse order.
    printf '\033[?1006l\033[?1000l\033[?25h\033[?1049l'
    if [ -n "$IUI_STTY_SAVED" ] && [ "${IUI_NO_STTY:-0}" -ne 1 ]; then
        stty "$IUI_STTY_SAVED" <&3 2>/dev/null || true
    fi
}

iui_install_traps() {
    trap 'iui_term_leave' EXIT
    trap 'iui_term_leave; exit 130' INT
    trap 'iui_term_leave; exit 143' TERM
}

# The splash animation runs here, on the normal screen, before iui_run() takes
# the terminal over. A no-op so this file stays standalone.
iui_splash_hook() {
    return 0
}

# ─────────────────────────────────────────────────────────────────────────────
# 11. Event loop
# ─────────────────────────────────────────────────────────────────────────────

# Selecting a blocked skill is refused with the reason rather than allowed and
# then rejected by the installer. Deselecting is always allowed.
iui_toggle() {
    local index="$1"
    [ "$index" -ge 0 ] && [ "$index" -lt "${#IUI_SKILL_NAMES[@]}" ] || return 0
    if [ "${IUI_SKILL_SEL[$index]}" -eq 1 ]; then
        IUI_SKILL_SEL[$index]=0
        return 0
    fi
    iui_skill_state "$index"
    if [ "$IUI_SKILL_STATE" = "blocked" ]; then
        IUI_MESSAGE=("cannot select ${IUI_SKILL_NAMES[$index]}: $IUI_SKILL_BLOCKER is required and missing")
        return 0
    fi
    IUI_SKILL_SEL[$index]=1
}

iui_action_dep_hint() {
    local index="$IUI_CURSOR" i any=0 line
    IUI_MESSAGE=('HOW TO INSTALL THE MISSING DEPENDENCIES')
    for ((i = 0; i < ${#IUI_REQ_SKILL[@]}; i++)); do
        iui_req_applies "$i" "$index" || continue
        iui_dep_state "${IUI_REQ_TOOL[$i]}"
        [ "$IUI_DEP_STATE" = "missing" ] || continue
        any=1
        iui_dep_hint "${IUI_REQ_TOOL[$i]}"
        IUI_MESSAGE+=("${IUI_REQ_TOOL[$i]} (${IUI_REQ_STRENGTH[$i]}): ${IUI_REQ_WHY[$i]}")
        for line in "${IUI_HINT_LINES[@]}"; do
            IUI_MESSAGE+=("  $line")
        done
    done
    [ "$any" -eq 1 ] || IUI_MESSAGE+=('nothing missing for this skill')
}

iui_action_reverify() {
    iui_dep_reverify_skill "$IUI_CURSOR"
    IUI_MESSAGE=('reverified; the per-tool cache is shared by every skill')
}

iui_handle_mouse() {
    [ "$IUI_MOUSE_RELEASE" -eq 0 ] || return 0
    case "$IUI_MOUSE_BTN" in
        64|65) return 0 ;;
    esac
    [ "$IUI_MOUSE_BTN" -lt 32 ] || return 0
    local first=3 last=$((IUI_BODY_ROWS + 2)) index
    [ "$IUI_MOUSE_ROW" -ge "$first" ] && [ "$IUI_MOUSE_ROW" -le "$last" ] || return 0
    if [ "$IUI_NARROW" -eq 0 ] && [ "$IUI_MOUSE_COL" -gt $((IUI_LEFT_W + 1)) ]; then
        IUI_FOCUS=info
        [ "$IUI_MOUSE_ROW" -eq "$IUI_ACTION_ROW_DEP" ] && iui_action_dep_hint
        [ "$IUI_MOUSE_ROW" -eq "$IUI_ACTION_ROW_VERIFY" ] && iui_action_reverify
        return 0
    fi
    [ "$IUI_FOCUS" = "info" ] && return 0
    index=$((IUI_SCROLL + IUI_MOUSE_ROW - first))
    [ "$index" -lt "${#IUI_SKILL_NAMES[@]}" ] || return 0
    IUI_CURSOR="$index"
    iui_toggle "$index"
}

iui_handle_key() {
    local count="${#IUI_SKILL_NAMES[@]}" i
    case "$1" in
        UP|k) IUI_CURSOR=$((IUI_CURSOR - 1)) ;;
        DOWN|j) IUI_CURSOR=$((IUI_CURSOR + 1)) ;;
        PGUP) IUI_CURSOR=$((IUI_CURSOR - IUI_BODY_ROWS)) ;;
        PGDN) IUI_CURSOR=$((IUI_CURSOR + IUI_BODY_ROWS)) ;;
        HOME) IUI_CURSOR=0 ;;
        END) IUI_CURSOR=$((count - 1)) ;;
        ENTER|SPACE) iui_toggle "$IUI_CURSOR" ;;
        $'\t') if [ "$IUI_FOCUS" = "list" ]; then IUI_FOCUS=info; else IUI_FOCUS=list; fi ;;
        SHIFTTAB) if [ "$IUI_FOCUS" = "info" ]; then IUI_FOCUS=list; else IUI_FOCUS=info; fi ;;
        a) for ((i = 0; i < count; i++)); do
               IUI_SKILL_SEL[$i]=0
               iui_toggle "$i"
           done ;;
        n) for ((i = 0; i < count; i++)); do IUI_SKILL_SEL[$i]=0; done ;;
        d) [ "$IUI_FOCUS" = "info" ] && iui_action_dep_hint ;;
        r) [ "$IUI_FOCUS" = "info" ] && iui_action_reverify ;;
        i) IUI_DONE=1; IUI_RC=0 ;;
        q|ESC|EOF) IUI_DONE=1; IUI_RC=130 ;;
        MOUSE) iui_handle_mouse ;;
    esac
    return 0
}

# Only for the standalone --demo; a caller that already owns fd 3 must not have
# it reopened underneath it.

# PORTABILITY(tty-probe): the open runs in a subshell first, so a failed
# redirection cannot take a non-interactive shell down with it.
iui_open_input() {
    if [ -t 0 ]; then
        exec 3<&0
    elif ( exec 3</dev/tty ) 2>/dev/null; then
        exec 3</dev/tty
    else
        exec 3<&-
    fi
}

# Returns 69 without touching the terminal when fd 3 is not a tty, so the caller
# can fall back to the plain menu. An idle tick redraws only the two eye rows; a
# keypress redraws the whole frame.
iui_run() {
    [ -t 3 ] || return 69
    [ -n "$COLOR_MODE" ] || iui_detect_color_mode
    iui_set_glyphs "$IUI_GLYPHS"
    iui_install_traps
    iui_term_enter
    IUI_POSITION=1
    IUI_DONE=0
    IUI_RC=130
    local need_full=1
    while [ "$IUI_DONE" -eq 0 ]; do
        if [ "$need_full" -eq 1 ]; then
            iui_measure
            iui_render_frame
            need_full=0
        fi
        iui_read_key
        if [ "$IUI_KEY" = "TICK" ]; then
            iui_advance_eye
            iui_redraw_eyes
            continue
        fi
        iui_handle_key "$IUI_KEY"
        need_full=1
    done
    iui_term_leave
    IUI_POSITION=0
    return "$IUI_RC"
}

iui_selected_names() {
    local i
    for ((i = 0; i < ${#IUI_SKILL_NAMES[@]}; i++)); do
        [ "${IUI_SKILL_SEL[$i]}" -eq 1 ] && printf '%s\n' "${IUI_SKILL_NAMES[$i]}"
    done
    return 0
}

# ─────────────────────────────────────────────────────────────────────────────
# 12. Demo data and CLI
# ─────────────────────────────────────────────────────────────────────────────

# Skills 0 and 4 both require jq, which is what makes the per-tool cache visible:
# reverifying it from either info box refreshes both rows.
iui_load_requirements() {
    iui_req_reset
    iui_req_add 0 jq '*' hard 'plan validation and the permission editors refuse to run'
    iui_req_add 2 memlimit 'Darwin:arm64' soft 'memory cap not enforced; CPU limiting still works'
    iui_req_add 2 cpulimit '*' soft 'CPU cap falls back to nice, which is advisory only'
    iui_req_add 4 jq '*' hard 'the finding tables cannot be assembled'
}

iui_load_demo() {
    IUI_SKILL_NAMES=(planning project-specificies resource-limited-testing brainstorm post-implementation-review install-ui)
    IUI_SKILL_DESCS=(
        'Durable, resumable plans with steps, verification and progress trackers.'
        'Records project conventions, quirks, and deviations worth remembering.'
        'Caps CPU and memory for demanding tool runs so a build cannot wedge the box.'
        'Shapes an idea into a recorded, agreed picture before any plan exists.'
        'After-the-fact review and proposed fixes for code that is already built.'
        'This picker. Listed so the list outgrows a short terminal.'
    )
    IUI_SKILL_INSTALLED=(yes no yes no yes no)
    IUI_SKILL_HAVE=(v25 '' v26 '' v26 '')
    IUI_SKILL_WANT=(v27 v27 v27 v27 v26 v27)
    IUI_SKILL_SEL=(1 0 1 0 0 0)
    IUI_DEP_TOOLS=(); IUI_DEP_STATES=()
    iui_load_requirements
    local i
    for ((i = 0; i < ${#IUI_SKILL_NAMES[@]}; i++)); do
        iui_dep_reverify_skill "$i"
    done
}

iui_usage() {
    sed -n '2,18p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

iui_main() {
    local mode='' width='' height='' cursor='' focus='' color='' glyphs=blocks
    while [ "$#" -gt 0 ]; do
        case "$1" in
            -h|--help) iui_usage; return 0 ;;
            --demo) mode=demo; shift ;;
            --render) mode=render; shift ;;
            --width) [ "$#" -ge 2 ] || return 64; width="$2"; shift 2 ;;
            --height) [ "$#" -ge 2 ] || return 64; height="$2"; shift 2 ;;
            --cursor) [ "$#" -ge 2 ] || return 64; cursor="$2"; shift 2 ;;
            --focus) [ "$#" -ge 2 ] || return 64; focus="$2"; shift 2 ;;
            --eye) [ "$#" -ge 2 ] || return 64; IUI_EYE="$2"; shift 2 ;;
            --color) [ "$#" -ge 2 ] || return 64; color="$2"; shift 2 ;;
            --glyphs) [ "$#" -ge 2 ] || return 64; glyphs="$2"; shift 2 ;;
            *) printf 'install-ui.sh: unknown option: %s\n' "$1" >&2; return 64 ;;
        esac
    done
    [ -n "$mode" ] || { iui_usage >&2; return 64; }
    iui_load_demo
    [ -n "$cursor" ] && IUI_CURSOR="$cursor"
    [ -n "$focus" ] && IUI_FOCUS="$focus"
    [ -n "$color" ] && COLOR_MODE="$color"
    iui_set_glyphs "$glyphs"
    if [ "$mode" = "render" ]; then
        [ -n "$width" ] && IUI_COLS="$width"
        [ -n "$height" ] && IUI_ROWS="$height"
        IUI_POSITION=0
        iui_render_frame
        return 0
    fi
    iui_open_input
    iui_splash_hook
    local rc=0
    iui_run || rc="$?"
    [ "$rc" -eq 0 ] && iui_selected_names
    return "$rc"
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
    export LC_ALL=C
    iui_main "$@"
fi
