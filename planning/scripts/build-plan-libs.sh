#!/usr/bin/env bash
# MODE: DEV
# build-plan-libs.sh — compile planning/scripts/lib/<group>/*.sh into the five
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
#   build-plan-libs.sh                  # write the libraries (prod target)
#   build-plan-libs.sh --target dev     # write them with the dev-only aids
#   build-plan-libs.sh --check          # exit 1 if a committed library is stale
#   build-plan-libs.sh --help
#
# Two targets, one concatenation. The prod target is what ships and what is
# committed: no provenance, and any function file marked `# PACKAGE: DEV` left
# out. The dev target adds a provenance line before each function so a stack
# trace or a grep hit in the compiled file names the source file to edit, and it
# includes the dev-only functions.
#
# The dev target writes to the same paths on purpose -- it is the same library,
# built for a maintainer's machine. --check always compares against a PROD build,
# so a dev build left in the tree reads as stale and cannot be committed by
# accident. That is the guard; there is no second committed copy to keep fresh.
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
Usage: ${0##*/} [--target dev|prod]
       ${0##*/} --check
       ${0##*/} --help

  --target  prod (default) is what ships: no provenance comments, and function
            files marked '# PACKAGE: DEV' are left out. dev keeps both.
  --check   compare the committed libraries against a fresh PROD build and exit
            1 on any difference, naming the group that drifted. A dev build in
            the tree is a difference, which is what stops it being committed.
USAGE
    exit "${1:-64}"
}

check_only=false
target=prod
while [ "$#" -gt 0 ]; do
    case "$1" in
        --check) check_only=true ;;
        --target)
            [ "$#" -ge 2 ] || { printf '%s: --target needs a value\n' "${0##*/}" >&2; usage; }
            target="$2"
            shift
            ;;
        --target=*) target="${1#--target=}" ;;
        -h|--help) usage 0 ;;
        *) printf '%s: unknown argument: %s\n' "${0##*/}" "$1" >&2; usage ;;
    esac
    shift
done
case "$target" in
    dev|prod) ;;
    *) printf '%s: --target must be dev or prod, not %s\n' "${0##*/}" "$target" >&2; usage ;;
esac
# --check is about what is committed, and what is committed is the prod build.
[ "$check_only" = false ] || target=prod

# The group's own name, its output file, and the one-line purpose that goes in
# the generated header. Adding a group means one row here and one directory.
group_output() {
    case "$1" in
        core)     printf 'plan-core-lib.sh\tfailure, guards, temp files, atomic writes, plan root, snapshots\n' ;;
        document) printf 'plan-document-lib.sh\tsections, paragraphs, titles and fields\n' ;;
        table)    printf 'plan-table-lib.sh\tCSV and Markdown table rendering\n' ;;
        progress) printf 'plan-progress-lib.sh\tprogress arithmetic and the status glyphs\n' ;;
        crypt)    printf 'plan-crypt-lib.sh\tSHA-256 digests, fix-key derivation and OS entropy\n' ;;
        *) return 1 ;;
    esac
}

# A function file the prod library does without. PACKAGE is the compiler's axis:
# PACKAGE: PROD means "compile me into the end-user library", PACKAGE: DEV means
# "only into the dev build, which carries dev and prod inputs together". Every
# file here is MODE: DEV, because the file itself is a maintainer's -- what the
# end user receives is the compiled library.
#
# No `| grep -q`: with pipefail a grep that exits on its first match makes sed
# fail with SIGPIPE, which PORTABILITY.md bans as 'pipefail-grep-q'.
member_is_dev_only() { # <path>
    case "$(sed -n '1,15p' "$1")" in
        *'# PACKAGE: DEV'*) return 0 ;;
    esac
    return 1
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
    # The end user receives this file, so MODE: PROD. No PACKAGE: that axis is
    # for what a compiler consumes, and this is what a compiler produced.
    printf '# MODE: PROD\n'
    printf '# GENERATED FILE — do not edit. Compiled from scripts/lib/%s/*.sh by:\n' "$group"
    printf '#   planning/scripts/build-plan-libs.sh\n'
    printf '# Edit the function file in that directory, then re-run the build.\n'
    printf '# Target: %s\n' "$target"
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
        if [ "$target" = prod ] && member_is_dev_only "$member"; then
            continue
        fi
        printf '\n'
        [ "$target" = dev ] && printf '# from scripts/lib/%s/%s\n' "$group" "${member##*/}"
        # Strip the standalone shebang, any `set` line, and the file's own MODE
        # and PACKAGE markers: the header above owns them, and a marker copied
        # into a generated file would claim the compiled library is a source.
        # One expression per literal. `\(DEV\|PROD\)` is GNU sed only: BSD sed
        # does not take `\|` as alternation in a BRE, so on macOS it matched
        # nothing and every source marker was copied into the compiled library.
        sed -e '1{/^#!/d;}' -e '/^set -euo pipefail$/d' \
            -e '/^# MODE: DEV$/d' -e '/^# MODE: PROD$/d' \
            -e '/^# PACKAGE: DEV$/d' -e '/^# PACKAGE: PROD$/d' "$member" \
            | awk 'NF || printed { print; printed = 1 }'
        first=0
    done
    [ "$first" -eq 0 ] || {
        printf '%s: group %s has no source files\n' "${0##*/}" "$group" >&2
        exit 65
    }
}

status=0
for group in core document table progress crypt; do
    spec="$(group_output "$group")"
    output_path="$script_dir/${spec%%	*}"
    rendered="$(mktemp "${TMPDIR:-/tmp}/plan-lib.XXXXXX")"
    emit_library "$group" > "$rendered"
    if [ "$check_only" = true ]; then
        if ! cmp -s "$rendered" "$output_path"; then
            printf '%s: %s is stale; run %s\n' "${0##*/}" "${output_path##*/}" "${0##*/}" >&2
            status=1
        fi
    else
        cat "$rendered" > "$output_path"
        printf 'Wrote %s for %s (%s lines from %s files)\n' "${output_path##*/}" "$target" \
            "$(grep -c . "$output_path")" "$(ls "$lib_root/$group" | wc -l | tr -d ' ')"
    fi
    rm -f "$rendered"
done

[ "$check_only" = false ] || [ "$status" -ne 0 ] || printf 'All five libraries are up to date\n'
exit "$status"
