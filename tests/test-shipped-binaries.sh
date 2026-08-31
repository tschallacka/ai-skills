#!/usr/bin/env bash
# MODE: DEV
# test-shipped-binaries.sh - validates every skill's binaries.tsv, the registry
# of prebuilt artifacts a skill SHIPS (as opposed to requires.tsv, which
# declares what the target machine must already have).
#
# A row may name a binary that is not built yet: the registry is the
# declaration, and a release builds against it. What must always hold is that
# the declaration is well formed, unambiguous, and that nothing sits under
# bin/ that no row accounts for. A skill may ship several binaries for one
# target (one row per binary, e.g. a server and a client), so uniqueness is per
# (condition, binary), not per target.

set -euo pipefail
export LC_ALL=C

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$root/planning/tests/lib-test.sh"
t_begin

want_header="$(printf 'target\tcondition\tbinary\twhy')"

# rustc validates the triples when the dev shell is present; without it that
# one assertion is skipped rather than guessed at.
rust_targets=""
if command -v rustc >/dev/null 2>&1; then
    rust_targets="$(rustc --print target-list 2>/dev/null || true)"
fi

registries="$(find "$root" -name binaries.tsv -not -path '*/.git/*' | sort)"
[ -n "$registries" ] || t_fail 'no binaries.tsv found anywhere; this test has nothing to guard'

for reg in $registries; do
    rel="${reg#"$root"/}"
    skill="${rel%%/*}"

    awk '/^# MODE: PROD$/ { found = 1 } END { exit !found }' "$reg" \
        || t_fail "$rel: no '# MODE: PROD' marker (it ships, so the installer reads it on the target)"

    header="$(awk '!/^#/ && NF { print; exit }' "$reg")"
    t_assert_eq "$rel: header is the declared column set" "$header" "$want_header"

    rows="$(awk '!/^#/ && NF' "$reg" | tail -n +2)"
    [ -n "$rows" ] || t_fail "$rel: registry has a header but no rows"

    seen_pairs="" seen_targets=""
    while IFS= read -r row; do
        [ -n "$row" ] || continue
        fields="$(printf '%s' "$row" | awk -F'\t' '{print NF}')"
        [ "$fields" -eq 4 ] \
            || t_fail "$rel: row has $fields tab-separated fields, want 4: $row"

        target="$(printf '%s' "$row" | cut -f1)"
        condition="$(printf '%s' "$row" | cut -f2)"
        binary="$(printf '%s' "$row" | cut -f3)"
        why="$(printf '%s' "$row" | cut -f4)"

        # A skill may ship several binaries for one target (e.g. a server and a
        # client). Uniqueness is per (condition, binary), so the same binary for
        # the same condition is never declared twice, but a target may host as
        # many binaries as it ships.
        pair="$condition|$binary"
        case " $seen_pairs " in
            *" $pair "*) t_fail "$rel: (condition, binary) '$pair' declared twice, so which row wins is ambiguous" ;;
        esac
        seen_pairs="$seen_pairs $pair"
        seen_targets="$seen_targets $target"

        # condition is <uname -s glob>:<uname -m glob>
        case "$condition" in
            *:*) : ;;
            *) t_fail "$rel: condition '$condition' is not <uname -s>:<uname -m>" ;;
        esac
        [ -n "${condition%%:*}" ] || t_fail "$rel: condition '$condition' has an empty OS half"
        [ -n "${condition#*:}" ] || t_fail "$rel: condition '$condition' has an empty arch half"

        case "$binary" in
            */*) t_fail "$rel: binary '$binary' must be a bare filename; the path is bin/<target>/" ;;
            '') t_fail "$rel: row for $target names no binary" ;;
        esac

        # A Windows target must carry .exe, and only a Windows target may.
        case "$target" in
            *windows*|*cygwin*)
                case "$binary" in
                    *.exe) : ;;
                    *) t_fail "$rel: $target ships '$binary'; a Windows binary needs .exe" ;;
                esac ;;
            *)
                case "$binary" in
                    *.exe) t_fail "$rel: $target ships '$binary'; .exe on a non-Windows target" ;;
                esac ;;
        esac

        [ -n "$why" ] || t_fail "$rel: row for $target has an empty why column"

        if [ -n "$rust_targets" ]; then
            printf '%s\n' "$rust_targets" | awk -v t="$target" '$0 == t { found = 1 } END { exit !found }' \
                || t_fail "$rel: '$target' is not a target rustc knows"
        fi

        # Unbuilt is legal; built-but-wrong is not.
        art="$(dirname "$reg")/bin/$target/$binary"
        if [ -e "$art" ]; then
            [ -f "$art" ] || t_fail "$rel: $art exists but is not a regular file"
            [ -x "$art" ] || t_fail "$rel: $art is not executable"
        fi
    done <<ROWS
$rows
ROWS

    # Nothing under bin/ that no row accounts for.
    bindir="$(dirname "$reg")/bin"
    if [ -d "$bindir" ]; then
        for d in "$bindir"/*; do
            [ -e "$d" ] || continue
            name="${d##*/}"
            case " $seen_targets " in
                *" $name "*) : ;;
                *) t_fail "$rel: bin/$name is not a target declared in the registry" ;;
            esac
        done
    fi
done


# The installer's own file list must name exactly the artifacts the registry
# declares. This arm came from the plan-overview work: a skill that gains a
# second binary is easy to add to binaries.tsv and easy to forget in the
# manifest, and then the installer ships a skill missing one of its binaries.
declared_paths="$(awk -F'\t' '!/^[[:space:]]*#/ && NF && $1 != "target" { print "bin/" $1 "/" $3 }' \
    "$root/planning/binaries.tsv" | sort -u)"
installer_paths="$(awk '/^SKILL.md$/{in_plan=1} in_plan && /^bin\//{print} in_plan && /^EOF$/{exit}' \
    "$root/install.sh" | sort -u)"
if [ "$declared_paths" != "$installer_paths" ]; then
    printf 'installer binary paths differ from planning/binaries.tsv\n' >&2
    printf 'registry:\n%s\ninstaller:\n%s\n' "$declared_paths" "$installer_paths" >&2
    t_fail 'install.sh does not ship every declared planning binary'
fi

t_end
