#!/usr/bin/env sh
# MODE: PROD
# ---------------------------------------------------------------
# The tiny curl-piped entry point for the Rust installer
# ---------------------------------------------------------------
# This is what `curl -fsSL .../bootstrap.sh | sh` actually runs. It knows
# nothing about skills or manifests -- only "figure out which release asset
# this host needs, download it while the splash plays, then hand off to the
# installer binary that asset carries." A host it does not cover (Windows
# cmd/PowerShell, an offline mirror) downloads the release asset for its
# platform by hand; the bash installer install.sh is retired.
#
# Plain POSIX sh throughout, deliberately: no arrays, no `local`, no `(( ))`
# command form, no `<<<`, no `printf -v`, no bash substring/pattern-
# substitution expansions. `/bin/sh` on a real host is whatever that host
# ships (dash, ash/busybox, or bash in POSIX mode) -- this runs correctly
# under any of them, and under bash too, since POSIX sh is a subset of what
# bash accepts.
#
# It must be entirely self-contained: at the moment this script runs, NOTHING
# else from this repository is on disk yet, so the mascot pixels (ART),
# palette (color_for/fg_sgr/detect_color_mode) and the eye states are written
# out here rather than sourced -- there is no other place a piped script can
# reach before its own payload exists locally.
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
# pinned older version); AI_SKILLS_NO_SPLASH=1
# skips the animation but not the minimum-time wait, so scripted callers see
# the same total time budget a human does. AI_SKILLS_NO_SPLASH=1 additionally
# implies BOOTSTRAP_MIN_SPLASH_SECONDS=0.
BOOTSTRAP_MIN_SPLASH_SECONDS="${BOOTSTRAP_MIN_SPLASH_SECONDS:-4}"
[ "${AI_SKILLS_NO_SPLASH:-0}" != "1" ] || BOOTSTRAP_MIN_SPLASH_SECONDS=0

# The one fixed escape sequence used per-pixel in the hot render loop below;
# computed once here rather than in a subshell per pixel (16 pixels x 16
# rows x several frames a second adds up).
RESET_SGR="$(printf '\033[0m')"

bootstrap_die() {
    printf 'bootstrap: %s\n' "$1" >&2
    exit "${2:-1}"
}

# Resolves the local host to one of the target triples this project ships
# prebuilt binaries for. Command-substitution callers ($(bootstrap_target))
# already run this in a subshell, so its own variables never need to be
# scoped beyond that -- POSIX sh has no `local` at all.
bootstrap_target() {
    _bt_os="$(uname -s 2>/dev/null || echo unknown)"
    _bt_arch="$(uname -m 2>/dev/null || echo unknown)"
    case "$_bt_arch" in
        x86_64 | amd64 | AMD64) _bt_arch=x86_64 ;;
        aarch64 | arm64) _bt_arch=aarch64 ;;
    esac
    case "$_bt_os" in
        Linux)
            case "$_bt_arch" in
                x86_64) printf 'x86_64-unknown-linux-musl\n'; return 0 ;;
                aarch64) printf 'aarch64-unknown-linux-musl\n'; return 0 ;;
            esac
            ;;
        Darwin)
            case "$_bt_arch" in
                x86_64) printf 'x86_64-apple-darwin\n'; return 0 ;;
                aarch64) printf 'aarch64-apple-darwin\n'; return 0 ;;
            esac
            ;;
        MINGW* | MSYS* | CYGWIN* | Windows_NT)
            # Git for Windows' bash (MSYS2/MinGW) and Cygwin.
            case "$_bt_arch" in
                x86_64) printf 'x86_64-pc-windows-msvc\n'; return 0 ;;
            esac
            ;;
    esac
    return 1
}

bootstrap_release_url() {
    _bru_target="$1"
    [ -z "${AI_SKILLS_RELEASE_URL:-}" ] || { printf '%s\n' "$AI_SKILLS_RELEASE_URL"; return 0; }
    printf '%s/releases/latest/download/ai-skills-%s.tar.gz\n' "$REPO_URL" "$_bru_target"
}

# ─────────────────────────────────────────────────────────────────────────────
# The mascot
# ─────────────────────────────────────────────────────────────────────────────

# 16 rows, one pixel-color list per line (space-separated 6-hex-digit RGB).
# A single multi-line single-quoted string, not a bash array: `render_art`
# reads it one line at a time, so nothing here is ever addressed by index.
ART='f2cf38 f2cf38 fdc100 fdc100 fcf246 fcf246 e8b11a e8b11a fcdb28 fcdb28 fcd228 fcd228 fdfd5e fcfd5f fcf347 fcf347
f2cf38 f2cf38 fdc100 fdc100 fcf246 fcf246 e8b11a e8b11a fcdb28 fcdb28 fcd228 fcd228 fbfb5d fdfd5e fcf347 fcf347
e8be38 e8be38 fcd84b fcd84b fdc100 fdc100 fcdb28 fcdb28 fcdb28 fcdb28 e8b11a e8b11a fddc51 fddc51 fdbb37 fdbb37
e8be38 e8be38 fcd84b fcd84b fdc100 fdc100 fcdb28 fcdb28 fcdb28 fcdb28 e8b11a e8b11a fcdc51 fcdc51 fdbb37 fdbb37
c37f18 c37f18 fdc127 fdc127 e6a621 e6a621 fcd22b fcd22b fdc127 fdc127 e6a621 e6a621 e6a621 e6a621 c37f18 c37f18
c37f18 c37f18 fdc127 fdc127 e6a621 e6a621 fcd22b fcd22b fdc127 fdc127 e6a621 e6a621 e6a621 e6a621 c37f18 c37f18
d8a521 d8a521 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 c37f18 c37f18
d8a521 d8a521 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 2d1b00 c37f18 c37f18
c27f18 c27f18 fbfbfb fbfbfb 009c00 009c00 c27417 c27417 dd8100 df8200 009c00 009c00 fbfbfb fbfbfb d68601 d68601
c27f18 c27f18 6c3100 6c3100 c27f18 c27f18 67522d 67522d 67522d 67522d 883300 883300 c27f18 c27f18 6c3100 6c3100
623b00 623b00 321400 321400 3a2910 3a2910 67522d 67522d 67522d 67522d 3a2910 3a2910 6c3100 6c3100 210000 210000
623b00 623b00 321400 321400 3a2910 3a2910 67522d 67522d 67522d 67522d 3a2910 3a2910 6c3100 6c3100 210000 210000
280c02 280c02 300d0a 300d0a 240a00 240a00 67522d 67522d 67522d 67522d 3e0907 3e0907 300d0a 300d0a 210000 210000
280c02 280c02 300d0a 300d0a 240a00 240a00 67522d 67522d 67522d 67522d 3e0907 3e0907 300d0a 300d0a 210000 210000
300d0a 300d0a 280c02 280c02 240a00 240a00 67522d 67522d 67522d 67522d 210000 210000 3e0907 3e0907 240a00 240a00
300d0a 300d0a 280c02 280c02 240a00 240a00 67522d 67522d 67522d 67522d 210000 210000 3e0907 3e0907 240a00 240a00'

detect_color_mode() {
    _dc_colors=0
    if command -v tput >/dev/null 2>&1; then
        _dc_colors="$(tput colors 2>/dev/null || echo 0)"
    fi
    case "$_dc_colors" in
        '' | *[!0-9]*) _dc_colors=0 ;;
    esac
    case "${COLORTERM:-}" in
        truecolor | 24bit)
            COLOR_MODE=truecolor
            return
            ;;
    esac
    if [ "$_dc_colors" -ge 16777216 ]; then
        COLOR_MODE=truecolor
    elif [ "$_dc_colors" -ge 256 ]; then
        COLOR_MODE=256
    elif [ "$_dc_colors" -ge 8 ]; then
        COLOR_MODE=8
    else
        COLOR_MODE=none
    fi
}

# Sets FG_SGR. Reads/sets the shared COLOR_MODE global deliberately -- POSIX
# sh functions share one variable namespace, and that sharing is the return
# channel every helper here uses instead of `local` + an actual return value.
fg_sgr() {
    _fg_rgb="$1"
    _fg_attr="${2:-}"
    [ -n "${COLOR_MODE:-}" ] || detect_color_mode
    case "$COLOR_MODE" in
        truecolor)
            FG_SGR="$(printf '\033[%s38;2;%sm' "$_fg_attr" "$_fg_rgb")"
            return
            ;;
        none)
            FG_SGR=''
            return
            ;;
    esac
    _fg_r="${_fg_rgb%%;*}"
    _fg_rest="${_fg_rgb#*;}"
    _fg_g="${_fg_rest%%;*}"
    _fg_b="${_fg_rest##*;}"
    if [ "$COLOR_MODE" = "256" ]; then
        FG_SGR="$(printf '\033[%s38;5;%dm' "$_fg_attr" \
            "$((16 + 36 * (_fg_r * 5 / 255) + 6 * (_fg_g * 5 / 255) + (_fg_b * 5 / 255)))")"
    else
        FG_SGR="$(printf '\033[%s3%dm' "$_fg_attr" \
            "$(((_fg_r >= 128) + 2 * (_fg_g >= 128) + 4 * (_fg_b >= 128)))")"
    fi
}

# Sets COLOR from a 6-hex-digit pixel like "fdc100" -- fixed-width parameter
# expansion (POSIX, no subprocess), not bash's `${1:0:2}` substring syntax.
# Hex-to-decimal uses the `0x` C-constant prefix arithmetic already
# recognizes, not `16#...` -- that base-N syntax is a bash/ksh extension,
# not POSIX, and dash rejects it outright ("arithmetic expression: expecting
# EOF") -- measured live, once per rendered pixel, during real PTY testing.
color_for() {
    _cf_hex="$1"
    _cf_r="${_cf_hex%????}"
    _cf_rest="${_cf_hex#??}"
    _cf_g="${_cf_rest%??}"
    _cf_b="${_cf_hex#????}"
    COLOR="$((0x$_cf_r));$((0x$_cf_g));$((0x$_cf_b))"
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
# a real verification found U+2588 rendering blank in at least one real
# terminal-emulation stack, so this never risks it either.
#
# Takes the row's own pixel-color STRING directly (not an index into ART --
# there is no array to index into any more): render_art already read that
# line out of ART sequentially, so it hands the content over, not a lookup
# key.
# Builds ROW_PIXELS: one colored, _rrp_scale*2-wide run per pixel in the row,
# or blank padding where the pixel is transparent. Split out of
# render_art_row() below to keep both functions under CODE-STYLE.md's
# 40-line cap.
render_row_pixels() {
    _rrp_row="$1"
    _rrp_scale="$2"
    _rrp_out=''
    _rrp_blocks=''

    _rrp_n=$((_rrp_scale * 2))
    _rrp_i=0
    while [ "$_rrp_i" -lt "$_rrp_n" ]; do
        _rrp_blocks="${_rrp_blocks}#"
        _rrp_i=$((_rrp_i + 1))
    done
    _rrp_pad="$(printf "%${_rrp_n}s" '')"

    # Word-splitting the row into positional parameters (POSIX field
    # splitting on IFS, which is already space here) stands in for bash's
    # `read -a` into an array.
    set -- $_rrp_row
    for _rrp_pixel in "$@"; do
        color_for "$_rrp_pixel"
        if [ -n "$COLOR" ]; then
            fg_sgr "$COLOR"
            _rrp_out="$_rrp_out$FG_SGR$_rrp_blocks$RESET_SGR"
        else
            _rrp_out="$_rrp_out$_rrp_pad"
        fi
    done
    ROW_PIXELS="$_rrp_out"
}

render_art_row() {
    _rar_row="$1"
    _rar_art_y="$2"
    _rar_offset_x="$3"
    _rar_offset_y="$4"
    _rar_scale="$5"
    _rar_eye_state="$6"

    if [ "$_rar_art_y" -eq 8 ] || [ "$_rar_art_y" -eq 9 ]; then
        eye_row_for "$_rar_eye_state"
        _rar_row="$EYE_ROW"
    fi

    render_row_pixels "$_rar_row" "$_rar_scale"

    _rar_i=0
    while [ "$_rar_i" -lt "$_rar_scale" ]; do
        printf '\033[%d;%dH%s' "$((_rar_offset_y + _rar_art_y * _rar_scale + _rar_i))" "$_rar_offset_x" "$ROW_PIXELS"
        _rar_i=$((_rar_i + 1))
    done
}

render_art() {
    _ra_offset_x="$1"
    _ra_offset_y="$2"
    _ra_scale="$3"
    _ra_eye_state="$4"
    _ra_y=0
    printf '%s\n' "$ART" | while IFS= read -r _ra_row; do
        render_art_row "$_ra_row" "$_ra_y" "$_ra_offset_x" "$_ra_offset_y" "$_ra_scale" "$_ra_eye_state"
        _ra_y=$((_ra_y + 1))
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
bootstrap_draw_progress() {
    _bdp_row="$1"
    _bdp_cols="$2"
    _bdp_downloaded="$3"
    _bdp_total="${4:-}"
    _bdp_label_w=14
    _bdp_width=$((_bdp_cols - 3 - _bdp_label_w))
    [ "$_bdp_width" -ge 10 ] || _bdp_width=10
    if [ -n "$_bdp_total" ] && [ "$_bdp_total" -gt 0 ] 2>/dev/null; then
        _bdp_pct=$((_bdp_downloaded * 100 / _bdp_total))
        [ "$_bdp_pct" -le 100 ] || _bdp_pct=100
        _bdp_filled=$((_bdp_width * _bdp_pct / 100))
        _bdp_label="$(printf "%-${_bdp_label_w}s" " $_bdp_pct%")"
    else
        # Unknown Content-Length: an indeterminate marker sweeps the bar
        # instead of a percentage that would just be a guess.
        _bdp_filled=$(( (_bdp_downloaded / 65536) % _bdp_width ))
        _bdp_label="$(printf "%-${_bdp_label_w}s" ' downloading')"
    fi
    _bdp_bar="$(printf "%${_bdp_filled}s" '')"
    _bdp_bar="$(printf '%s' "$_bdp_bar" | tr ' ' '=')"
    _bdp_bar="$(printf '%s%s' "$_bdp_bar" "$(printf "%$((_bdp_width - _bdp_filled))s" '')")"
    printf '\033[%d;1H\033[K [%s]%s' "$_bdp_row" "$_bdp_bar" "$_bdp_label"
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
# A plain space-separated ring, not a bash array: each tick below reads the
# first word off the front and rotates it to the back, which is the same
# round-robin `eye_states[eye_index]; eye_index = (eye_index+1) % N` cycle
# without an array or a modulo at all.
eye_states='front front front front right right front left left front'
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
        current_eye="${eye_states%% *}"
        rest_eye="${eye_states#* }"
        eye_states="$rest_eye $current_eye"
        render_art "$(( (columns - scale * 32) / 2 + 1 ))" 2 "$scale" "$current_eye"
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

# The Windows tarball carries installer.exe; every other target's is plain
# `installer`.
installer_bin=''
for candidate in installer installer.exe; do
    if [ -x "$extract_dir/$candidate" ]; then
        installer_bin="$extract_dir/$candidate"
        break
    fi
done
[ -n "$installer_bin" ] || installer_bin="$(find "$extract_dir" -maxdepth 2 \( -name installer -o -name installer.exe \) -type f -perm -u+x 2>/dev/null | head -1)"
[ -n "$installer_bin" ] && [ -x "$installer_bin" ] \
    || bootstrap_die "the downloaded release has no executable 'installer'"

# The documented one-liner is `curl ... | sh`: the shell running THIS SCRIPT
# then reads it from stdin, not a file, so stdin is the pipe carrying
# bootstrap.sh's own remaining bytes. `exec` inherits that fd as-is into the
# compiled installer -- confirmed for real (B381): its interactive
# root-choice `read` came back with a line of bootstrap.sh's OWN source text
# instead of the keystroke actually typed, because the shell was still
# reading ITS OWN script from that same fd. `sh bootstrap.sh` (downloaded to
# a file first) never hits this: its stdin is the real terminal from the
# start, since the script comes from a path argument, not from stdin.
#
# The redirect belongs on the exec statement itself, not on a bare `exec <
# /dev/tty` line before it: a bare redirect is its own statement, and the
# shell must still read more of ITS OWN script from the (now-redirected) fd
# 0 afterward -- confirmed for real, this starves the shell's own script
# reading and it tries to parse a typed keystroke as the next line of shell
# source. `exec CMD ARGS < /dev/tty`, in contrast, is parsed as a single
# complete statement before any part of it runs, and it is unconditionally
# the last thing this script ever does, so the shell never needs to read its
# own source again after it.
#
# Only redirect when it is actually needed (stdin is not already a tty,
# i.e. it is that pipe) and actually possible. "Possible" has to mean the
# open genuinely succeeds, not just that /dev/tty exists with readable
# permission bits: confirmed for real, a process with no controlling
# terminal at all (this exact case: a Bash tool call with no pty attached)
# still has a /dev/tty device node that `[ -r /dev/tty ]` happily passes,
# and then the real `exec ... < /dev/tty` below dies outright with "cannot
# open /dev/tty: No such device or address" -- worse than the bug this is
# fixing. A no-op redirection actually attempts the open and fails closed
# (silently, stderr discarded) exactly when that would happen, so a
# headless run with no controlling terminal at all is left alone rather
# than crashing on a redirect it can never satisfy.
#
# `true`, not the `:` builtin: confirmed for real, a redirection error on a
# POSIX *special* builtin (`:` is one) terminates a non-interactive shell
# outright, per POSIX itself -- dash does exactly that, so this probe was
# taking down the whole script the instant it had no controlling terminal
# to open, which is the one case it exists to handle gracefully. `true` is
# an ordinary builtin: its own redirection failure is just this command's
# exit status, not a shell-ending event.
use_tty=0
if [ ! -t 0 ] && { true < /dev/tty; } 2>/dev/null; then
    use_tty=1
fi

# With no arguments, default to the interactive skill picker: the compiled
# installer's own argv parsing does not special-case an empty argv (it
# prints --help instead), so bootstrap.sh supplies that default here.
if [ "$#" -eq 0 ]; then
    if [ "$use_tty" -eq 1 ]; then
        exec "$installer_bin" interactive < /dev/tty
    fi
    exec "$installer_bin" interactive
fi
if [ "$use_tty" -eq 1 ]; then
    exec "$installer_bin" "$@" < /dev/tty
fi
exec "$installer_bin" "$@"
