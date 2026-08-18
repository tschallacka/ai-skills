#!/usr/bin/env bash
set -euo pipefail

# Interactive installer for the skills in this repository.
# It is intentionally self-contained so it can be used as:
#   curl -fsSL https://raw.githubusercontent.com/tschallacka/ai-skills/main/install.sh | bash

REPO_URL="${AI_SKILLS_REPO_URL:-https://github.com/tschallacka/ai-skills}"
REPO_REF="${AI_SKILLS_REF:-main}"
SOURCE_ROOT=""
TEMP_ROOT=""
SOURCE_VERSION=""
YES=0
SKILL_SELECTION=""
TARGET_SELECTION=""
CLI_MODE=""
CLI_SKILL=""
CLI_FORMAT=""
CLI_RELATIVE=""
CLI_APPROVAL=""

SKILL_NAMES=(planning project-specificies resource-limited-testing brainstorm post-implementation-review)
TARGET_NAMES=(
    "Universal Agent Skills"
    "Codex"
    "Claude Code"
    "OpenCode"
    "OpenClaw"
    "Cline"
)
TARGET_PATHS=(
    "$HOME/.agents/skills"
    "$HOME/.codex/skills"
    "$HOME/.claude/skills"
    "$HOME/.config/opencode/skills"
    "$HOME/.openclaw/skills"
    "$HOME/.cline/skills"
)
CUSTOM_LOCATIONS_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills/custom-locations"

usage() {
    cat <<'EOF'
Usage: install.sh [options]

Interactive by default. Options are useful for automation:
  --all                    Install or update all skills
  --skill <name>           Install or update one skill
  --target <path>          Install into one skill root without prompting
  --yes                    Accept replacements; managed version changes omit backups
  --help                   Show this help

Interactive prompts accept a for "yes to all" (auto-accepts every
remaining confirmation, e.g. replace/backup prompts and permission grants).

Supported skills:
  planning
  project-specificies
  resource-limited-testing
  brainstorm
  post-implementation-review
EOF
}

die() {
    echo "Error: $*" >&2
    exit 1
}

case "${1:-}" in
    --print-skill-files)
        [ "$#" -eq 3 ] || die "--print-skill-files needs skill and --format"
        CLI_MODE="print"
        CLI_SKILL="$2"
        [ "$3" = "--format=tsv" ] || die "--print-skill-files requires --format=tsv"
        CLI_FORMAT="tsv"
        set --
        ;;
    --resolve-source)
        [ "$#" -eq 3 ] || die "--resolve-source needs skill and relative path"
        CLI_MODE="resolve"
        CLI_SKILL="$2"
        CLI_RELATIVE="$3"
        set --
        ;;
    --install-skill)
        [ "$#" -eq 6 ] || die "--install-skill needs skill, --target, and --approval"
        CLI_MODE="install"
        CLI_SKILL="$2"
        [ "$3" = "--target" ] || die "--install-skill requires --target"
        TARGET_SELECTION="$4"
        [ "$5" = "--approval" ] || die "--install-skill requires --approval"
        CLI_APPROVAL="$6"
        set --
        ;;
esac

cleanup() {
    if [ -n "$TEMP_ROOT" ] && [ -d "$TEMP_ROOT" ]; then
        rm -rf "$TEMP_ROOT"
    fi
}
trap cleanup EXIT

while [ "$#" -gt 0 ]; do
    case "$1" in
        --all)
            SKILL_SELECTION="all"
            shift
            ;;
        --skill)
            [ "$#" -ge 2 ] || die "--skill needs a skill name"
            SKILL_SELECTION="$2"
            shift 2
            ;;
        --target)
            [ "$#" -ge 2 ] || die "--target needs a directory"
            TARGET_SELECTION="$2"
            shift 2
            ;;
        --yes)
            YES=1
            shift
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            die "Unknown option: $1 (use --help for usage)"
            ;;
    esac
done

if [ -t 0 ]; then
    exec 3<&0
elif [ "$(ps -p "$$" -o tty= 2>/dev/null | tr -d ' ')" != "?" ] \
    && [ -e /dev/tty ]; then
    exec 3</dev/tty
else
    exec 3<&-
fi

ask() {
    local prompt="$1"
    printf '%s' "$prompt" >&2
    IFS= read -r -u 3 REPLY || die "Interactive input is required"
}

confirm() {
    local prompt="$1"
    if [ "${YES_ALL:-0}" -eq 1 ]; then
        return 0
    fi
    ask "$prompt [y/N/a] "
    case "$REPLY" in
        y|Y|yes|YES) return 0 ;;
        a|A|all|ALL)
            YES_ALL=1
            return 0
            ;;
        *) return 1 ;;
    esac
}

contains() {
    local wanted="$1"
    shift
    local item
    for item in "$@"; do
        [ "$item" = "$wanted" ] && return 0
    done
    return 1
}

# ---------------------------------------------------------------
# Runtime tool verification
# ---------------------------------------------------------------
# Some skills need tools on the target system at runtime (not at install
# time). When one is missing the skill would be installed broken, so abort
# with system-specific install commands for the detected platform.
runtime_requirements() {
    case "$1" in
        planning) printf '%s\n' jq ;;
    esac
}

runtime_tool_install_hint() {
    local tool="$1"
    case "$tool" in
        jq)
            case "$(uname -s)" in
                Darwin)
                    if command -v brew >/dev/null 2>&1; then
                        printf '  brew install jq\n'
                    elif command -v port >/dev/null 2>&1; then
                        printf '  sudo port install jq\n'
                    else
                        printf '  install Homebrew (https://brew.sh) then: brew install jq\n'
                    fi
                    ;;
                Linux)
                    if command -v apt-get >/dev/null 2>&1; then
                        printf '  sudo apt-get install -y jq\n'
                    elif command -v dnf >/dev/null 2>&1; then
                        printf '  sudo dnf install -y jq\n'
                    elif command -v pacman >/dev/null 2>&1; then
                        printf '  sudo pacman -S --noconfirm jq\n'
                    elif command -v zypper >/dev/null 2>&1; then
                        printf '  sudo zypper install -y jq\n'
                    elif command -v apk >/dev/null 2>&1; then
                        printf '  sudo apk add jq\n'
                    elif command -v snap >/dev/null 2>&1; then
                        printf '  sudo snap install jq\n'
                    else
                        printf '  download the static jq binary from https://github.com/jqlang/jq/releases\n'
                    fi
                    ;;
                MINGW*|MSYS*|CYGWIN*)
                    if command -v winget >/dev/null 2>&1; then
                        printf '  winget install jqlang.jq\n'
                    elif command -v choco >/dev/null 2>&1; then
                        printf '  choco install jq\n'
                    elif command -v scoop >/dev/null 2>&1; then
                        printf '  scoop install jq\n'
                    else
                        printf '  download the static jq binary from https://github.com/jqlang/jq/releases\n'
                    fi
                    ;;
                *)
                    printf '  download the static jq binary from https://github.com/jqlang/jq/releases\n'
                    ;;
            esac
            ;;
        *)
            printf '  install %s via your system package manager\n' "$tool"
            ;;
    esac
}

verify_runtime_tools() {
    local skill tool missing=0
    for skill in "$@"; do
        while IFS= read -r tool; do
            [ -n "$tool" ] || continue
            if ! command -v "$tool" >/dev/null 2>&1; then
                [ "$missing" -eq 0 ] && {
                    echo >&2
                    echo "The selected skills need these tools at runtime; they are missing:" >&2
                    missing=1
                }
                echo "  - $tool (required by $skill)" >&2
            fi
        done < <(runtime_requirements "$skill")
    done
    [ "$missing" -eq 0 ] && return 0
    echo >&2
    echo "Install them first, then re-run this installer:" >&2
    for skill in "$@"; do
        while IFS= read -r tool; do
            [ -n "$tool" ] || continue
            command -v "$tool" >/dev/null 2>&1 || {
                echo "  $tool:" >&2
                runtime_tool_install_hint "$tool" >&2
            }
        done < <(runtime_requirements "$skill")
    done
    die "missing runtime tools"
}

agent_target_available() {
    local index="$1"
    local path="${TARGET_PATHS[$index]}"

    case "$index" in
        0) return 0 ;; # Universal Agent Skills has no owning application.
        1) command -v codex >/dev/null 2>&1 || [ -d "$HOME/.codex" ] ;;
        2) command -v claude >/dev/null 2>&1 || [ -d "$HOME/.claude" ] ;;
        3) command -v opencode >/dev/null 2>&1 || [ -d "$HOME/.config/opencode" ] ;;
        4) command -v openclaw >/dev/null 2>&1 || [ -d "$HOME/.openclaw" ] ;;
        5)
            [ -d "$path" ] || [ -d "$HOME/.vscode/extensions/saoudrizwan.claude-dev" ] \
                || compgen -G "$HOME/.vscode/extensions/saoudrizwan.claude-dev-*" >/dev/null 2>&1 \
                || compgen -G "$HOME/.vscode-server/extensions/saoudrizwan.claude-dev-*" >/dev/null 2>&1 \
                || [ -d "${XDG_CONFIG_HOME:-$HOME/.config}/Code/User/globalStorage/saoudrizwan.claude-dev" ] \
                || compgen -G "${XDG_CONFIG_HOME:-$HOME/.config}/Code/User/globalStorage/saoudrizwan.claude-dev*" >/dev/null 2>&1
            ;;
        *) return 1 ;;
    esac
}

save_custom_location() {
    local path="$1"
    mkdir -p "$(dirname "$CUSTOM_LOCATIONS_FILE")"
    touch "$CUSTOM_LOCATIONS_FILE"
    if ! grep -Fqx "$path" "$CUSTOM_LOCATIONS_FILE"; then
        printf '%s\n' "$path" >> "$CUSTOM_LOCATIONS_FILE"
    fi
}

load_custom_locations() {
    SAVED_CUSTOM_LOCATIONS=()
    [ -f "$CUSTOM_LOCATIONS_FILE" ] || return 0
    local path
    while IFS= read -r path; do
        [ -n "$path" ] || continue
        [[ "$path" = \#* ]] && continue
        [ -d "$path" ] || continue
        contains "$path" "${SAVED_CUSTOM_LOCATIONS[@]}" || SAVED_CUSTOM_LOCATIONS+=("$path")
    done < "$CUSTOM_LOCATIONS_FILE"
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

render_art() {
    local offset_x="$1"
    local offset_y="$2"
    local scale="$3"
    local eye_state="$4"
    local row char x y repeat pixel_width blocks
    local -a pixels
    local eye_row eye_row_2

    case "$eye_state" in
        left)
            eye_row='c27f18 c27f18 009c00 009c00 fbfbfb fbfbfb c27417 c27417 df8200 df8200 009c00 009c00 fbfbfb fbfbfb d68601 d68601'
            eye_row_2='c27f18 c27f18 009c00 009c00 fbfbfb fbfbfb c27417 c27417 df8200 df8200 009c00 009c00 fbfbfb fbfbfb d68601 d68601'
            ;;
        right)
            eye_row='c27f18 c27f18 fbfbfb fbfbfb 009c00 009c00 c27417 c27417 df8200 df8200 fbfbfb fbfbfb 009c00 009c00 d68601 d68601'
            eye_row_2='c27f18 c27f18 fbfbfb fbfbfb 009c00 009c00 c27417 c27417 df8200 df8200 fbfbfb fbfbfb 009c00 009c00 d68601 d68601'
            ;;
        *)
            eye_row='c27f18 c27f18 fbfbfb fbfbfb 009c00 009c00 c27417 c27417 df8200 df8200 009c00 009c00 fbfbfb fbfbfb d68601 d68601'
            eye_row_2='c27f18 c27f18 fbfbfb fbfbfb 009c00 009c00 c27417 c27417 df8200 df8200 009c00 009c00 fbfbfb fbfbfb d68601 d68601'
            ;;
    esac

    pixel_width=$((scale * 2))
    blocks=''
    for ((x = 0; x < pixel_width; x++)); do
        blocks="${blocks}█"
    done
    for ((y = 0; y < ${#ART[@]}; y++)); do
        row="${ART[$y]}"
        [ "$y" -eq 8 ] && row="$eye_row"
        [ "$y" -eq 9 ] && row="$eye_row_2"
        IFS=' ' read -r -a pixels <<< "$row"
        for ((repeat = 0; repeat < scale; repeat++)); do
            printf '\033[%d;%dH' "$((offset_y + y * scale + repeat))" "$offset_x"
            for ((x = 0; x < ${#pixels[@]}; x++)); do
                char="${pixels[$x]}"
                color_for "$char"
                if [ -n "$COLOR" ]; then
                    printf '\033[38;2;%sm%s\033[0m' "$COLOR" "$blocks"
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
    local -a labels=(planning project-specificies resource-limited-testing brainstorm post-implementation-review 'all five skills')
    local -a descriptions=(
        'Durable, resumable plans with steps and verification.'
        'Records project conventions, quirks, and deviations.'
        'Caps CPU and memory for demanding tool runs.'
        'Shapes an idea into a recorded, agreed picture before planning.'
        'After-the-fact review and proposed fixes for built code.'
        'Installs or updates the complete skill set.'
    )

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
    printf '\033[%d;%dH\033[1;38;2;255;211;64m╭%s╮\033[0m' "$row" "$MENU_COL" "$horizontal"
    printf '\033[%d;%dH\033[1;38;2;255;211;64m│%*s%s%*s│\033[0m' \
        "$((row + 1))" "$MENU_COL" "$((title_padding / 2))" '' "$title" \
        "$((title_padding - title_padding / 2))" ''
    printf '\033[%d;%dH\033[38;2;255;211;64m╰%s╯\033[0m' "$((row + 2))" "$MENU_COL" "$horizontal"

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
    for ((row = 0; row < 7; row++)); do
        printf '\033[%d;%dH\033[38;2;255;211;64m' "$((base_y + row))" "$base_x"
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

    ART=(
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

select_skills() {
    SELECTED_SKILLS=()

    if [ -z "$SKILL_SELECTION" ]; then
        show_shop_menu
        printf '\033[%d;1H' "$MENU_PROMPT_ROW"
        ask "Choose 1-6 or enter comma-separated names [6]: "
        SKILL_SELECTION="${REPLY:-6}"
    fi

    if [ "$SKILL_SELECTION" = "all" ] || [ "$SKILL_SELECTION" = "6" ]; then
        SELECTED_SKILLS=("${SKILL_NAMES[@]}")
        return
    fi

    local choice name
    IFS=',' read -r -a choices <<< "$SKILL_SELECTION"
    for choice in "${choices[@]}"; do
        name="$choice"
        case "$name" in
            1) name="planning" ;;
            2) name="project-specificies" ;;
            3) name="resource-limited-testing" ;;
            4) name="brainstorm" ;;
            5) name="post-implementation-review" ;;
        esac
        contains "$name" "${SKILL_NAMES[@]}" || die "Unknown skill: $name"
        contains "$name" "${SELECTED_SKILLS[@]}" || SELECTED_SKILLS+=("$name")
    done
}

select_targets() {
    SELECTED_TARGET_PATHS=()
    SELECTED_TARGET_NAMES=()

    if [ -n "$TARGET_SELECTION" ]; then
        SELECTED_TARGET_PATHS=("$TARGET_SELECTION")
        SELECTED_TARGET_NAMES=("$TARGET_SELECTION")
        if ! contains "$TARGET_SELECTION" "${TARGET_PATHS[@]}"; then
            save_custom_location "$TARGET_SELECTION"
        fi
        return
    fi

    AVAILABLE_TARGET_PATHS=()
    AVAILABLE_TARGET_NAMES=()
    local index path choice selection custom_choice
    load_custom_locations

    for index in "${!TARGET_PATHS[@]}"; do
        if agent_target_available "$index"; then
            AVAILABLE_TARGET_PATHS+=("${TARGET_PATHS[$index]}")
            AVAILABLE_TARGET_NAMES+=("${TARGET_NAMES[$index]}")
        fi
    done
    for path in "${SAVED_CUSTOM_LOCATIONS[@]}"; do
        AVAILABLE_TARGET_PATHS+=("$path")
        AVAILABLE_TARGET_NAMES+=("Custom: $path")
    done

    [ "${#AVAILABLE_TARGET_PATHS[@]}" -gt 0 ] || die "No installed agent roots or saved custom locations were found"

    echo >&2
    echo "Install into which skill root?" >&2
    for index in "${!AVAILABLE_TARGET_PATHS[@]}"; do
        path="${AVAILABLE_TARGET_PATHS[$index]}"
        if [ -d "$path" ]; then
            printf '  %d) %s: %s [exists]\n' "$((index + 1))" "${AVAILABLE_TARGET_NAMES[$index]}" "$path" >&2
        else
            printf '  %d) %s: %s [will create]\n' "$((index + 1))" "${AVAILABLE_TARGET_NAMES[$index]}" "$path" >&2
        fi
    done
    custom_choice=$(( ${#AVAILABLE_TARGET_PATHS[@]} + 1 ))
    echo "  $custom_choice) custom directory" >&2
    echo "  a) all listed roots" >&2
    ask "Choose 1-$custom_choice, comma-separated numbers, or a [1]: "
    selection="${REPLY:-1}"

    if [ "$selection" = "a" ] || [ "$selection" = "all" ]; then
        SELECTED_TARGET_PATHS=("${AVAILABLE_TARGET_PATHS[@]}")
        SELECTED_TARGET_NAMES=("${AVAILABLE_TARGET_NAMES[@]}")
        echo "Warning: multiple roots can make the same skill appear more than once." >&2
        return
    fi

    IFS=',' read -r -a choices <<< "$selection"
    for choice in "${choices[@]}"; do
        if [ "$choice" = "$custom_choice" ]; then
            ask "Custom skill root: "
            [ -n "$REPLY" ] || die "A custom directory is required"
            path="${REPLY/#\~/$HOME}"
            case "$path" in
                /*) ;;
                *) die "Custom directory must be an absolute path" ;;
            esac
            if [ ! -d "$path" ]; then
                if ! confirm "$path does not exist. Create it?"; then
                    die "Custom directory does not exist: $path"
                fi
                mkdir -p "$path"
            fi
            save_custom_location "$path"
            SELECTED_TARGET_PATHS+=("$path")
            SELECTED_TARGET_NAMES+=("Custom: $path")
        elif [ "$choice" -ge 1 ] 2>/dev/null && [ "$choice" -le "${#AVAILABLE_TARGET_PATHS[@]}" ]; then
            index=$((choice - 1))
            SELECTED_TARGET_PATHS+=("${AVAILABLE_TARGET_PATHS[$index]}")
            SELECTED_TARGET_NAMES+=("${AVAILABLE_TARGET_NAMES[$index]}")
        else
            die "Unknown target choice: $choice"
        fi
    done
}

download_source() {
    local script_dir ref_prefix archive source_commit source_branch
    if [ -n "${BASH_SOURCE[0]:-}" ] && [ -f "${BASH_SOURCE[0]}" ]; then
        script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
        if [ -f "$script_dir/planning/SKILL.md" ]; then
            SOURCE_ROOT="$script_dir"
            source_commit="$(git -C "$SOURCE_ROOT" rev-parse --short=12 HEAD 2>/dev/null || true)"
            source_branch="$(git -C "$SOURCE_ROOT" symbolic-ref --short -q HEAD 2>/dev/null || true)"
            SOURCE_VERSION="$(git -C "$SOURCE_ROOT" describe --tags --exact-match HEAD 2>/dev/null || true)"
            if [ -n "$SOURCE_VERSION" ]; then
                SOURCE_VERSION="tag:$SOURCE_VERSION commit:${source_commit:-unknown}"
            else
                SOURCE_VERSION="branch:${source_branch:-detached} commit:${source_commit:-unknown}"
            fi
            return
        fi
    fi

    command -v curl >/dev/null 2>&1 || die "curl is required"
    command -v tar >/dev/null 2>&1 || die "tar is required"
    TEMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/ai-skills.XXXXXX")"
    local archive="$TEMP_ROOT/source.tar.gz"
    if [[ "$REPO_REF" =~ ^v?[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        ref_prefix="refs/tags"
        SOURCE_VERSION="tag:$REPO_REF"
        local url="${REPO_URL%/}/archive/${ref_prefix}/${REPO_REF}.tar.gz"
    elif [[ "$REPO_REF" =~ ^[0-9a-fA-F]{7,40}$ ]]; then
        SOURCE_VERSION="commit:$REPO_REF"
        local url="${REPO_URL%/}/archive/${REPO_REF}.tar.gz"
    else
        ref_prefix="refs/heads"
        source_commit=""
        if command -v git >/dev/null 2>&1; then
            source_commit="$(git ls-remote "$REPO_URL" "refs/heads/$REPO_REF" 2>/dev/null | awk 'NR == 1 {print $1}')"
        fi
        SOURCE_VERSION="branch:$REPO_REF commit:${source_commit:-unknown}"
        local url="${REPO_URL%/}/archive/${ref_prefix}/${REPO_REF}.tar.gz"
    fi
    echo "Downloading skills from $url" >&2
    curl -fsSL "$url" -o "$archive"
    tar -xzf "$archive" -C "$TEMP_ROOT"
    SOURCE_ROOT="$(find "$TEMP_ROOT" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
    [ -n "$SOURCE_ROOT" ] && [ -f "$SOURCE_ROOT/planning/SKILL.md" ] \
        || die "Downloaded archive does not contain the expected skills"
}

version_marker_content() {
    printf 'format=ai-skills-version-1\n'
    printf 'source_version=%s\n' "$SOURCE_VERSION"
    printf 'source_ref=%s\n' "$REPO_REF"
}

skill_files() {
    case "$1" in
        planning)
            cat <<'EOF'
SKILL.md
REVIEWER.md
references/plan-read-contract.md
references/ui-user-story-validation.md
references/comment-discipline-contract.md
telemetry-schema.json
placeholders.json
state-change-registry.json
never-executable-extensions.json
context/brainstorm-limiting-context.md
context/brainstorm-limiting-context-contract.json
context/brainstorm-limiting-context-benchmark.json
context/brainstorm-limiting-context-oracle.json
tests/fixtures/planning-context/case-matrix.tsv
tests/fixtures/planning-context/expected-outcomes.jsonl
tests/fixtures/planning-context/test-signing-key.pub
tests/fixtures/planning-context/platform-inputs.tsv
tests/fixtures/planning-context/runner-targets.discovery.txt
tests/fixtures/planning-context/runner-targets.tsv
tests/fixtures/progress-shape/progress.md
tests/fixtures/progress-shape/01-goal-a/goal.md
tests/fixtures/progress-shape/01-goal-a/progress.md
tests/fixtures/progress-shape/01-goal-a/steps/01-step-a.md
tests/fixtures/progress-shape/02-goal-b/goal.md
tests/fixtures/progress-shape/02-goal-b/progress.md
tests/fixtures/progress-shape/02-goal-b/steps/01-step-b.md
tests/fixtures/progress-shape/02-goal-b/steps/02-step-b2.md
tests/fixtures/adversary-probe/FIXTURE-VERSION
tests/fixtures/adversary-probe/README.md
tests/fixtures/adversary-probe/plan-description.md
tests/fixtures/adversary-probe/progress.md
tests/fixtures/adversary-probe/work-unit-inventory.md
tests/fixtures/adversary-probe/adversarial-review.md
tests/fixtures/adversary-probe/01-health-endpoint/goal.md
tests/fixtures/adversary-probe/01-health-endpoint/steps/01-step-add-handler.md
tests/fixtures/adversary-probe/01-health-endpoint/steps/02-step-add-test.md
tests/test-planning-context-contract.sh
tests/test-installer-manifest.sh
tests/test-plan-env.sh
tests/test-plan-integrity-and-monitor.sh
tests/test-reviewer-projection.sh
tests/test-plan-context-reviewer.sh
tests/test-voice-artifact-drift.sh
tests/test-supervision-frame.sh
tests/test-persona-drift.sh
tests/test-progress-bar-shape.sh
tests/test-fix-keys.sh
tests/test-coverage-gaps.sh
tests/test-flag-coverage.sh
PACKAGE-MANIFEST.txt
ROLES.md
MAINTAINER-STYLE-CONTRACT.md
roles/planning.md
roles/execution.md
roles/cleanup.md
roles/VOICES.md
scripts/add-coverage.sh
scripts/add-adversarial-finding.sh
scripts/add-goal.sh
scripts/add-ui-story.sh
scripts/add-work-unit.sh
scripts/configure-ui-story-cache.sh
scripts/create-adversarial-review.sh
scripts/create-plan-progress.sh
scripts/create-plan.sh
scripts/create-progress.sh
scripts/create-step-testing.sh
scripts/rebuild-plan-progress.sh
scripts/register-command.sh
scripts/create-ui-story-run-cache.sh
scripts/create-ui-validation.sh
scripts/create-work-unit-inventory.sh
scripts/plan-content.sh
scripts/plan-context-lib.sh
scripts/plan-context.sh
scripts/plan-context-wrapper.sh
scripts/plan-env.sh
scripts/plan-mutate.sh
scripts/plan-root.sh
scripts/plan-reconcile-lib.sh
scripts/role-context.sh
scripts/monitor-read.sh
scripts/supervision-frame.sh
scripts/update-work-unit.sh
scripts/remove-work-unit.sh
scripts/plan-document-lib.sh
scripts/update-plan-content.sh
scripts/update-adversarial-review.sh
scripts/mint-fix-keys.sh
scripts/verify-fix-keys.sh
scripts/verify-target.sh
scripts/generate-reviewer.sh
scripts/update-plan-progress.sh
scripts/update-progress.sh
scripts/update-step.sh
scripts/validate-plan.sh
scripts/remove-plan.sh
scripts/cleanup-plans.sh
scripts/run-adversary-probe.sh
EOF
            ;;
        project-specificies)
            printf '%s\n' SKILL.md
            ;;
        resource-limited-testing)
            printf '%s\n' SKILL.md
            for file in "$SOURCE_ROOT/resource-limited-testing/scripts/"*.sh; do
                [ -f "$file" ] && printf '%s\n' "scripts/$(basename "$file")"
            done
            ;;
        brainstorm)
            printf '%s\n' SKILL.md
            ;;
        post-implementation-review)
            printf '%s\n' SKILL.md
            ;;
    esac
}

source_file() {
    local skill="$1"
    local relative="$2"
    printf '%s/%s/%s\n' "$SOURCE_ROOT" "$skill" "$relative"
}

cli_print_skill_files() {
    [ "$CLI_SKILL" = "planning" ] || die "unsupported CLI skill: $CLI_SKILL"
    [ "$CLI_FORMAT" = "tsv" ] || die "unsupported CLI format: $CLI_FORMAT"
    cat "$SOURCE_ROOT/planning/PACKAGE-MANIFEST.txt"
}

cli_resolve_source() {
    [ "$CLI_SKILL" = "planning" ] || die "unsupported CLI skill: $CLI_SKILL"
    local source
    source="$(source_file "$CLI_SKILL" "$CLI_RELATIVE")"
    [ -f "$source" ] || die "source does not exist: $CLI_RELATIVE"
    printf '%s\n' "$source"
}

cli_install_skill() {
    contains "$CLI_SKILL" "${SKILL_NAMES[@]}" || die "unsupported CLI skill: $CLI_SKILL"
    verify_runtime_tools "$CLI_SKILL"
    case "$CLI_APPROVAL" in yes|no) ;; *) die "--approval must be yes or no" ;; esac
    local relative source destination_file collision=0 unsafe_collision=0 managed_version_transition=0
    while IFS= read -r relative; do
        [ -n "$relative" ] || continue
        source="$(source_file "$CLI_SKILL" "$relative")"
        [ -f "$source" ] || die "source does not exist: $relative"
        destination_file="$TARGET_SELECTION/$CLI_SKILL/$relative"
        if [ -e "$destination_file" ] || [ -L "$destination_file" ]; then
            printf 'Collision: %s\n' "$destination_file" >&2
            collision=1
            [ -L "$destination_file" ] && unsafe_collision=1
        fi
    done < <(skill_files "$CLI_SKILL")
    if [ -e "$TARGET_SELECTION/$CLI_SKILL/.version" ] || [ -L "$TARGET_SELECTION/$CLI_SKILL/.version" ]; then
        printf 'Collision: %s\n' "$TARGET_SELECTION/$CLI_SKILL/.version" >&2
        collision=1
        [ -L "$TARGET_SELECTION/$CLI_SKILL/.version" ] && unsafe_collision=1
        if [ -f "$TARGET_SELECTION/$CLI_SKILL/.version" ] && ! cmp -s <(version_marker_content) "$TARGET_SELECTION/$CLI_SKILL/.version"; then
            managed_version_transition=1
        fi
    fi
    if [ "$collision" -ne 0 ] && { [ "$managed_version_transition" -eq 0 ] || [ "$unsafe_collision" -ne 0 ]; }; then
        return 3
    fi
    [ "$CLI_APPROVAL" = "yes" ] || { printf 'Approval declined; no files changed.\n' >&2; return 2; }
    while IFS= read -r relative; do
        [ -n "$relative" ] || continue
        source="$(source_file "$CLI_SKILL" "$relative")"
        destination_file="$TARGET_SELECTION/$CLI_SKILL/$relative"
        mkdir -p "$(dirname "$destination_file")"
        cp -p "$source" "$destination_file"
    done < <(skill_files "$CLI_SKILL")
    version_marker_content > "$TARGET_SELECTION/$CLI_SKILL/.version"
    printf 'Installed: %s/%s\n' "$TARGET_SELECTION" "$CLI_SKILL"
}

backup_file() {
    local file="$1"
    local backup="$file.bak"
    local suffix=1
    while [ -e "$backup" ] || [ -L "$backup" ]; do
        backup="$file.bak.$suffix"
        suffix=$((suffix + 1))
    done
    cp -p "$file" "$backup"
    echo "  Backup: $backup" >&2
}

install_skill() {
    local skill="$1"
    local root="$2"
    local destination="$root/$skill"
    local relative source destination_file
    local changed=0
    local missing=0
    local managed_version_transition=0
    local files

    files="$(skill_files "$skill")"
    while IFS= read -r relative; do
        [ -n "$relative" ] || continue
        source="$(source_file "$skill" "$relative")"
        destination_file="$destination/$relative"
        if [ -L "$destination" ] || [ -L "$destination_file" ]; then
            echo "Skipping $root/$skill: existing symlink requires manual review." >&2
            return
        fi
        if [ ! -e "$destination_file" ]; then
            missing=1
        elif ! cmp -s "$source" "$destination_file"; then
            changed=1
        fi
    done <<EOF
$files
EOF

    if [ -L "$destination/.version" ]; then
        echo "Skipping $root/$skill: existing .version symlink requires manual review." >&2
        return
    elif [ ! -e "$destination/.version" ]; then
        missing=1
    elif ! cmp -s <(version_marker_content) "$destination/.version"; then
        changed=1
        managed_version_transition=1
    fi

    if [ "$changed" -eq 1 ]; then
        if [ "$YES" -eq 1 ]; then
            if [ "$managed_version_transition" -eq 1 ]; then
                echo "Version transition detected in $destination; replacing without backups." >&2
            else
                echo "Changes detected in $destination; replacing after backup." >&2
            fi
        elif [ "$managed_version_transition" -eq 1 ]; then
            if ! confirm "Installed version differs in $destination. Replace it without backups?"; then
                echo "Skipped $destination" >&2
                return
            fi
        elif ! confirm "Changes detected in $destination. Replace them and create .bak backups?"; then
            echo "Skipped $destination" >&2
            return
        fi
    elif [ "$missing" -eq 0 ]; then
        echo "Up to date: $destination" >&2
        return
    fi

    mkdir -p "$destination"
    while IFS= read -r relative; do
        [ -n "$relative" ] || continue
        source="$(source_file "$skill" "$relative")"
        destination_file="$destination/$relative"
        if [ -e "$destination_file" ] && ! cmp -s "$source" "$destination_file"; then
            if [ "$managed_version_transition" -eq 0 ]; then
                backup_file "$destination_file"
            fi
        fi
        mkdir -p "$(dirname "$destination_file")"
        cp -p "$source" "$destination_file"
    done <<EOF
$files
EOF
    if [ -e "$destination/.version" ] && ! cmp -s <(version_marker_content) "$destination/.version" && [ "$managed_version_transition" -eq 0 ]; then
        backup_file "$destination/.version"
    fi
    version_marker_content > "$destination/.version"
    echo "Installed: $destination" >&2
}

legacy_plan_migration() {
    local root plan_skill_dir plan_root source_dir plan marker destination state_dir
    plan_skill_dir="${SELECTED_TARGET_PATHS[0]}/planning"
    [ -f "$plan_skill_dir/scripts/plan-document-lib.sh" ] || return 0
    # shellcheck source=/dev/null
    source "$plan_skill_dir/scripts/plan-document-lib.sh"
    plan_root="$(plan_ensure_root_permissions "$(plan_default_root)" "$plan_skill_dir/scripts")"
    state_dir="$plan_root/.migration-state"
    mkdir -p "$state_dir"
    for root in "${SELECTED_TARGET_PATHS[@]}"; do
        source_dir="$root/planning/plans"
        [ -d "$source_dir" ] || continue
        while IFS= read -r -d '' plan; do
            marker="$state_dir/$(printf '%s' "$plan" | cksum | awk '{print $1}')"
            [ -e "${marker}.complete" ] && continue
            destination="$plan_root/$(basename "$plan")"
            if [ -e "$destination" ] || [ -L "$destination" ]; then
                printf 'Plan migration blocked by collision; human review required: %s -> %s\n' "$plan" "$destination" >&2
                printf '%s\n' "$plan" > "${marker}.blocked"
                continue
            fi
            printf 'Migrating plan: %s -> %s\n' "$plan" "$destination" >&2
            printf '%s\n' "$plan" > "${marker}.moving"
            if mv "$plan" "$destination"; then
                rm -f "${marker}.moving"
                printf '%s\n' "$plan" > "${marker}.complete"
            else
                printf 'Plan migration blocked; rerun after fixing permissions: %s\n' "$plan" >&2
                printf '%s\n' "$plan" > "${marker}.blocked"
            fi
        done < <(find "$source_dir" -mindepth 1 -maxdepth 1 -type d -print0 | LC_ALL=C sort -z)
    done
    printf 'Portable plan root ready: %s\n' "$plan_root" >&2
}

ensure_plan_root_after_install() {
    legacy_plan_migration
}

# ---------------------------------------------------------------
# Step 2: planning runtime permissions (interactive main path only)
# ---------------------------------------------------------------
# Grants the user-chosen agents read/write on the plans root and execution
# access to the copied planning shell scripts. Every config file that is
# modified is first backed up as <file>.bak.<timestamp>. Additions are
# idempotent: entries already present are never duplicated.
backup_file_timestamp() {
    local file="$1" stamp backup n=1
    stamp="$(date +%Y%m%dT%H%M%S 2>/dev/null || date +%Y%m%dT%H%M%S)"
    backup="${file}.bak.${stamp}"
    while [ -e "$backup" ]; do
        backup="${file}.bak.${stamp}.${n}"
        n=$((n + 1))
    done
    cp -p "$file" "$backup"
    echo "  Backup: $backup" >&2
}

agent_kind_for_root() {
    case "${1%/}" in
        "$HOME/.claude/skills")          echo claude ;;
        "$HOME/.config/opencode/skills") echo opencode ;;
        "$HOME/.codex/skills")           echo codex ;;
        "$HOME/.openclaw/skills")        echo openclaw ;;
        "$HOME/.cline/skills")           echo cline ;;
        "$HOME/.agents/skills")          echo universal ;;
        *)                               echo custom ;;
    esac
}

claude_permissions() {
    local cfg="${CLAUDE_CONFIGFILE:-$HOME/.claude/settings.json}" scripts="$1" plans="$2" tmp="$3"
    [ -f "$cfg" ] || { echo "  claude-code: no $cfg found; skipped" >&2; return 0; }
    backup_file_timestamp "$cfg"
    PLANS="$plans" SCRIPTS="$scripts" TMPDIR_AGENT="$tmp" python3 - "$cfg" <<'PY'
import json, os, sys
cfg = sys.argv[1]
plans = os.environ["PLANS"].rstrip("/")
scripts = os.environ["SCRIPTS"].rstrip("/")
tmp = os.environ["TMPDIR_AGENT"].rstrip("/")
entries = [
    f"Read({plans}/**)", f"Edit({plans}/**)",
    f"Bash({scripts}/**:*)", f"Read({scripts}/**)",
    f"Bash(bash {scripts}/**:*)", f"Bash(sh {scripts}/**:*)", f"Bash(python3 {scripts}/**:*)",
    f"Read({tmp}/**)", f"Edit({tmp}/**)",
    f"Bash({tmp}/**:*)",
]
data = {}
try:
    data = json.load(open(cfg))
except Exception:
    data = {}
allow = data.setdefault("permissions", {}).setdefault("allow", [])
if not isinstance(allow, list):
    allow = data["permissions"]["allow"] = list(allow) if isinstance(allow, (list, tuple)) else []
added = [e for e in entries if e not in allow]
allow.extend(added)
with open(cfg + ".tmp", "w") as f:
    json.dump(data, f, indent=2, ensure_ascii=False); f.write("\n")
os.replace(cfg + ".tmp", cfg)
if added:
    print("  claude-code: added to permissions.allow:"); [print("    - " + x) for x in added]
else:
    print("  claude-code: permissions already present")
PY
}

opencode_permissions() {
    local cfg="${OPENCODE_CONFIGFILE:-$HOME/.config/opencode/opencode.json}" scripts="$1" plans="$2" tmp="$3"
    [ -f "$cfg" ] || { echo "  opencode: no $cfg found; skipped" >&2; return 0; }
    backup_file_timestamp "$cfg"
    PLANS="$plans" SCRIPTS="$scripts" TMPDIR_AGENT="$tmp" python3 - "$cfg" <<'PY'
import json, os, sys
cfg = sys.argv[1]
plans = os.environ["PLANS"].rstrip("/")
scripts = os.environ["SCRIPTS"].rstrip("/")
tmp = os.environ["TMPDIR_AGENT"].rstrip("/")

# opencode's permission block is keyed by tool name; each value is either an
# action string ("ask"/"allow"/"deny") or a {pattern: action} object.
wanted = {
    "read": [f"{plans}/**", f"{scripts}/**", f"{tmp}/**"],
    "edit": [f"{plans}/**", f"{tmp}/**"],
    "bash": [f"{scripts}/**", f"bash {scripts}/**", f"sh {scripts}/**", f"python3 {scripts}/**", f"{tmp}/**"],
    "external_directory": [f"{plans}/**", f"{scripts}/**", f"{tmp}/**"],
}

try:
    data = json.load(open(cfg))
except Exception:
    data = {}
if not isinstance(data, dict):
    data = {}

perm = data.get("permission")
if isinstance(perm, str):
    # a bare action applied to everything; keep it as the fallback pattern
    perm = {tool: {"*": perm} for tool in wanted}
elif not isinstance(perm, dict):
    perm = {}

# a stray Claude-style allow list is not valid here; migrate it out
legacy = perm.pop("allow", None)
legacy_note = isinstance(legacy, list) and bool(legacy)
perm.pop("deny", None)
perm.pop("ask", None)

added = []
for tool, patterns in wanted.items():
    rule = perm.get(tool)
    if isinstance(rule, str):
        rule = {"*": rule}  # preserve the old blanket action as the fallback
    elif not isinstance(rule, dict):
        rule = {}
    for pattern in patterns:
        if rule.get(pattern) != "allow":
            rule[pattern] = "allow"
            added.append(f"{tool}: {pattern}")
    perm[tool] = rule

data["permission"] = perm
with open(cfg + ".tmp", "w") as f:
    json.dump(data, f, indent=2, ensure_ascii=False); f.write("\n")
os.replace(cfg + ".tmp", cfg)
if legacy_note:
    print("  opencode: removed invalid claude-style permission.allow list")
if added:
    print("  opencode: allowed in permission:"); [print("    - " + x) for x in added]
else:
    print("  opencode: permissions already present")
PY
}

print_manual_permissions() {
    local kind="$1" scripts="$2" plans="$3" tmp="$4"
    echo "  $kind: no safe auto-editable permission file was modified." >&2
    echo "    - grant $kind read/write on $plans" >&2
    echo "    - allow $kind to execute the planning helpers under $scripts" >&2
    echo "    - allow $kind read/write/execute under the planning temp dir $tmp" >&2
    echo "    - example (Claude Code settings.json permissions.allow):" >&2
    echo "        Read($plans/**), Edit($plans/**), Bash($scripts/**:*), Bash(bash $scripts/**:*)" >&2
}

# Fallback because auto-configuration is not always possible or effective
# (e.g. Codex/OpenClaw/Cline have no stable JSON permission key, or an agent
# changed its format). Prints a self-contained prompt the user can paste into
# their AI agent so the agent configures the correct permissions itself.
print_agent_permission_prompt() {
    local plans="$1" tmp="$2"; shift 2
    local root kind
    echo >&2
    echo "If the automatic permission setup above is not effective for your agent," >&2
    echo "paste the following into your AI agent to configure it for the planning" >&2
    echo "skill. It is self-contained and refuses broad permissions:" >&2
    echo >&2
    printf '%s\n' '--- BEGIN AGENT PROMPT (copy from here) ---'
    cat <<PROMPT
Please configure my AI tooling so the "planning" skill can run. Concretely ensure:

1. My plans directory is readable and writable by you and me:
   $plans

2. My planning scratch directory is readable, writable, and executable by you
   (the planning skill writes temporary capsules and run artifacts there):
   $tmp
   Create it if it does not exist (mkdir -p).

3. The planning helper shell scripts under each of the following scripts
   directories may be executed by you; do not block them as unknown or
   suspicious:
PROMPT
    for root in "$@"; do
        kind="$(agent_kind_for_root "$root")"
        printf '   - %s: %s\n' "$kind" "${root%/}/planning/scripts"
    done
    cat <<PROMPT
Find your own permission/settings file (for example: Claude Code
settings.json "permissions.allow", OpenCode opencode.json "permission.allow",
Codex policy/sandbox config, Cline or OpenClaw allowed-tools list) and add
entries that:
  - grant read + write under $plans
  - grant read/write/execute under $tmp
  - allow executing Bash for the planning helper scripts (Read/Edit/Write plus
    Bash rules scoped to those scripts)
Add only entries that are not already present. If you modify a config file,
first copy it to <file>.bak.<timestamp> before editing, then tell me the exact
path and the entries you changed. Do not change any other permissions and do
not grant broad or all-tools access.
--- END AGENT PROMPT (copy from here) ---
PROMPT
}

planning_permission_step() {
    local plans="$HOME/.plans" agent_tmp="${TMPDIR:-/tmp}/planning-agent" root kind scripts
    echo >&2
    echo "== Step 2: planning runtime permissions ==" >&2
    if confirm "Create $plans as the global plans directory?"; then
        mkdir -p "$plans" && echo "  Created $plans" >&2
    fi
    if confirm "Grant the selected agents read/write on $plans and $agent_tmp, and allow them to execute the planning shell scripts? (Each edited config is backed up as .bak.timestamp)"; then
        for root in "${SELECTED_TARGET_PATHS[@]}"; do
            kind="$(agent_kind_for_root "$root")"
            scripts="${root%/}/planning/scripts"
            case "$kind" in
                claude)   claude_permissions "$scripts" "$plans" "$agent_tmp" ;;
                opencode) opencode_permissions "$scripts" "$plans" "$agent_tmp" ;;
                *)        print_manual_permissions "$kind" "$scripts" "$plans" "$agent_tmp" ;;
            esac
        done
    fi
    print_agent_permission_prompt "$plans" "$agent_tmp" "${SELECTED_TARGET_PATHS[@]}"
}

if [ -n "$CLI_MODE" ]; then
    download_source
    case "$CLI_MODE" in
        print) cli_print_skill_files ;;
        resolve) cli_resolve_source ;;
        install) cli_install_skill ;;
    esac
    exit $?
fi

if [ -z "$SKILL_SELECTION" ] || [ -z "$TARGET_SELECTION" ]; then
    show_splash
fi
select_skills
verify_runtime_tools "${SELECTED_SKILLS[@]}"
select_targets
download_source

echo >&2
echo "Selected skills: ${SELECTED_SKILLS[*]}" >&2
echo "Selected roots:  ${SELECTED_TARGET_NAMES[*]}" >&2
echo >&2

for root in "${SELECTED_TARGET_PATHS[@]}"; do
    for skill in "${SELECTED_SKILLS[@]}"; do
        install_skill "$skill" "$root"
    done
done

if contains planning "${SELECTED_SKILLS[@]}"; then
    ensure_plan_root_after_install
    planning_permission_step
fi

echo >&2
echo "Done. Restart the agent CLI if it does not detect the new skills automatically." >&2
