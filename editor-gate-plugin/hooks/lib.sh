#!/usr/bin/env bash
# MODE: PROD
# editor-gate-plugin/hooks/lib.sh -- shared by pre-tool-use-bash.sh and the
# token minting/consuming helpers.
#
# JSON in and out is hand-rolled here (editor_gate_json_field/
# editor_gate_json_escape below), not rjq: B319 -- a hook script runs
# independently of install.sh, potentially long after and on a machine with
# no cargo/dev toolchain, and neither ai-text-editor nor interactive-shell
# ever promised rjq would be on PATH there. Worse for this plugin than for
# tui-hint-plugin: a hard gate that cannot even emit its own deny decision
# risks failing OPEN for exactly the commands it exists to catch.
#
# Extracts the string value of a top-level JSON key from stdin. Not a
# general parser -- exactly as much JSON as a Claude Code PreToolUse payload
# ever needs: find "key", skip to the first quote after it, then copy until
# an unescaped closing quote, unescaping \" \\ \/ \n \t \r as it goes. Prints
# nothing and returns 1 when the key is absent.
editor_gate_json_field() { # <key>, payload on stdin
    awk -v key="$1" '
    { s = s $0 "\n" }
    END {
        needle = "\"" key "\""
        pos = index(s, needle)
        if (pos == 0) { exit 1 }
        pos += length(needle)
        len = length(s)
        while (pos <= len) {
            c = substr(s, pos, 1)
            if (c == ":" || c == " " || c == "\t" || c == "\n" || c == "\r") { pos++; continue }
            break
        }
        if (substr(s, pos, 1) != "\"") { exit 1 }
        pos++
        out = ""
        while (pos <= len) {
            c = substr(s, pos, 1)
            if (c == "\\") {
                nc = substr(s, pos + 1, 1)
                if (nc == "n") out = out "\n"
                else if (nc == "t") out = out "\t"
                else if (nc == "r") out = out "\r"
                else out = out nc
                pos += 2
                continue
            }
            if (c == "\"") break
            out = out c
            pos++
        }
        printf "%s", out
        exit 0
    }'
}

# The inverse: escapes a value for embedding inside a JSON string literal.
# Every dynamic value this plugin writes into a deny reason goes through
# this first -- the stored command a token was minted for, in particular,
# is arbitrary attacker/model-controlled text, and an unescaped quote in it
# could otherwise inject a sibling JSON key (e.g. turn a deny into an
# allow). Backslash first, or escaping the quote would double-escape the
# backslash that operation itself just introduced.
editor_gate_json_escape() { # <value>
    local value="$1"
    value="${value//\\/\\\\}"
    value="${value//\"/\\\"}"
    value="${value//$'\n'/\\n}"
    value="${value//$'\t'/\\t}"
    value="${value//$'\r'/\\r}"
    printf '%s' "$value"
}
#
# gated() flags a Bash command line as an in-place shell edit: `sed -i`,
# `perl -i`, or a heredoc-fed write (any command whose text contains a
# heredoc operator, `<<`, alongside a `>` or `>>` redirect to something other
# than /dev/null -- this covers a python/ruby/node/perl script body that
# opens a path for writing just as well as a plain `cat > file <<EOF`). The
# point, per interactive-shell/../ai-text-editor's own case: `sed -i`
# rewrites and exits 0 whether or not the pattern matched, a heredoc stacks
# the shell's escaping on top of the target file's own syntax, and neither
# verifies what it replaces -- the editor's expected_text refuses on
# mismatch and its journal survives a git checkout that discards a shell
# rewrite.
editor_gate_matches() { # <command line>
    local command="$1"
    printf '%s' "$command" | grep -Eq '(^|[|;&(]|[[:space:]])(sudo[[:space:]]+)?sed([[:space:]][^|;&]*)?[[:space:]]-[a-zA-Z]*i\b' && return 0
    printf '%s' "$command" | grep -Eq '(^|[|;&(]|[[:space:]])(sudo[[:space:]]+)?perl([[:space:]][^|;&]*)?[[:space:]]-[a-zA-Z]*i\b' && return 0
    if printf '%s' "$command" | grep -Eq '<<-?[[:space:]]*['"'"'"]?[A-Za-z_][A-Za-z0-9_]*['"'"'"]?'; then
        printf '%s' "$command" | grep -Eq '>>?[[:space:]]*[^&[:space:]]' \
            && ! printf '%s' "$command" | grep -Eq '>[[:space:]]*/dev/null\b' \
            && return 0
        # A script body's own write call, not a shell redirect at all --
        # python/ruby/node/perl opening a path for writing (open(...,"w"),
        # .write_text(, .writeFileSync(, File.write() -- still a heredoc-fed
        # write this gate exists to catch.
        printf '%s' "$command" | grep -Eq "open\\([^)]*['\"]w['\"]|\\.write_text\\(|\\.writeFileSync\\(|File\\.write\\(" \
            && return 0
    fi
    return 1
}

EDITOR_GATE_STATE="${XDG_STATE_HOME:-$HOME/.local/state}/editor-gate"
EDITOR_GATE_TOKENS="$EDITOR_GATE_STATE/tokens"
EDITOR_GATE_AUDIT="$EDITOR_GATE_STATE/audit.log"
EDITOR_GATE_TTL=120

# Squeezes whitespace the same way on both the minting and the consuming
# side, so a harmless difference in how the model re-quotes the command it
# already declared does not itself cause a mismatch.
editor_gate_normalize() { # <command line>
    printf '%s' "$1" | tr -s '[:space:]' ' ' | sed -e 's/^ //' -e 's/ $//'
}

# Mints a token bound to one exact command, single-use, expiring in
# $EDITOR_GATE_TTL seconds. Prints "EDIT_OK=<token>" to stdout on success.
editor_gate_mint() { # --why <text> --command <text>
    local why='' command='' token now expires normalized
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --why) why="$2"; shift 2 ;;
            --command) command="$2"; shift 2 ;;
            *) printf 'editor-token: unknown argument: %s\n' "$1" >&2; return 64 ;;
        esac
    done
    if [ "${#why}" -lt 15 ]; then
        printf 'editor-token: --why needs a real reason, not a placeholder\n' >&2
        return 64
    fi
    [ -n "$command" ] || { printf 'editor-token: --command is required\n' >&2; return 64; }

    mkdir -p "$EDITOR_GATE_TOKENS" || return 70
    chmod 700 "$EDITOR_GATE_STATE" "$EDITOR_GATE_TOKENS" 2>/dev/null || true
    token="$(head -c16 /dev/urandom | od -An -tx1 | tr -d ' \n')"
    now="$(date +%s)"
    expires=$((now + EDITOR_GATE_TTL))
    normalized="$(editor_gate_normalize "$command")"

    {
        printf '%s\n' "$expires"
        printf '%s\n' "$normalized"
    } > "$EDITOR_GATE_TOKENS/$token"
    chmod 600 "$EDITOR_GATE_TOKENS/$token" 2>/dev/null || true

    printf '%s\t%s\t%s\n' "$now" "$why" "$command" >> "$EDITOR_GATE_AUDIT" 2>/dev/null || true

    printf 'EDIT_OK=%s\n' "$token"
    printf '# expires in %ds, single use, bound to that exact command\n' "$EDITOR_GATE_TTL" >&2
}

# Consumes a token: true (and removes the token file) only when it exists,
# has not expired, and its stored command matches the one presented -- all
# after the same whitespace normalization editor_gate_mint applied.
editor_gate_consume() { # <token> <command line>
    local token="$1" command="$2" file expires stored now normalized
    file="$EDITOR_GATE_TOKENS/$token"
    [ -f "$file" ] || { printf 'unknown token, or already spent\n'; return 1; }
    expires="$(sed -n '1p' "$file")"
    stored="$(sed -n '2p' "$file")"
    now="$(date +%s)"
    if [ -z "$expires" ] || [ "$now" -ge "$expires" ]; then
        rm -f "$file"
        printf 'token expired; mint a fresh one\n'
        return 1
    fi
    normalized="$(editor_gate_normalize "$command")"
    if [ "$normalized" != "$stored" ]; then
        printf 'token was minted for a different command\nIt authorises only:\n    %s\n' "$stored"
        return 1
    fi
    rm -f "$file"
    return 0
}
