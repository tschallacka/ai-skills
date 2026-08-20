#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: PROD
# install-ui — standalone entry point for install.sh's full-screen skill picker.
#
# The picker itself is not here: it is installer/src/35-ui-model.sh,
# 36-ui-render.sh and 37-ui-input.sh — sections 6b to 6d of the assembled
# install.sh — and this file sources them, together with the palette and sprite
# primitives of installer/src/05-config.sh and 30-render.sh that they share with
# the splash. So there is exactly one copy of every iui_ function, and this file
# holds only what the shipped installer must not carry: demo data, the argument
# parser, and the headless --render mode the test drives.
#
# Usage:
#   install-ui.sh --demo
#   install-ui.sh --render [--width N] [--height N] [--cursor N]
#                          [--focus list|info] [--eye front|left|right]
#                          [--color truecolor|256|8|none] [--glyphs blocks|ascii]
#   install-ui.sh --help
#
# Exit codes: 0 confirmed, 64 bad usage, 65 install.sh's dependency block is
# empty, 66 install.sh is missing, 69 fd 3 is not a tty (the caller must fall
# back to the plain numeric menu), 130 aborted by the user.

set -euo pipefail

# No `export LC_ALL=C` at file scope: this file is sourceable and must not change
# the caller's locale. It byte-counts only ASCII, so widths are locale-free.

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Every part is required; a missing one is a broken checkout, so no [ -f ] guard.
for iui_part in 05-config 30-render 35-ui-model 36-ui-render 37-ui-input; do
    # shellcheck source=/dev/null
    . "$script_dir/installer/src/$iui_part.sh"
done
unset iui_part

# The dependency tables are generated into install.sh between its marker
# comments and exist nowhere else, so the standalone path reads them from the
# artifact rather than keeping a second copy of the same data.
iui_dev_load_tables() {
    local artifact="$1" block
    if [ ! -f "$artifact" ]; then
        printf 'install-ui.sh: missing %s; run installer/build.sh\n' "$artifact" >&2
        return 66
    fi
    block="$(awk '
        /^# END GENERATED DEPENDENCY BLOCK$/ { inside = 0 }
        inside { print }
        /^# BEGIN GENERATED DEPENDENCY BLOCK$/ { inside = 1 }
    ' "$artifact")"
    if [ -z "$block" ]; then
        printf 'install-ui.sh: %s carries no dependency block; run installer/build.sh\n' \
            "$artifact" >&2
        return 65
    fi
    eval "$block"
}
iui_dev_load_tables "$script_dir/install.sh"

# The trap policy for a standalone run. install.sh does not use this: it already
# owns EXIT for cleanup(), so iui_select_skills() chains the two itself.
iui_install_traps() {
    trap 'iui_term_leave' EXIT
    trap 'iui_term_leave; exit 130' INT
    trap 'iui_term_leave; exit 143' TERM
}

# Only for --demo; the installer opens fd 3 in section 3 and a caller that
# already owns it must not have it reopened underneath.
#
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

# Six skills so the list outgrows a short terminal, and the real requirement
# table on top of them: index 0 is planning and index 4 is
# post-implementation-review, which is what makes the per-tool cache visible.
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
    awk 'NR == 1 { next }
         /^#/ {
             sub(/^#[[:space:]]?/, "")
             if ($0 ~ /^----[[:space:]]*(quoted:|end quoted)/) next
             print; next
         }
         { exit }' "${BASH_SOURCE[0]}"
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
    iui_install_traps
    local rc=0
    iui_run || rc="$?"
    [ "$rc" -eq 0 ] && iui_selected_names
    return "$rc"
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
    export LC_ALL=C
    iui_main "$@"
fi
