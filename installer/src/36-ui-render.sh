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

# One entry per tick, and a tick is one second: iui_read_first_byte's timeout is
# `read -t 1`, and bash 3.2 -- the macOS floor -- rejects a fractional timeout
# outright, so one second is the shortest frame available. That is why the shake
# at the end is two held beats rather than a flutter.
#
# The dwell at the front is deliberate: the sprite sat still for one second and
# then moved, which reads as a glitch rather than as the start of something. Six
# still frames give the movement a beginning.
IUI_EYE_FRAMES=(
    front front front front front front
    right right
    front
    left left
    front
    left right front
)
IUI_EYE_INDEX=0
# Advances by index, not by searching for the current state. The search version
# matched the FIRST entry equal to IUI_EYE, and `front` appears more than once,
# so the sequence collapsed to front/right/front/right and `left` was never
# reached at all -- measured, not suspected.
iui_advance_eye() {
    IUI_EYE_INDEX=$(( (IUI_EYE_INDEX + 1) % ${#IUI_EYE_FRAMES[@]} ))
    IUI_EYE="${IUI_EYE_FRAMES[$IUI_EYE_INDEX]}"
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
    local scale="$IUI_HEAD_SCALE" col art_y r row top
    if [ "$IUI_HEAD_PLACE" = right-top ]; then
        col=$((IUI_LEFT_W + 3))
        top=3
    else
        col=2
        top=$((3 + IUI_LIST_ROWS + 1))
    fi
    for art_y in 8 9; do
        iui_head_line "$art_y" "$scale" $((scale * 32)) "$IUI_EYE"
        for ((r = 0; r < scale; r++)); do
            row=$((top + art_y * scale + r))
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
    if [ "$focused" -eq 1 ]; then
        iui_seg focus "$text"
    else
        iui_seg dirt "$text"
    fi
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
# The pane's head: name, one-line summary, and the rule above them.
iui_info_head() {
    local width="$1" index="$2" line
    iui_repeat "$IUI_G_RULE" "$width"
    IUI_INFO_TEXT+=("$IUI_REPEAT"); IUI_INFO_ROLE+=(dirt); IUI_INFO_TAG+=(rule)
    iui_info_push gold "${IUI_SKILL_NAMES[$index]}" name "$width"
    iui_wrap "${IUI_SKILL_DESCS[$index]}" "$width"
    for line in "${IUI_WRAP_LINES[@]}"; do
        iui_info_push gold "$line" body "$width"
    done
    iui_info_push stone '' body "$width"
}

# Dependencies and status: the rows a reader decides on, so they come before the
# prose and stay on screen when it scrolls off.
iui_info_status() {
    local width="$1" index="$2" line state up
    iui_info_push diamond 'DEPENDENCIES' body "$width"
    iui_info_requirements "$index" "$width"
    iui_info_push diamond 'STATUS' body "$width"
    iui_skill_state "$index"
    printf -v line '  %-14s %s' 'install' "$(iui_install_verdict)"
    iui_info_push "$(iui_state_role "$IUI_SKILL_STATE")" "$line" body "$width"
    printf -v line '  %-14s %s' 'installed' "${IUI_SKILL_INSTALLED[$index]}"
    iui_info_push stone "$line" body "$width"
    up="$(iui_uptodate_text "$index")"
    printf -v line '  %-14s %s' 'latest release' "$up"
    # Red is for something wrong. An earlier version painted this row redstone
    # whenever the answer was not literally "yes", so a skill that is simply not
    # installed yet read as a warning. An available update is an invitation.
    case "$up" in
        yes) state=diamond ;;
        'no ('*) state=gold ;;
        *) state=stone ;;
    esac
    iui_info_push "$state" "$line" body "$width"
}

# The README-style body: one source paragraph per line, each wrapped. A skill
# with no detail recorded degrades to its summary rather than failing, which is
# what an index-parallel array does when someone extends only one of them.
iui_info_detail() {
    local width="$1" index="$2" paragraph line
    [ "$index" -lt "${#IUI_SKILL_DETAILS[@]}" ] || return 0
    iui_info_push stone '' body "$width"
    while IFS= read -r paragraph; do
        [ -n "$paragraph" ] || continue
        iui_wrap "$paragraph" "$width"
        for line in "${IUI_WRAP_LINES[@]}"; do
            iui_info_push stone "$line" body "$width"
        done
    done <<< "${IUI_SKILL_DETAILS[$index]}"
}

iui_info_lines() {
    local width="$1" index="$IUI_CURSOR"
    IUI_INFO_TEXT=(); IUI_INFO_ROLE=(); IUI_INFO_TAG=()
    iui_info_head "$width" "$index"
    iui_info_status "$width" "$index"
    iui_info_detail "$width" "$index"
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
            "$(runtime_requirement_label "${IUI_REQ_TOOL[$i]}")" "${IUI_REQ_STRENGTH[$i]}" "$IUI_DEP_STATE"
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
# Answers "is a newer release published", by comparing the release this copy was
# installed from against master's package.json. The earlier version compared the
# installed marker against the local checkout's own commit, which under
# `curl … | bash` is not known yet -- hence its "unknown until the source is
# fetched", the commonest answer and the least useful one.
iui_uptodate_text() {
    local index="$1" have
    if [ "${IUI_SKILL_INSTALLED[$index]}" != "yes" ]; then
        printf 'not installed'
        return
    fi
    iui_release_version
    have="${IUI_SKILL_HAVE_PKG[$index]:-}"
    if [ -z "$IUI_RELEASE_VERSION" ]; then
        # Offline, no curl, or a fetch that failed. Never guess a version.
        printf 'unknown (could not reach the release)'
    elif [ -z "$have" ]; then
        printf 'unknown (installed copy records no release)'
    elif [ "$have" = "$IUI_RELEASE_VERSION" ]; then
        printf 'yes'
    else
        printf 'no (%s -> %s)' "$have" "$IUI_RELEASE_VERSION"
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

# What a reader wants from the title is the state of the machine, not the state
# of the checkboxes: how many skills are installed, and how many of those have a
# newer release. "9/9 selected" said only that everything installable was ticked,
# which is the default and so carries no information.
iui_title_bar() {
    local text total="${#IUI_SKILL_NAMES[@]}"
    iui_count_states
    if [ "$IUI_COUNT_UPGRADE" -gt 0 ]; then
        printf -v text ' %s  %d/%d installed  %d upgradeable  %d selected ' \
            'TSCHALLACKA SKILL SHOP' "$IUI_COUNT_INSTALLED" "$total" \
            "$IUI_COUNT_UPGRADE" "$(iui_selected_count)"
    else
        printf -v text ' %s  %d/%d installed  %d selected ' \
            'TSCHALLACKA SKILL SHOP' "$IUI_COUNT_INSTALLED" "$total" \
            "$(iui_selected_count)"
    fi
    iui_pad "$text" "$IUI_COLS"
    iui_seg gold "$IUI_PAD"
    iui_out_line 1 "$IUI_SEG"
}

# Installed, and how many of those are behind the published release. Upgradeable
# is only ever counted when the release is known: offline it stays 0 and the
# title says nothing about upgrades rather than implying everything is current.
iui_count_states() {
    local i
    IUI_COUNT_INSTALLED=0
    IUI_COUNT_UPGRADE=0
    iui_release_version
    for ((i = 0; i < ${#IUI_SKILL_NAMES[@]}; i++)); do
        [ "${IUI_SKILL_INSTALLED[$i]}" = yes ] || continue
        IUI_COUNT_INSTALLED=$((IUI_COUNT_INSTALLED + 1))
        [ -n "$IUI_RELEASE_VERSION" ] || continue
        [ -n "${IUI_SKILL_HAVE_PKG[$i]:-}" ] || continue
        [ "${IUI_SKILL_HAVE_PKG[$i]}" != "$IUI_RELEASE_VERSION" ] || continue
        IUI_COUNT_UPGRADE=$((IUI_COUNT_UPGRADE + 1))
    done
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
    iui_seg dirt "$IUI_G_TJ"; td="$IUI_SEG"
    iui_seg dirt "$IUI_G_TR"; tr="$IUI_SEG"
    iui_out_line 2 "$tl$left$td$right$tr"
}

# The divider gets a junction here too, so the frame closes on the same column
# the body rows divide at rather than running a plain line under it.
iui_bottom_border() {
    local bl bj br left right
    iui_seg dirt "$IUI_G_BL"; bl="$IUI_SEG"
    iui_repeat "$IUI_G_BOT" "$IUI_LEFT_W"
    iui_seg dirt "$IUI_REPEAT"; left="$IUI_SEG"
    iui_seg dirt "$IUI_G_BJ"; bj="$IUI_SEG"
    iui_repeat "$IUI_G_BOT" "$IUI_RIGHT_W"
    iui_seg dirt "$IUI_REPEAT"; right="$IUI_SEG"
    iui_seg dirt "$IUI_G_BR"; br="$IUI_SEG"
    iui_out_line $((IUI_ROWS - 1)) "$bl$left$bj$right$br"
}

# The right cell is the sprite for its first IUI_HEAD_H rows and info text after
# that, so the head and the text share one row budget and neither can overflow.
iui_right_cell() {
    local body="$1" row="$2"
    if [ "$IUI_HEAD_PLACE" = right-top ] && [ "$body" -lt "$IUI_HEAD_H" ]; then
        iui_head_beside_hint "$body"
        return
    fi
    local offset=0
    [ "$IUI_HEAD_PLACE" != right-top ] || offset="$IUI_HEAD_H"
    iui_info_cell $((body - offset + IUI_INFO_SCROLL)) "$row"
}

# One band row: the sprite, then the hint that belongs beside it. The space to
# the right of the sprite was blank before -- on a wide terminal that is most of
# the pane -- so the hints cost no rows that content was using.
iui_head_beside_hint() {
    local body="$1" cols slot glyph_row at text=''
    iui_head_line $((body / IUI_HEAD_SCALE)) "$IUI_HEAD_SCALE" \
        $((IUI_HEAD_SCALE * 32)) "$IUI_EYE"
    cols=$((IUI_RIGHT_W - IUI_HEAD_SCALE * 32))
    slot=$((body / 8))
    glyph_row=$((body % 8))
    if [ "$glyph_row" -lt 7 ]; then
        at=$(( (IUI_HINT_PAGE * IUI_HINT_PER_PAGE + slot) % ${#IUI_HINTS[@]} ))
        text="${IUI_HINTS[$at]}"
        iui_big_line " $text" "$glyph_row" "$cols"
    else
        iui_pad '' "$cols"
        IUI_BIG_LINE="$IUI_PAD"
    fi
    iui_seg gold "$IUI_BIG_LINE"
    IUI_INFO_CELL="$IUI_HEAD_LINE$IUI_SEG"
}

# Hints for the keys a reader would otherwise have to guess at. Shown only on a
# tall terminal, beside nothing and above the sprite, so a small screen keeps the
# sprite alone in its block. Kept to one line each and phrased as actions.
# Kept inside the pane width on purpose: a hint that truncates with a tilde is
# worse than no hint. Installing over an existing copy is how an update happens,
# so the install line says both rather than implying a separate command.
IUI_HINTS=(
    'PRESS A: ALL'
    'PRESS I: INSTALL'
    'PRESS N: NONE'
    'PRESS D: FIX DEPS'
    'TAB: DETAILS'
    'CLICK: TOGGLE'
)
# Two hints fit beside a 16-row sprite at seven pixel rows each plus a gap, so
# the rest cycle. IUI_HINT_DIRTY tells the event loop that a tick changed more
# than the eyes and the whole frame has to be repainted.
IUI_HINT_PER_PAGE=2
IUI_HINT_PAGE=0
IUI_HINT_TICKS=0
IUI_HINT_TICKS_PER_PAGE=4
IUI_HINT_DIRTY=0
iui_advance_hints() {
    [ "$IUI_HINT_ROWS" -gt 0 ] || return 0
    [ "${#IUI_HINTS[@]}" -gt "$IUI_HINT_PER_PAGE" ] || return 0
    IUI_HINT_TICKS=$((IUI_HINT_TICKS + 1))
    [ "$IUI_HINT_TICKS" -ge "$IUI_HINT_TICKS_PER_PAGE" ] || return 0
    IUI_HINT_TICKS=0
    IUI_HINT_PAGE=$((IUI_HINT_PAGE + 1))
    IUI_HINT_DIRTY=1
}
iui_hint_cell() {
    local at="$1" text=''
    [ "$at" -lt "${#IUI_HINTS[@]}" ] && text=" ${IUI_HINTS[$at]}"
    iui_pad "$text" "$IUI_LEFT_W"
    iui_seg slate "$IUI_PAD"
    IUI_LIST_CELL="$IUI_SEG"
}

# The left pane, top to bottom: the skill list, a separator, optional hint rows,
# then the sprite pinned to the bottom in its own block.
iui_left_cell() {
    local body="$1" index hint_at
    if [ "$body" -lt "$IUI_LIST_ROWS" ]; then
        index=$((IUI_SCROLL + body))
        if [ "$index" -lt "${#IUI_SKILL_NAMES[@]}" ]; then
            iui_list_cell "$index" "$IUI_LEFT_W"
        else
            iui_pad '' "$IUI_LEFT_W"; iui_seg stone "$IUI_PAD"; IUI_LIST_CELL="$IUI_SEG"
        fi
        return
    fi
    if [ "$IUI_HEAD_PLACE" != left-bottom ]; then
        iui_pad '' "$IUI_LEFT_W"; iui_seg stone "$IUI_PAD"; IUI_LIST_CELL="$IUI_SEG"
        return
    fi
    if [ "$body" -eq "$IUI_LIST_ROWS" ]; then
        iui_repeat "$IUI_G_RULE" "$IUI_LEFT_W"
        iui_seg dirt "$IUI_REPEAT"; IUI_LIST_CELL="$IUI_SEG"
        return
    fi
    hint_at=$((body - IUI_LIST_ROWS - 1))
    if [ "$hint_at" -lt "$IUI_HINT_ROWS" ]; then
        iui_hint_cell "$hint_at"
        return
    fi
    iui_head_line $(( (body - IUI_LIST_ROWS - 1 - IUI_HINT_ROWS) / IUI_HEAD_SCALE )) \
        "$IUI_HEAD_SCALE" "$IUI_LEFT_W" "$IUI_EYE"
    IUI_LIST_CELL="$IUI_HEAD_LINE"
}

iui_render_wide() {
    local row body index v
    iui_top_border
    iui_seg dirt "$IUI_G_V"; v="$IUI_SEG"
    iui_info_lines "$IUI_RIGHT_W"
    iui_clamp_info_scroll
    for ((body = 0; body < IUI_BODY_ROWS; body++)); do
        row=$((body + 3))
        iui_left_cell "$body"
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

