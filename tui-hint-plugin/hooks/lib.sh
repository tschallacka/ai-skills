#!/usr/bin/env bash
# MODE: PROD
# tui-hint-plugin/hooks/lib.sh -- shared by pre-tool-use.sh.
#
# rjq is this repo's real JSON tool. install_shared_rjq (installer/src/
# 20-runtime-tools.sh) copies it to a fixed, install-root-independent path
# as part of installing this plugin -- deliberately not the user's global
# PATH, since neither ai-text-editor nor interactive-shell (the skills this
# plugin rides with) ever promised rjq would be there themselves.
# tui_hint_rjq_bin resolves that path (B319: install.sh's own PATH-prepend
# does not outlive its process, so a hook running independently later
# cannot assume an ambient `rjq`).
tui_hint_rjq_bin() {
    local bin="${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills/bin/rjq"
    if [ -x "$bin" ]; then
        printf '%s\n' "$bin"
        return 0
    fi
    command -v rjq
}
#
# The program a shell command line runs is not always its first word: a
# leading `VAR=value` assignment, or a leading `sudo`/`env` wrapper, is
# common and would otherwise hide the real target from a naive match.
# Word-split on purpose (not quote-aware) -- this is an advisory hook, never
# blocking, so a wrong guess on an unusual quoting shape costs a missed or
# spurious reminder, not a wrong action.
#
# tui_hint_stripped_command removes only that leading wrapper and returns
# the rest of the line unchanged, for tui_hint_match_profile to test against
# each profile's own ### Invocation patterns (interactive-shell/appprofiles/
# FORMAT.md) -- so a git subcommand, or any other profile whose filename is
# not simply its leading word, is recognized from data the profile itself
# declares rather than from hook-side special-casing.
tui_hint_stripped_command() { # <command line>
    local line="$1" token out=""
    # shellcheck disable=SC2086
    set -- $line
    while [ "$#" -gt 0 ]; do
        token="$1"
        case "$token" in
            *=*) shift ;;
            sudo)
                shift
                while [ "$#" -gt 0 ]; do
                    case "$1" in
                        -u | --user) shift 2 ;;
                        -*) shift ;;
                        *) break ;;
                    esac
                done
                ;;
            env)
                shift
                while [ "$#" -gt 0 ]; do
                    case "$1" in
                        -*) shift ;;
                        *=*) shift ;;
                        *) break ;;
                    esac
                done
                ;;
            *) break ;;
        esac
    done
    for token in "$@"; do
        out="${out:+$out }$token"
    done
    printf '%s\n' "$out"
}

# The leading word of an already-stripped command line, basename-only (a
# path like /usr/bin/mc reads as mc) -- a profile's default, implicit match
# when it declares no ### Invocation section of its own.
tui_hint_first_word() { # <stripped command line>
    local first
    first="${1%% *}"
    printf '%s\n' "${first##*/}"
}

# One extended-regex pattern per line, from a profile's own ### Invocation
# section (FORMAT.md); empty output when the file has no such section. The
# section ends at the next "### " heading or end of file.
tui_hint_profile_invocation_patterns() { # <profile file>
    awk '
        /^### Invocation[ \t]*$/ { in_section = 1; next }
        /^### / { in_section = 0 }
        in_section && NF { print }
    ' "$1" 2>/dev/null
}

# A directory of profile .md files this hook may recommend without an
# explicit trust marker inside each file -- true for the vendor-shipped
# appprofiles/ directory, since the directory itself is the trust boundary
# (it ships with this repo). appprofiles.d/, its agent-writable sibling, is
# not: any process can drop a .md file there, so a file must carry the
# literal marker on its own first line before this hook treats it as a
# profile at all (FORMAT.md). See tui_hint_match_profile's $require_marker.
TUI_HINT_MARKER='<!-- tui-app-profile: v1 -->'
tui_hint_profile_has_marker() { # <profile file>
    local first_line
    IFS= read -r first_line <"$1" 2>/dev/null || return 1
    [ "$first_line" = "$TUI_HINT_MARKER" ]
}

# Scans every profile in $dir that declares its own ### Invocation patterns
# (a filename that is not simply its leading command word -- a git
# subcommand, e.g.) for one whose pattern matches $command. Split out of
# tui_hint_match_profile to keep both under CODE-STYLE.md's 40-line cap.
tui_hint_match_declared_invocation() { # <profiles dir> <stripped command line> <require_marker: 0|1>
    local dir="$1" command="$2" require_marker="$3"
    local file base patterns pattern
    while IFS= read -r file; do
        [ -n "$file" ] || continue
        base="${file##*/}"
        base="${base%.md}"
        [ "$base" != FORMAT ] || continue
        if [ "$require_marker" -eq 1 ] && ! tui_hint_profile_has_marker "$file"; then
            continue
        fi
        patterns="$(tui_hint_profile_invocation_patterns "$file")"
        [ -n "$patterns" ] || continue
        while IFS= read -r pattern; do
            [ -n "$pattern" ] || continue
            # PORTABILITY(pipefail-grep-q): grep -c, not -q, reads the whole
            # line rather than exiting on first match, so this pipe cannot
            # SIGPIPE the writer under `set -o pipefail`.
            if printf '%s' "$command" | grep -Ec -- "$pattern" >/dev/null; then
                printf '%s\n' "$base"
                return 0
            fi
        done <<EOF
$patterns
EOF
    done < <(grep -l -F -- '### Invocation' "$dir"/*.md 2>/dev/null)
    return 1
}

# Finds the profile (by basename, without .md) that a stripped command line
# invokes, searching one directory. When $require_marker is 1, a candidate
# file must carry the literal marker line (appprofiles.d/ is agent-writable
# and so not self-trusting the way the vendor directory is); when 0, the
# directory itself is the trust boundary and no marker is required.
tui_hint_match_profile() { # <profiles dir> <stripped command line> <require_marker: 0|1>
    local dir="$1" command="$2" require_marker="$3"
    local first_word candidate
    [ -d "$dir" ] || return 1

    first_word="$(tui_hint_first_word "$command")"
    [ -n "$first_word" ] || return 1
    candidate="$dir/$first_word.md"
    if [ -f "$candidate" ] && [ "$first_word" != FORMAT ]; then
        if [ "$require_marker" -eq 1 ] && ! tui_hint_profile_has_marker "$candidate"; then
            : # unmarked memory-dir file: not self-trusting, fall through
        elif ! grep -q -F -- '### Invocation' "$candidate"; then
            printf '%s\n' "$first_word"
            return 0
        fi
    fi

    tui_hint_match_declared_invocation "$dir" "$command" "$require_marker"
}
