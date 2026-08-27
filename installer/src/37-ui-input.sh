# MODE: DEV
# PACKAGE: PROD
# ---------------------------------------------------------------
# 6d. Full-screen skill picker: input, the terminal, and the seam
# ---------------------------------------------------------------
# Reads fd 3 — which section 3 opened, so this part must stay below it — runs the
# event loop, and exposes iui_select_skills(), the one function section 7 calls.
#
# Banners in this part, in order:
#   Input — keys and SGR mouse, read from fd 3
#   Terminal enter/leave
#   Event loop
#   The installer seam

# ─────────────────────────────────────────────────────────────────────────────
# Input — keys and SGR mouse, read from fd 3
# ─────────────────────────────────────────────────────────────────────────────

# Keys come from fd 3 (open on /dev/tty, so prompts survive `curl … | bash`) and
# escapes go to stdout. A bare ESC blocks for one more byte; `q` is the quit key.
# The continuation bytes of an escape sequence, with a timeout, because a bare
# Escape has none. Without it `read` blocked forever waiting for a second byte
# that a lone Escape never sends: the picker hung in raw mode on the alternate
# screen with the cursor hidden, so the terminal came back unusable and the only
# way out was to kill the shell.
#
# One second rather than the ~100ms a terminal usually allows: bash 3.2, the
# floor, refuses a fractional `read -t` outright. An arrow key's bytes arrive at
# once and pay nothing; only a lone Escape waits, and it waits once.
iui_read_byte() {
    IUI_BYTE=''
    IFS= read -r -n 1 -t 1 -u 3 IUI_BYTE || return 1
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
# Terminal enter/leave
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
    # The drain and the restore are one block on purpose. Split, the drain could
    # leave the terminal non-canonical with `min 0 time 0` while the restore was
    # skipped because no state had been saved -- the shell then came back with an
    # unusable tty, which is worse than the stray mouse bytes being drained.
    if [ -n "$IUI_STTY_SAVED" ] && [ "${IUI_NO_STTY:-0}" -ne 1 ]; then
        iui_drain_input
        stty "$IUI_STTY_SAVED" <&3 2>/dev/null || true
    fi
}

# Discard input the terminal queued but nobody read -- typically an SGR mouse
# report, which the shell then prints as `[<0;79;20M` at its next prompt.
# Disabling mouse mode does not help: those bytes are already in the tty queue.
#
# Every read here carries a timeout, and that is not belt-and-braces. The first
# version used `stty min 0 time 0` and then an untimed `read -n 256`, reasoning
# that non-canonical mode makes read(2) return immediately. When the stty did not
# take effect the read blocked for 256 bytes that never came -- after the restore
# sequences had already been emitted, so the terminal was left in raw mode with
# no prompt and no way out but killing the shell. A cosmetic tidy-up must never
# be able to do that.
#
# PORTABILITY(read-timeout-floor): the timeout is a whole second because bash 3.2
# refuses a fractional one. One second on exit is acceptable; the loop is gone so
# it is paid at most once.
iui_drain_input() {
    local discard
    [ -n "$IUI_STTY_SAVED" ] || return 0
    stty min 0 time 0 <&3 2>/dev/null || return 0
    IFS= read -r -t 1 -n 256 discard <&3 2>/dev/null || true
    return 0
}

# ─────────────────────────────────────────────────────────────────────────────
# Event loop
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
        IUI_MESSAGE+=("$(runtime_requirement_label "${IUI_REQ_TOOL[$i]}") (${IUI_REQ_STRENGTH[$i]}): ${IUI_REQ_WHY[$i]}")
        # Section 4's generated hint table, so the picker offers the same
        # instruction verify_runtime_tools() prints on the non-interactive path.
        while IFS= read -r line; do
            [ -n "$line" ] || continue
            IUI_MESSAGE+=("$line")
        done < <(runtime_requirement_install_hint "${IUI_REQ_TOOL[$i]}")
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

# Movement follows focus. Previously Up/Down always moved the skill cursor, so
# tabbing to the detail pane highlighted it and then scrolled the wrong thing --
# the detail text below the fold could not be reached at all.
iui_move() {
    local delta="$1" count="${#IUI_SKILL_NAMES[@]}"
    if [ "$IUI_FOCUS" = "info" ]; then
        IUI_INFO_SCROLL=$((IUI_INFO_SCROLL + delta))
        return 0
    fi
    IUI_CURSOR=$((IUI_CURSOR + delta))
    # A new skill has its own detail of its own length, so the old offset would
    # leave the pane scrolled into blank rows.
    IUI_INFO_SCROLL=0
    [ "$count" -ge 0 ] || return 0
}

iui_handle_key() {
    local count="${#IUI_SKILL_NAMES[@]}" i
    case "$1" in
        UP|k) iui_move -1 ;;
        DOWN|j) iui_move 1 ;;
        PGUP) iui_move -"$IUI_BODY_ROWS" ;;
        PGDN) iui_move "$IUI_BODY_ROWS" ;;
        HOME) if [ "$IUI_FOCUS" = "info" ]; then IUI_INFO_SCROLL=0; else IUI_CURSOR=0; IUI_INFO_SCROLL=0; fi ;;
        END) if [ "$IUI_FOCUS" = "info" ]; then IUI_INFO_SCROLL="$IUI_INFO_MAX_SCROLL"; else IUI_CURSOR=$((count - 1)); IUI_INFO_SCROLL=0; fi ;;
        ENTER|SPACE) iui_toggle "$IUI_CURSOR" ;;
        $'\t') if [ "$IUI_FOCUS" = "list" ]; then IUI_FOCUS=info; else IUI_FOCUS=list; fi
              IUI_INFO_SCROLL=0 ;;
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

# Returns 69 without touching the terminal when fd 3 is not a tty, so the caller
# can fall back to the plain menu. An idle tick redraws only the two eye rows; a
# keypress redraws the whole frame. The caller, not this, installs the traps.
iui_run() {
    [ -t 3 ] || return 69
    [ -n "$COLOR_MODE" ] || detect_color_mode
    iui_set_glyphs "$IUI_GLYPHS"
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
            iui_advance_hints
            # A hint page turn changes rows the eye redraw does not touch, so it
            # asks for a full frame instead of leaving half the band stale.
            if [ "$IUI_HINT_DIRTY" -eq 1 ]; then
                IUI_HINT_DIRTY=0
                need_full=1
            else
                iui_redraw_eyes
            fi
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
# The installer seam
# ─────────────────────────────────────────────────────────────────────────────

# The source_version of an already-installed copy of one skill, into IUI_MARKER,
# or '' when no known root holds it. The target root is chosen after the skills
# are, so every candidate root is searched and the first marker found wins.
iui_installed_marker() {
    local skill="$1" root marker
    IUI_MARKER=''
    IUI_MARKER_PACKAGE=''
    for root in ${TARGET_SELECTION:+"$TARGET_SELECTION"} "${TARGET_PATHS[@]}"; do
        marker="$root/$skill/.version"
        [ -f "$marker" ] || continue
        IUI_MARKER="$(awk '/^source_version=/ {
            sub(/^source_version=/, "")
            print
            exit
        }' "$marker")"
        IUI_MARKER_PACKAGE="$(awk '/^package_version=/ {
            sub(/^package_version=/, "")
            print
            exit
        }' "$marker")"
        [ -n "$IUI_MARKER" ] || IUI_MARKER=unknown
        return 0
    done
}

# Section 1's registry into the picker's tables. Everything installable starts
# selected, which is what the numbered menu's default of "all" does; iui_toggle
# refuses a blocked skill, so the preselection cannot include one.
iui_load_installer_skills() {
    local i
    IUI_SKILL_NAMES=("${SKILL_NAMES[@]}")
    IUI_SKILL_DESCS=("${SKILL_DESCRIPTIONS[@]}")
    IUI_SKILL_DETAILS=("${SKILL_DETAILS[@]}")
    IUI_SKILL_INSTALLED=(); IUI_SKILL_HAVE=(); IUI_SKILL_WANT=(); IUI_SKILL_SEL=()
    IUI_SKILL_HAVE_PKG=()
    for ((i = 0; i < ${#IUI_SKILL_NAMES[@]}; i++)); do
        iui_installed_marker "${IUI_SKILL_NAMES[$i]}"
        if [ -n "$IUI_MARKER" ]; then
            IUI_SKILL_INSTALLED+=(yes)
        else
            IUI_SKILL_INSTALLED+=(no)
        fi
        IUI_SKILL_HAVE+=("$IUI_MARKER")
        IUI_SKILL_HAVE_PKG+=("$IUI_MARKER_PACKAGE")
        IUI_SKILL_WANT+=("$SOURCE_VERSION")
        IUI_SKILL_SEL+=(0)
    done
    IUI_DEP_TOOLS=(); IUI_DEP_STATES=()
    iui_load_requirements
    for ((i = 0; i < ${#IUI_SKILL_NAMES[@]}; i++)); do
        iui_dep_reverify_skill "$i"
        iui_toggle "$i"
    done
    IUI_CURSOR=0; IUI_SCROLL=0; IUI_FOCUS=list; IUI_MESSAGE=()
}

# Runs the picker and leaves the answer in SELECTED_SKILLS. Returns 69 when fd 3
# is not a tty, which is select_skills()'s signal to draw the numbered menu, and
# the picker's own code otherwise (130 when the user quit).
#
# Two EXIT traps cannot coexist and cleanup() already owns EXIT, so a second one
# would silently drop the temp-directory removal and the summary. EXIT therefore
# chains both for the lifetime of the picker and is put back afterwards.
iui_select_skills() {
    local rc=0 name
    iui_load_installer_skills
    trap 'iui_term_leave; cleanup' EXIT
    trap 'iui_term_leave; exit 130' INT
    trap 'iui_term_leave; exit 143' TERM
    iui_run || rc="$?"
    trap cleanup EXIT
    trap - INT
    trap - TERM
    [ "$rc" -ne 69 ] || return 69
    # show_splash() drew on the normal screen and iui_term_leave() just restored
    # it, so wipe the mascot before the install log starts writing over it.
    printf '\033[2J\033[H'
    [ "$rc" -eq 0 ] || return "$rc"
    SELECTED_SKILLS=()
    while IFS= read -r name; do
        [ -n "$name" ] || continue
        SELECTED_SKILLS+=("$name")
    done < <(iui_selected_names)
    return 0
}
