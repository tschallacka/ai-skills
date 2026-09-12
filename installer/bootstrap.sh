#!/usr/bin/env bash
# MODE: PROD
# ---------------------------------------------------------------
# The tiny curl-piped entry point for the Rust installer
# ---------------------------------------------------------------
# This is what `curl -fsSL .../bootstrap.sh | bash` actually runs. It knows
# nothing about skills or manifests -- only "figure out which release asset
# this host needs, download it while the splash plays, then hand off to the
# installer binary that asset carries." The full bash installer (install.sh)
# is still the fallback for a host bootstrap.sh does not cover (Windows
# cmd/PowerShell, an offline mirror, or someone who wants no binary at all).
#
# It must be entirely self-contained: at the moment this script runs, NOTHING
# else from this repository is on disk yet, so the mascot pixels (ART),
# palette (color_for/fg_sgr/detect_color_mode) and the eye states
# (eye_row_for) are copied here verbatim from installer/src/05-config.sh and
# installer/src/30-render.sh rather than sourced. Keep the two in sync by
# hand if the sprite or palette ever changes -- there is no third place for
# either to live that both a piped script and the full installer can reach
# before their own payload exists locally.
#
# The animation and the download are deliberately decoupled: the splash has
# its own minimum play time (BOOTSTRAP_MIN_SPLASH_SECONDS) independent of how
# long the download takes. A fast connection still gets the full intro
# rather than a blink-and-it's-gone flash; a slow one keeps the mascot
# animating (not frozen on its last frame) for as long as the download needs.
# The installer only starts once BOTH are done.

set -u

REPO_URL="${AI_SKILLS_REPO_URL:-https://github.com/tschallacka/ai-skills}"
# GitHub's own "latest release" redirect -- no API call, no token, and it
# always resolves to whatever was most recently published. AI_SKILLS_
# RELEASE_URL overrides the whole URL outright (a local test server, a
# pinned older version); AI_SKILLS_NO_SPLASH=1 (install.sh's own flag)
# skips the animation but not the minimum-time wait, so scripted callers see
# the same total time budget a human does. AI_SKILLS_NO_SPLASH=1 additionally
# implies BOOTSTRAP_MIN_SPLASH_SECONDS=0.
BOOTSTRAP_MIN_SPLASH_SECONDS="${BOOTSTRAP_MIN_SPLASH_SECONDS:-4}"
[ "${AI_SKILLS_NO_SPLASH:-0}" != "1" ] || BOOTSTRAP_MIN_SPLASH_SECONDS=0

bootstrap_die() {
    printf 'bootstrap: %s\n' "$1" >&2
    exit "${2:-1}"
}

# Mirrors installer-platform's Target::resolve (src/installer-platform/src/lib.rs)
# and install.sh's normalize_platform, so all three name the same five hosts
# the same way.
bootstrap_target() {
    local os arch
    os="$(uname -s 2>/dev/null || echo unknown)"
    arch="$(uname -m 2>/dev/null || echo unknown)"
    case "$arch" in
        x86_64 | amd64 | AMD64) arch=x86_64 ;;
        aarch64 | arm64) arch=aarch64 ;;
    esac
    case "$os" in
        Linux)
            case "$arch" in
                x86_64) printf 'x86_64-unknown-linux-musl\n'; return 0 ;;
                aarch64) printf 'aarch64-unknown-linux-musl\n'; return 0 ;;
            esac
            ;;
        Darwin)
            case "$arch" in
                x86_64) printf 'x86_64-apple-darwin\n'; return 0 ;;
                aarch64) printf 'aarch64-apple-darwin\n'; return 0 ;;
            esac
            ;;
    esac
    return 1
}

bootstrap_release_url() {
    local target="$1"
    [ -z "${AI_SKILLS_RELEASE_URL:-}" ] || { printf '%s\n' "$AI_SKILLS_RELEASE_URL"; return 0; }
    printf '%s/releases/latest/download/ai-skills-%s.tar.gz\n' "$REPO_URL" "$target"
}

# ─────────────────────────────────────────────────────────────────────────────
# The mascot (copied from installer/src/05-config.sh's ART and
# installer/src/30-render.sh's detect_color_mode/fg_sgr/color_for/
# eye_row_for -- see the file header for why this cannot be sourced instead)
# ─────────────────────────────────────────────────────────────────────────────

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

detect_color_mode() {
    local colors=0
    if command -v tput >/dev/null 2>&1; then
        colors="$(tput colors 2>/dev/null || echo 0)"
    fi
    case "$colors" in
        '' | *[!0-9]*) colors=0 ;;
    esac
    case "${COLORTERM:-}" in
        truecolor | 24bit)
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
        printf -v FG_SGR '\033[%s38;5;%dm' "$attr" \
            "$((16 + 36 * (r * 5 / 255) + 6 * (g * 5 / 255) + (b * 5 / 255)))"
    else
        printf -v FG_SGR '\033[%s3%dm' "$attr" \
            "$(((r >= 128) + 2 * (g >= 128) + 4 * (b >= 128)))"
    fi
}

color_for() {
    COLOR="$((16#${1:0:2}));$((16#${1:2:2}));$((16#${1:4:2}))"
}

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

# One sprite row, ASCII fill glyph ('#', not the Unicode block character):
# ui/mascot.rs's own comment records why -- a real verification of the
# Rust picker's identical sprite found U+2588 rendering blank in at least
# one real terminal-emulation stack, so this never risks it either.
render_art_row() {
    local art_y="$1" offset_x="$2" offset_y="$3" scale="$4" eye_state="$5"
    local row out='' pad blocks='' x i
    row="${ART[$art_y]}"
    if [ "$art_y" -eq 8 ] || [ "$art_y" -eq 9 ]; then
        eye_row_for "$eye_state"
        row="$EYE_ROW"
    fi
    for ((i = 0; i < scale * 2; i++)); do blocks="${blocks}#"; done
    printf -v pad '%*s' $((scale * 2)) ''
    local -a pixels
    IFS=' ' read -r -a pixels <<<"$row"
    for ((x = 0; x < ${#pixels[@]}; x++)); do
        color_for "${pixels[$x]}"
        if [ -n "$COLOR" ]; then
            fg_sgr "$COLOR"
            out="$out$FG_SGR$blocks"$'\033[0m'
        else
            out="$out$pad"
        fi
    done
    for ((i = 0; i < scale; i++)); do
        printf '\033[%d;%dH%s' "$((offset_y + art_y * scale + i))" "$offset_x" "$out"
    done
}

render_art() {
    local offset_x="$1" offset_y="$2" scale="$3" eye_state="$4" y
    for ((y = 0; y < ${#ART[@]}; y++)); do
        render_art_row "$y" "$offset_x" "$offset_y" "$scale" "$eye_state"
    done
}

# ─────────────────────────────────────────────────────────────────────────────
# Download progress
# ─────────────────────────────────────────────────────────────────────────────

# `curl -sI` rather than a HEAD-only client flag, so this works against a
# plain static file server (python3 -m http.server) as much as a CDN.
bootstrap_content_length() {
    curl -fsSL -I "$1" 2>/dev/null \
        | tr -d '\r' \
        | awk 'tolower($1) == "content-length:" { print $2; exit }'
}

bootstrap_bytes_so_far() {
    # `<"$1" 2>/dev/null` cannot swallow the shell's own "No such file"
    # error: the input redirection is set up before the stderr one, so a
    # missing file (curl has not created it yet) prints an error before this
    # function's own caller ever masks it. Caught live: it leaked onto the
    # splash's own progress line on every tick until curl opened the file.
    [ -f "$1" ] || { printf '0'; return 0; }
    wc -c <"$1" 2>/dev/null | tr -d ' '
}

# Fixed-width label (LABEL_W) so " [<bar>]<label>" always comes out to
# exactly `cols` cells -- a line even one cell over wraps onto the next row
# in a real terminal, which corrupts every row this splash owns below it.
# Caught live: a 100-column run wrapped onto 3-4 rows before this fix.
bootstrap_draw_progress() {
    local row="$1" cols="$2" downloaded="$3" total="${4:-}" width bar filled pct label
    local label_w=14
    width=$((cols - 3 - label_w))
    [ "$width" -ge 10 ] || width=10
    if [ -n "$total" ] && [ "$total" -gt 0 ] 2>/dev/null; then
        pct=$((downloaded * 100 / total))
        [ "$pct" -le 100 ] || pct=100
        filled=$((width * pct / 100))
        printf -v label '%-*s' "$label_w" " $pct%"
    else
        # Unknown Content-Length: an indeterminate marker sweeps the bar
        # instead of a percentage that would just be a guess.
        filled=$(((downloaded / 65536) % width))
        printf -v label '%-*s' "$label_w" ' downloading'
    fi
    printf -v bar '%*s' "$filled" ''
    bar="${bar// /=}"
    printf -v bar '%s%*s' "$bar" $((width - filled)) ''
    printf '\033[%d;1H\033[K [%s]%s' "$row" "$bar" "$label"
}

# ─────────────────────────────────────────────────────────────────────────────
# Main
# ─────────────────────────────────────────────────────────────────────────────

command -v curl >/dev/null 2>&1 || bootstrap_die "curl is required; install it and retry" 69
command -v tar >/dev/null 2>&1 || bootstrap_die "tar is required; install it and retry" 69

target="$(bootstrap_target)" \
    || bootstrap_die "unsupported host ($(uname -s 2>/dev/null) $(uname -m 2>/dev/null)); use install.sh instead" 69
url="$(bootstrap_release_url "$target")"

work_dir="$(mktemp -d "${TMPDIR:-/tmp}/ai-skills-bootstrap.XXXXXX")" \
    || bootstrap_die "cannot create a temp directory"
archive="$work_dir/release.tar.gz"
extract_dir="$work_dir/extracted"
trap 'rm -rf "$work_dir"' EXIT

total_bytes="$(bootstrap_content_length "$url")"
case "$total_bytes" in '' | *[!0-9]*) total_bytes='' ;; esac

curl -fsSL -o "$archive" "$url" &
dl_pid=$!

columns="${COLUMNS:-80}"
lines="${LINES:-24}"
if command -v tput >/dev/null 2>&1; then
    columns="$(tput cols 2>/dev/null || echo "$columns")"
    lines="$(tput lines 2>/dev/null || echo "$lines")"
fi
detect_color_mode
show_mascot=0
scale=1
if [ -t 1 ] && [ "$COLOR_MODE" != "none" ] \
    && [ "$columns" -ge 32 ] && [ "$lines" -ge 20 ]; then
    show_mascot=1
    while [ $(( (scale + 1) * 32 )) -le "$columns" ] && [ $(( (scale + 1) * 16 + 4 )) -le "$lines" ]; do
        scale=$((scale + 1))
    done
fi

start_ts="$(date +%s 2>/dev/null || echo 0)"
eye_states=(front front front front right right front left left front)
eye_index=0
progress_row="$lines"
[ "$progress_row" -ge 1 ] || progress_row=1

if [ "$show_mascot" -eq 1 ]; then
    printf '\033[?25l\033[2J\033[H'
fi

download_done=0
min_elapsed=0
while :; do
    if ! kill -0 "$dl_pid" 2>/dev/null; then
        download_done=1
    fi
    now_ts="$(date +%s 2>/dev/null || echo 0)"
    if [ $((now_ts - start_ts)) -ge "$BOOTSTRAP_MIN_SPLASH_SECONDS" ]; then
        min_elapsed=1
    fi
    [ "$download_done" -eq 1 ] && [ "$min_elapsed" -eq 1 ] && break

    if [ "$show_mascot" -eq 1 ]; then
        render_art $(( (columns - scale * 32) / 2 + 1 )) 2 "$scale" "${eye_states[$eye_index]}"
        eye_index=$(((eye_index + 1) % ${#eye_states[@]}))
    fi
    downloaded="$(bootstrap_bytes_so_far "$archive")"
    [ -n "$downloaded" ] || downloaded=0
    bootstrap_draw_progress "$progress_row" "$columns" "$downloaded" "$total_bytes"
    sleep 0.2
done

if [ "$show_mascot" -eq 1 ]; then
    printf '\033[?25h\033[2J\033[H'
fi

if ! wait "$dl_pid"; then
    bootstrap_die "download failed: $url"
fi
[ -s "$archive" ] || bootstrap_die "downloaded file is empty: $url"

mkdir -p "$extract_dir" || bootstrap_die "cannot create $extract_dir"
tar -xzf "$archive" -C "$extract_dir" || bootstrap_die "extracting $archive failed"

installer_bin="$extract_dir/installer"
[ -x "$installer_bin" ] || installer_bin="$(find "$extract_dir" -maxdepth 2 -name installer -type f -perm -u+x 2>/dev/null | head -1)"
[ -n "$installer_bin" ] && [ -x "$installer_bin" ] \
    || bootstrap_die "the downloaded release has no executable 'installer'"

exec "$installer_bin" "$@"
