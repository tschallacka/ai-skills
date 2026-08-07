#!/usr/bin/env bash
set -euo pipefail

# Interactive installer for the skills in this repository.
# It is intentionally self-contained so it can be used as:
#   curl -fsSL https://raw.githubusercontent.com/tschallacka/ai-skills/main/install.sh | bash

REPO_URL="${AI_SKILLS_REPO_URL:-https://github.com/tschallacka/ai-skills}"
REPO_REF="${AI_SKILLS_REF:-main}"
SOURCE_ROOT=""
TEMP_ROOT=""
YES=0
SKILL_SELECTION=""
TARGET_SELECTION=""

SKILL_NAMES=(planning project-specificies resource-limited-testing)
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

usage() {
    cat <<'EOF'
Usage: install.sh [options]

Interactive by default. Options are useful for automation:
  --all                    Install or update all skills
  --skill <name>           Install or update one skill
  --target <path>          Install into one skill root without prompting
  --yes                    Accept changed-file replacements after making backups
  --help                   Show this help

Supported skills:
  planning
  project-specificies
  resource-limited-testing
EOF
}

die() {
    echo "Error: $*" >&2
    exit 1
}

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
    ask "$prompt [y/N] "
    case "$REPLY" in
        y|Y|yes|YES) return 0 ;;
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

show_shop_menu() {
    local row=20
    MENU_PROMPT_ROW=$((row + 16))
    printf '\033[2;42H\033[38;2;0;180;70m ▄█▄\033[0m   \033[38;2;0;220;90m ▄█▄\033[0m   \033[38;2;70;255;110m ▄█▄\033[0m'
    printf '\033[3;42H\033[38;2;0;180;70m ███\033[0m   \033[38;2;0;220;90m ███\033[0m   \033[38;2;70;255;110m ███\033[0m'
    printf '\033[4;42H\033[38;2;0;180;70m  ▀\033[0m    \033[38;2;0;220;90m  ▀\033[0m    \033[38;2;70;255;110m  ▀\033[0m'
    printf '\033[%d;1H\033[1;38;2;255;211;64m╭────────────────────────────────────────────────────────────╮\033[0m' "$row"
    printf '\033[%d;1H\033[1;38;2;255;211;64m│\033[0m                  \033[1;38;2;255;211;64mTSCHALLACKA\x27S SKILL SHOP\033[0m                  \033[1;38;2;255;211;64m│\033[0m' "$((row + 1))"
    printf '\033[%d;1H\033[38;2;255;211;64m╰────────────────────────────────────────────────────────────╯\033[0m' "$((row + 2))"
    printf '\033[%d;1H  \033[38;2;0;220;80m◆\033[0m 1) planning' "$((row + 4))"
    printf '\033[%d;1H     Durable, resumable plans with steps and verification.' "$((row + 5))"
    printf '\033[%d;1H  \033[38;2;0;220;80m◆\033[0m 2) project-specificies' "$((row + 7))"
    printf '\033[%d;1H     Records project conventions, quirks, and deviations.' "$((row + 8))"
    printf '\033[%d;1H  \033[38;2;0;220;80m◆\033[0m 3) resource-limited-testing' "$((row + 10))"
    printf '\033[%d;1H     Caps CPU and memory for demanding tool runs.' "$((row + 11))"
    printf '\033[%d;1H  \033[38;2;0;220;80m◆\033[0m 4) all three skills' "$((row + 13))"
    printf '\033[%d;1H     Installs or updates the complete skill set.' "$((row + 14))"
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

    local columns lines scale art_width art_height offset_x offset_y state step anim_scale anim_x anim_y
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
    offset_x=$(( (columns - art_width) / 2 + 1 ))
    offset_y=$(( (lines - art_height) / 2 + 1 ))
    [ "$offset_x" -lt 1 ] && offset_x=1
    [ "$offset_y" -lt 1 ] && offset_y=1

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
        render_art 2 2 1 front
        pixel_message "$step" 2 19
        sleep 0.15
    done

    # Leave the small mascot in the top-left when the menu opens.
    printf '\033[2J\033[H'
    render_art 2 2 1 front
    printf '\033[?25h'
}

select_skills() {
    SELECTED_SKILLS=()

    if [ -z "$SKILL_SELECTION" ]; then
        show_shop_menu
        printf '\033[%d;1H' "$MENU_PROMPT_ROW"
        ask "Choose 1-4 or enter comma-separated names [4]: "
        SKILL_SELECTION="${REPLY:-4}"
    fi

    if [ "$SKILL_SELECTION" = "all" ] || [ "$SKILL_SELECTION" = "4" ]; then
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
        return
    fi

    echo >&2
    echo "Install into which skill root?" >&2
    echo "  1) Universal Agent Skills: ${TARGET_PATHS[0]} (recommended)" >&2
    echo "  2) Codex:                  ${TARGET_PATHS[1]}" >&2
    echo "  3) Claude Code:            ${TARGET_PATHS[2]}" >&2
    echo "  4) OpenCode:               ${TARGET_PATHS[3]}" >&2
    echo "  5) OpenClaw:               ${TARGET_PATHS[4]}" >&2
    echo "  6) Cline:                  ${TARGET_PATHS[5]}" >&2
    echo "  7) all listed roots" >&2
    echo "  8) custom directory" >&2
    ask "Choose 1-8 or comma-separated numbers [1]: "
    local selection="${REPLY:-1}"

    if [ "$selection" = "7" ] || [ "$selection" = "all" ]; then
        SELECTED_TARGET_PATHS=("${TARGET_PATHS[@]}")
        SELECTED_TARGET_NAMES=("${TARGET_NAMES[@]}")
        echo "Warning: multiple roots can make the same skill appear more than once." >&2
        return
    fi

    local choice
    IFS=',' read -r -a choices <<< "$selection"
    for choice in "${choices[@]}"; do
        if [ "$choice" = "8" ]; then
            ask "Custom skill root: "
            [ -n "$REPLY" ] || die "A custom directory is required"
            SELECTED_TARGET_PATHS+=("${REPLY/#\~/$HOME}")
            SELECTED_TARGET_NAMES+=("$REPLY")
        elif [ "$choice" -ge 1 ] 2>/dev/null && [ "$choice" -le 6 ]; then
            local index=$((choice - 1))
            SELECTED_TARGET_PATHS+=("${TARGET_PATHS[$index]}")
            SELECTED_TARGET_NAMES+=("${TARGET_NAMES[$index]}")
        else
            die "Unknown target choice: $choice"
        fi
    done
}

download_source() {
    if [ -n "${BASH_SOURCE[0]:-}" ] && [ -f "${BASH_SOURCE[0]}" ]; then
        local script_dir
        script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
        if [ -f "$script_dir/planning/SKILL.md" ]; then
            SOURCE_ROOT="$script_dir"
            return
        fi
    fi

    command -v curl >/dev/null 2>&1 || die "curl is required"
    command -v tar >/dev/null 2>&1 || die "tar is required"
    TEMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/ai-skills.XXXXXX")"
    local archive="$TEMP_ROOT/source.tar.gz"
    local url="${REPO_URL%/}/archive/refs/heads/${REPO_REF}.tar.gz"
    echo "Downloading skills from $url" >&2
    curl -fsSL "$url" -o "$archive"
    tar -xzf "$archive" -C "$TEMP_ROOT"
    SOURCE_ROOT="$(find "$TEMP_ROOT" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
    [ -n "$SOURCE_ROOT" ] && [ -f "$SOURCE_ROOT/planning/SKILL.md" ] \
        || die "Downloaded archive does not contain the expected skills"
}

skill_files() {
    case "$1" in
        planning)
            printf '%s\n' SKILL.md
            for file in "$SOURCE_ROOT/planning/scripts/"*.sh; do
                [ -f "$file" ] && printf '%s\n' "scripts/$(basename "$file")"
            done
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
    esac
}

source_file() {
    local skill="$1"
    local relative="$2"
    printf '%s/%s/%s\n' "$SOURCE_ROOT" "$skill" "$relative"
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

    if [ "$changed" -eq 1 ]; then
        if [ "$YES" -eq 1 ]; then
            echo "Changes detected in $destination; replacing after backup." >&2
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
            backup_file "$destination_file"
        fi
        mkdir -p "$(dirname "$destination_file")"
        cp -p "$source" "$destination_file"
    done <<EOF
$files
EOF
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

if [ -z "$SKILL_SELECTION" ] || [ -z "$TARGET_SELECTION" ]; then
    show_splash
fi
select_skills
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
fi

echo >&2
echo "Done. Restart the agent CLI if it does not detect the new skills automatically." >&2
