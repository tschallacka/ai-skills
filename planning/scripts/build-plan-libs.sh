#!/usr/bin/env bash
# build-plan-libs.sh — compile planning/scripts/lib/<group>/*.sh into the four
# shipped libraries.
#
# One function per file is the maintainable form: a change touches one file, a
# review diff shows one function, and a test can source a single function without
# pulling in the rest. Sourcing 47 files at runtime is not: measured at 2.6x the
# cost of one file, paid on every helper invocation. So the split is the source
# and the concatenation is what ships, the same arrangement installer/build.sh
# uses for install.sh.
#
# Usage:
#   build-plan-libs.sh            # write the libraries
#   build-plan-libs.sh --check    # exit 1 if a committed library is stale
#   build-plan-libs.sh --help
#
# Adding a function: create planning/scripts/lib/<group>/<name>.sh and run this.
# Nothing else. The group directory is the registration, so there is no list to
# forget to update. A file named 00-*.sh sorts first for group state a function
# reads; 99-*.sh sorts last for anything that must run after every definition.
#
# Each source file is standalone: it carries its own shebang so a test can
# source it directly. The compiler strips those shared lines -- shebang, the
# `set` line, and the leading blank run -- so the output declares them once.

set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
lib_root="$script_dir/lib"

usage() {
    cat <<USAGE
Usage: ${0##*/} [--check]
       ${0##*/} --help

  --check   compare the committed libraries against a fresh build and exit 1
            on any difference, naming the group that drifted
USAGE
    exit "${1:-64}"
}

check_only=false
case "${1:-}" in
    --check) check_only=true ;;
    -h|--help) usage 0 ;;
    '') ;;
    *) printf '%s: unknown argument: %s\n' "${0##*/}" "$1" >&2; usage ;;
esac

# The group's own name, its output file, and the one-line purpose that goes in
# the generated header. Adding a group means one row here and one directory.
group_output() {
    case "$1" in
        core)     printf 'plan-core-lib.sh\tfailure, guards, temp files, atomic writes, plan root, snapshots\n' ;;
        document) printf 'plan-document-lib.sh\tsections, paragraphs, titles and fields\n' ;;
        table)    printf 'plan-table-lib.sh\tCSV and Markdown table rendering\n' ;;
        progress) printf 'plan-progress-lib.sh\tprogress arithmetic and the status glyphs\n' ;;
        *) return 1 ;;
    esac
}

# Emitted once per library rather than once per source file.
emit_library() { # <group>
    local group="$1" spec output purpose member first=1
    spec="$(group_output "$group")" || {
        printf '%s: unknown group: %s\n' "${0##*/}" "$group" >&2
        exit 65
    }
    output="${spec%%	*}"
    purpose="${spec#*	}"

    printf '#!/usr/bin/env bash\n'
    printf '# GENERATED FILE — do not edit. Compiled from scripts/lib/%s/*.sh by:\n' "$group"
    printf '#   planning/scripts/build-plan-libs.sh\n'
    printf '# Edit the function file in that directory, then re-run the build.\n'
    printf '#\n'
    printf '# %s\n' "$purpose"
    printf '\n'
    printf 'set -euo pipefail\n'
    # Idempotent: the façade sources its siblings, and a script may source both,
    # which would otherwise re-run 00-state.sh and drop the registered temp
    # files. `return` is legal here because the file is only ever sourced.
    printf '\n'
    printf '[ -z "${PLAN_%s_LIB_LOADED:-}" ] || return 0\n' "$(printf '%s' "$group" | tr '[:lower:]' '[:upper:]')"
    printf 'PLAN_%s_LIB_LOADED=1\n' "$(printf '%s' "$group" | tr '[:lower:]' '[:upper:]')"

    for member in "$lib_root/$group"/*.sh; do
        [ -f "$member" ] || continue
        printf '\n'
        # Strip the standalone shebang and any `set` line: the header above owns
        # them. sed rather than tail -n +2, because a file may carry both.
        sed -e '1{/^#!/d;}' -e '/^set -euo pipefail$/d' "$member" \
            | awk 'NF || printed { print; printed = 1 }'
        first=0
    done
    [ "$first" -eq 0 ] || {
        printf '%s: group %s has no source files\n' "${0##*/}" "$group" >&2
        exit 65
    }
}

status=0
for group in core document table progress; do
    spec="$(group_output "$group")"
    target="$script_dir/${spec%%	*}"
    rendered="$(mktemp "${TMPDIR:-/tmp}/plan-lib.XXXXXX")"
    emit_library "$group" > "$rendered"
    if [ "$check_only" = true ]; then
        if ! cmp -s "$rendered" "$target"; then
            printf '%s: %s is stale; run %s\n' "${0##*/}" "${target##*/}" "${0##*/}" >&2
            status=1
        fi
    else
        cat "$rendered" > "$target"
        printf 'Wrote %s (%s lines from %s files)\n' "${target##*/}" \
            "$(grep -c . "$target")" "$(ls "$lib_root/$group" | wc -l | tr -d ' ')"
    fi
    rm -f "$rendered"
done

[ "$check_only" = false ] || [ "$status" -ne 0 ] || printf 'All four libraries are up to date\n'
exit "$status"
