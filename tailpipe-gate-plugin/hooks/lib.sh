#!/usr/bin/env bash
# MODE: PROD
# tailpipe-gate-plugin/hooks/lib.sh -- shared by pre-tool-use-bash.sh.
#
# rjq is this repo's real JSON tool. install_shared_rjq (installer/src/
# 20-runtime-tools.sh) copies it to a fixed, install-root-independent path
# as part of installing this plugin -- deliberately not the user's global
# PATH, since neither tailpipe nor ai-text-editor ever promised rjq would
# be there themselves (B319, editor-gate-plugin's own precedent).
tailpipe_gate_rjq_bin() {
    local bin="${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills/bin/rjq"
    if [ -x "$bin" ]; then
        printf '%s\n' "$bin"
        return 0
    fi
    # Never PATH: the shipped rjq lives in the shared bin, and an ambient
    # rjq is whatever the machine happens to have first.
    return 1
}

# Emits a non-blocking PreToolUse additionalContext note through rjq -- the
# safe way to build this JSON, the same reason editor-gate-plugin's own
# emit_context uses it: a hand-rolled printf string breaks the moment the
# note text contains a quote or an apostrophe.
tailpipe_gate_emit_context() { # <rjq_bin> <additionalContext text>
    "$1" -n -c --arg text "$2" \
        '{hookSpecificOutput: {hookEventName: "PreToolUse", permissionDecision: "allow", additionalContext: $text}}'
}

# Resolves this hook's reminder text: the named env var overrides the
# shipped default when the caller SETS it, including setting it to an empty
# string, which suppresses the reminder entirely (the hook still answers
# with {} and never blocks). Unset is the normal case and keeps the default
# verbatim -- the same override knob editor-gate-plugin's own reminders use,
# so a future wording sweep can vary this one the same way.
tailpipe_gate_reminder() { # <env_var_name> <default_text>
    local var_name="$1" default_text="$2"
    if [ "${!var_name+set}" = "set" ]; then
        printf '%s' "${!var_name}"
    else
        printf '%s' "$default_text"
    fi
}

# Flags a Bash command line as "watching an ongoing command's output" --
# tailpipe's own reason to exist. Two independent shapes:
#
#   1. `tail` given a follow-style flag (-f, -F, or --follow, as its own
#      whitespace-delimited token) -- the direct, textbook "live-follow a
#      file or a process's output" case. Matched case-insensitively and
#      terminated on a space/`=`/end/pipe/`;`/`&` rather than a `\b` word
#      boundary, which a flag immediately followed by a digit (`-f2`) or
#      glued to its argument would not cross.
#   2. A backgrounded command (a standalone `&` job-control separator --
#      never `&&`, never the `&` inside a `2>&1`/`>&2`-style redirect, and
#      not required to be the command's last token: `cmd &`, `(cmd &)`,
#      `{ cmd & }`, and `cmd & echo $!` -- capturing the backgrounded job's
#      own pid right after starting it -- all count) that also redirects or
#      pipes its own output somewhere in the line (`>`, `>>`, `2>&1`, or
#      `|`) -- "start something long-running and plan to come back to its
#      log", the shape tailpipe's ingest/read/search/wait directly replace.
#      Not scoped to the specific backgrounded segment -- a redirect
#      anywhere on the line is enough once a standalone `&` is present
#      anywhere else on it. A false positive here is a missed-opportunity
#      reminder, not a wrong action, since this hook only ever adds
#      non-blocking context; that is the tradeoff for not hand-parsing
#      shell structure in a regex.
#
# Deliberately narrower than "any pipe into less/head": that would fire on
# everyday, already-finished-by-the-time-you-read-it commands (`ls | less`)
# that have nothing to do with watching an ongoing process.
tailpipe_gate_matches() { # <command line>
    # grep -c rather than -q: a whole-input reader never exits before
    # printf's single write completes, so the pipeline can't SIGPIPE under
    # this script's `set -o pipefail` (PORTABILITY.md pipefail-grep-q).
    local command="$1"
    printf '%s' "$command" \
        | grep -Eic '(^|[|;&(]|[[:space:]])tail([[:space:]][^|;&]*)?[[:space:]](-[a-z]*f|--follow)([[:space:]]|=|$|[|;&])' >/dev/null \
        && return 0
    if printf '%s' "$command" | grep -Ec '[^&>]&([^&]|$)' >/dev/null; then
        printf '%s' "$command" | grep -Ec '(>>?|2>&1|\|)' >/dev/null && return 0
    fi
    return 1
}
