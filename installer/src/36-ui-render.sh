# MODE: DEV
# PACKAGE: PROD
# ---------------------------------------------------------------
# 6c. Full-screen skill picker: the sprite and the frame
# ---------------------------------------------------------------
# Draws one frame of the picker from the state 6b holds. The sprite pixels come
# from section 6's ART and eye_row_for(), so this part carries the geometry of
# the head and not a second copy of the head.
#
# Banners in this part, in order:
#   The animated head
#   Frame rendering

# ─────────────────────────────────────────────────────────────────────────────
# The animated head
# ─────────────────────────────────────────────────────────────────────────────

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
    row="${ART[$art_y]}"
    if [ "$art_y" -eq 8 ] || [ "$art_y" -eq 9 ]; then
        eye_row_for "$eye"
        row="$EYE_ROW"
    fi
    iui_repeat "$IUI_G_FILL" $((scale * 2))
    blocks="$IUI_REPEAT"
    IFS=' ' read -r -a pixels <<< "$row"
    printf -v pad '%*s' $((scale * 2)) ''
    for ((x = 0; x < ${#pixels[@]}; x++)); do
        color_for "${pixels[$x]}"
        if [ -n "$COLOR" ]; then
            fg_sgr "$COLOR"
            out="$out$FG_SGR$blocks"$'\033'"[0m"
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
# Frame rendering
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

# The wanted version is SOURCE_VERSION, which download_source() only sets after
# the selection is made, so during the picker it is legitimately empty.
iui_uptodate_text() {
    local index="$1"
    if [ "${IUI_SKILL_INSTALLED[$index]}" != "yes" ]; then
        printf 'not installed'
    elif [ -z "${IUI_SKILL_WANT[$index]}" ]; then
        printf 'unknown until the source is fetched'
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

