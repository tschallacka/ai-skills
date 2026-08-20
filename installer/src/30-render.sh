# MODE: DEV
# PACKAGE: DEV
# ---------------------------------------------------------------
# 6. Terminal capability, splash, and menu rendering
# ---------------------------------------------------------------
# Everything here writes absolute cursor positions to stdout and is only reached
# on a real terminal (show_splash returns early unless fd 3 is a tty).
#
# Implicit globals crossing function boundaries in this section — all of them
# deliberate, because bash 3.2 has no way to return a value:
#   COLOR_MODE       set by detect_color_mode, read by fg_sgr
#   FG_SGR           set by fg_sgr, read by its caller on the next line
#   COLOR            set by color_for, read by render_art
#   EYE_ROW          set by eye_row_for, read by render_art and iui_head_line
#   ART              constant (section 1), read by render_art
#   MENU_BOX_WIDTH   set by show_splash (or defaulted by show_shop_menu itself)
#   MENU_COMPACT     set by show_splash, read by show_shop_menu
#   MENU_COL         set by show_shop_menu, read by menu_wrap_text
#   MENU_NEXT_ROW    set by menu_wrap_text, read by show_shop_menu
#   MENU_PROMPT_ROW  set by show_shop_menu, read by select_skills
#   REPLY            set by ask (section 3), read by confirm and every caller
#   YES_ALL          set by confirm on "a", read by every later confirm

# 24-bit SGR is not universal — macOS Terminal.app has never supported it, and
# there the splash used to come out as literal escape residue. Probe once and let
# fg_sgr downgrade to 256-colour, then to the 8 ANSI colours, then to nothing.
detect_color_mode() {
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

# Sets FG_SGR to the best foreground escape this terminal understands for the
# "R;G;B" triple in $1, with the optional SGR attribute prefix $2 (e.g. '1;').
# printf -v avoids a subshell: this runs once per art pixel.
fg_sgr() {
    local rgb="$1" attr="${2:-}" r g b rest
    [ -n "$COLOR_MODE" ] || detect_color_mode
    case "$COLOR_MODE" in
        truecolor)
            printf -v FG_SGR '\033[%s38;2;%sm' "$attr" "$rgb"
            return
            ;;
        none)
            FG_SGR=''
            return
            ;;
    esac
    r="${rgb%%;*}"
    rest="${rgb#*;}"
    g="${rest%%;*}"
    b="${rest##*;}"
    if [ "$COLOR_MODE" = "256" ]; then
        # xterm 6x6x6 colour cube, which starts at index 16.
        printf -v FG_SGR '\033[%s38;5;%dm' "$attr" \
            "$(( 16 + 36 * (r * 5 / 255) + 6 * (g * 5 / 255) + (b * 5 / 255) ))"
    else
        printf -v FG_SGR '\033[%s3%dm' "$attr" \
            "$(( (r >= 128) + 2 * (g >= 128) + 4 * (b >= 128) ))"
    fi
}

color_for() {
    if [ "${#1}" -eq 6 ]; then
        COLOR="$((16#${1:0:2}));$((16#${1:2:2}));$((16#${1:4:2}))"
        return
    fi
    case "$1" in
        y) COLOR='224;183;44' ;;
        Y) COLOR='255;216;58' ;;
        o) COLOR='180;116;18' ;;
        O) COLOR='218;151;27' ;;
        d) COLOR='48;26;4' ;;
        b) COLOR='20;9;1' ;;
        s) COLOR='111;93;55' ;;
        H) COLOR='108;94;62' ;;
        h) COLOR='82;69;42' ;;
        W) COLOR='255;255;255' ;;
        P) COLOR='0;150;40' ;;
        G) COLOR='0;186;60' ;;
        r) COLOR='143;47;5' ;;
        *) COLOR='' ;;
    esac
}

# The pixels of sprite rows 8 and 9 for one eye state, into EYE_ROW. Both rows
# carry the same pixels in every state, so one row per state is enough for both.
# Section 6b's iui_head_line() reads this too, so the eye data exists once.
eye_row_for() {
    case "$1" in
        left)
            EYE_ROW='c27f18 c27f18 009c00 009c00 fbfbfb fbfbfb c27417 c27417 df8200 df8200 009c00 009c00 fbfbfb fbfbfb d68601 d68601'
            ;;
        right)
            EYE_ROW='c27f18 c27f18 fbfbfb fbfbfb 009c00 009c00 c27417 c27417 df8200 df8200 fbfbfb fbfbfb 009c00 009c00 d68601 d68601'
            ;;
        *)
            EYE_ROW='c27f18 c27f18 fbfbfb fbfbfb 009c00 009c00 c27417 c27417 df8200 df8200 009c00 009c00 fbfbfb fbfbfb d68601 d68601'
            ;;
    esac
}

render_art() {
    local offset_x="$1"
    local offset_y="$2"
    local scale="$3"
    local eye_state="$4"
    local row char x y repeat pixel_width blocks
    local -a pixels

    eye_row_for "$eye_state"

    pixel_width=$((scale * 2))
    blocks=''
    for ((x = 0; x < pixel_width; x++)); do
        blocks="${blocks}█"
    done
    for ((y = 0; y < ${#ART[@]}; y++)); do
        row="${ART[$y]}"
        { [ "$y" -eq 8 ] || [ "$y" -eq 9 ]; } && row="$EYE_ROW"
        IFS=' ' read -r -a pixels <<< "$row"
        for ((repeat = 0; repeat < scale; repeat++)); do
            printf '\033[%d;%dH' "$((offset_y + y * scale + repeat))" "$offset_x"
            for ((x = 0; x < ${#pixels[@]}; x++)); do
                char="${pixels[$x]}"
                color_for "$char"
                if [ -n "$COLOR" ]; then
                    fg_sgr "$COLOR"
                    printf '%s%s\033[0m' "$FG_SGR" "$blocks"
                else
                    printf '%*s' "$pixel_width" ''
                fi
            done
        done
    done
}

menu_wrap_text() {
    local row="$1"
    local text="$2"
    local width="$3"
    local indent="$4"
    local line split

    while [ -n "$text" ]; do
        line="$text"
        if [ "${#line}" -gt "$width" ]; then
            line="${text:0:width}"
            split="${line% *}"
            if [ -n "$split" ] && [ "$split" != "$line" ]; then
                line="$split"
            fi
        fi
        printf '\033[%d;%dH%*s%s' "$row" "${MENU_COL:-1}" "$indent" '' "$line"
        row=$((row + 1))
        text="${text:${#line}}"
        text="${text# }"
    done
    MENU_NEXT_ROW="$row"
}

show_shop_menu() {
    local row=2
    local title="TSCHALLACKA'S SKILL SHOP"
    local box_width
    local inner_width title_padding
    local horizontal=''
    local index label description
    # The registry from section 1, plus the synthetic "everything" entry whose
    # number select_skills() treats as "all".
    local -a labels=("${SKILL_NAMES[@]}" 'all five skills')
    local -a descriptions=("${SKILL_DESCRIPTIONS[@]}" 'Installs or updates the complete skill set.')

    [ -n "$COLOR_MODE" ] || detect_color_mode
    if [ -z "${MENU_BOX_WIDTH:-}" ]; then
        local columns="${COLUMNS:-80}"
        if command -v tput >/dev/null 2>&1; then
            columns="$(tput cols 2>/dev/null || echo "$columns")"
        fi
        MENU_BOX_WIDTH=$((columns - 36))
        [ "$MENU_BOX_WIDTH" -gt 60 ] && MENU_BOX_WIDTH=60
        [ "$MENU_BOX_WIDTH" -lt 28 ] && MENU_BOX_WIDTH=28
    fi
    MENU_COL=${MENU_COL:-1}
    MENU_COMPACT=0
    [ "${LINES:-24}" -lt 20 ] && MENU_COMPACT=1
    box_width="$MENU_BOX_WIDTH"
    inner_width=$((box_width - 2))
    title_padding=$((inner_width - ${#title}))

    for ((index = 0; index < inner_width; index++)); do
        horizontal+='─'
    done
    fg_sgr '255;211;64' '1;'
    printf '\033[%d;%dH%s╭%s╮\033[0m' "$row" "$MENU_COL" "$FG_SGR" "$horizontal"
    printf '\033[%d;%dH%s│%*s%s%*s│\033[0m' \
        "$((row + 1))" "$MENU_COL" "$FG_SGR" "$((title_padding / 2))" '' "$title" \
        "$((title_padding - title_padding / 2))" ''
    fg_sgr '255;211;64'
    printf '\033[%d;%dH%s╰%s╯\033[0m' "$((row + 2))" "$MENU_COL" "$FG_SGR" "$horizontal"

    row=$((row + 4))
    for ((index = 0; index < ${#labels[@]}; index++)); do
        label="$((index + 1))) ${labels[$index]}"
        menu_wrap_text "$row" "$label" "$((inner_width - 2))" 2
        row="$MENU_NEXT_ROW"
        if [ "${MENU_COMPACT:-0}" -eq 0 ]; then
            description="${descriptions[$index]}"
            menu_wrap_text "$row" "$description" "$((inner_width - 7))" 5
            row="$MENU_NEXT_ROW"
        fi
        row=$((row + 1))
    done
    MENU_PROMPT_ROW="$row"
}

pixel_message() {
    local frame="$1" base_x="$2" base_y="$3" row col char glyph line
    local -a chars glyph_rows
    chars=(m h h h - m h h h)
    fg_sgr '255;211;64'
    for ((row = 0; row < 7; row++)); do
        printf '\033[%d;%dH%s' "$((base_y + row))" "$base_x" "$FG_SGR"
        for ((col = 0; col < ${#chars[@]}; col++)); do
            if [ "$col" -ge "$frame" ]; then
                printf '          '
                continue
            fi
            char="${chars[$col]}"
            case "$char" in
                m) glyph='10001|11011|10101|10101|10001|10001|10001' ;;
                h) glyph='10001|10001|10001|11111|10001|10001|10001' ;;
                -) glyph='00000|00000|00000|11111|00000|00000|00000' ;;
            esac
            IFS='|' read -r -a glyph_rows <<< "$glyph"
            line="${glyph_rows[$row]}"
            line="${line//1/██}"
            line="${line//0/  }"
            printf '%s  ' "$line"
        done
        printf '\033[0m'
    done
}

show_splash() {
    [ "${AI_SKILLS_NO_SPLASH:-0}" = "1" ] && return
    [ -t 3 ] || return
    [ -n "$COLOR_MODE" ] || detect_color_mode
    # A terminal with no colour at all gets no pixel mascot; the menu still works.
    [ "$COLOR_MODE" = "none" ] && return

    local columns lines scale art_width art_height offset_x offset_y state step anim_scale anim_x anim_y message_y
    columns="${COLUMNS:-80}"
    lines="${LINES:-24}"
    if command -v tput >/dev/null 2>&1; then
        columns="$(tput cols 2>/dev/null || echo "$columns")"
        lines="$(tput lines 2>/dev/null || echo "$lines")"
    fi
    scale=1
    while [ $(( (scale + 1) * 32 )) -le "$columns" ] \
        && [ $(( (scale + 1) * 16 )) -le "$((lines - 5))" ]; do
        scale=$((scale + 1))
    done
    art_width=$((scale * 32))
    art_height=$((scale * 16))
    offset_x=$((columns - art_width - 1))
    offset_y=2
    [ "$offset_x" -lt 1 ] && offset_x=1
    MENU_BOX_WIDTH=$((offset_x - 4))
    [ "$MENU_BOX_WIDTH" -gt 60 ] && MENU_BOX_WIDTH=60
    [ "$MENU_BOX_WIDTH" -lt 28 ] && MENU_BOX_WIDTH=28
    MENU_COMPACT=0
    [ "$lines" -lt 20 ] && MENU_COMPACT=1
    message_y=$((lines - 7))
    [ "$message_y" -lt 1 ] && message_y=1

    printf '\033[?25l\033[2J\033[H'
    for ((anim_scale = 1; anim_scale <= scale; anim_scale++)); do
        printf '\033[2J\033[H'
        anim_x=$(( (columns - anim_scale * 32) / 2 + 1 ))
        anim_y=$(( (lines - anim_scale * 16) / 2 + 1 ))
        [ "$anim_x" -lt 1 ] && anim_x=1
        [ "$anim_y" -lt 1 ] && anim_y=1
        render_art "$anim_x" "$anim_y" "$anim_scale" front
        sleep 0.18
    done

    for state in right left front; do
        printf '\033[2J\033[H'
        render_art "$offset_x" "$offset_y" "$scale" "$state"
        sleep 0.60
    done

    for ((step = 1; step <= 9; step++)); do
        printf '\033[2J\033[H'
        render_art "$offset_x" "$offset_y" 1 front
        pixel_message "$step" 2 "$message_y"
        sleep 0.15
    done

    # Leave the mascot in the right column when the menu opens.
    printf '\033[2J\033[H'
    render_art "$offset_x" "$offset_y" 1 front
    printf '\033[?25h'
}

