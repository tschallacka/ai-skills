#!/usr/bin/env bash
# MODE: DEV
# ci-subjects.sh — turn ci-scope.sh's crate list into per-subject build flags.
#
# ci-scope.sh answers "which crates does this run have to build". The native
# job is organised by SUBJECT, not by crate: one subject owns a group of crates
# and a verification the group shares (run the binary, upload one named
# artifact). This maps one to the other.
#
# Prints one line per subject, and appends the same to $GITHUB_OUTPUT when set:
#
#   rjq=true|false
#   chat=true|false
#   plan_crypt=true|false
#   planning_commands=true|false
#   editor=true|false
#
# Usage:
#   ci-subjects.sh --scope full|selective|none [--crates "a b c"]
#   ci-subjects.sh --help
#
# THE DEFAULT IS ALWAYS true, for the same reason ci-scope.sh defaults to full:
# a subject wrongly skipped produces a green tick that means "we did not look".
# So scope=full turns everything on, an unrecognised scope turns everything on,
# and only an explicit scope=selective narrows anything. scope=none is the one
# case that turns everything off, and ci-scope.sh emits it only when it has
# established that no crate changed.
#
# PLANNING COMMANDS IS THE CATCH-ALL. It owns every workspace crate that is not
# claimed by a named subject, so a crate added under src/ is covered by default
# rather than silently unbuilt until someone remembers to edit this file. That
# asymmetry is deliberate: the failure mode of the catch-all is a slower run,
# and the failure mode of a whitelist is an untested crate.
#
# Exit codes: 0 always, unless usage is wrong (64).

set -uo pipefail
export LC_ALL=C

scope=""
crates=""

usage() {
    awk 'NR > 1 && /^#/{ sub(/^# ?/, ""); print } /^set -uo/{ exit }' "$0"
    exit "${1:-64}"
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --scope)  [ "$#" -ge 2 ] || usage; scope="$2"; shift 2 ;;
        --crates) [ "$#" -ge 2 ] || usage; crates="$2"; shift 2 ;;
        -h|--help) usage 0 ;;
        *) printf '%s: unknown argument: %s\n' "${0##*/}" "$1" >&2; usage ;;
    esac
done

rjq=false
chat=false
plan_crypt=false
planning_commands=false
editor=false

case "$scope" in
    none)
        : # every subject stays false
        ;;
    selective)
        for crate in $crates; do
            case "$crate" in
                rjq)              rjq=true ;;
                plan-crypt)       plan_crypt=true ;;
                chat-*)           chat=true ;;
                ai-text-editor*)  editor=true ;;
                # Anything else belongs to the planning command registry.
                ?*)               planning_commands=true ;;
            esac
        done
        ;;
    *)
        # full, empty, or anything unrecognised: build everything.
        rjq=true
        chat=true
        plan_crypt=true
        planning_commands=true
        editor=true
        ;;
esac

emit() {
    printf 'rjq=%s\n' "$rjq"
    printf 'chat=%s\n' "$chat"
    printf 'plan_crypt=%s\n' "$plan_crypt"
    printf 'planning_commands=%s\n' "$planning_commands"
    printf 'editor=%s\n' "$editor"
}

emit
if [ -n "${GITHUB_OUTPUT:-}" ]; then
    emit >> "$GITHUB_OUTPUT"
fi
exit 0
