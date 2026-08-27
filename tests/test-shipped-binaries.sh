#!/usr/bin/env bash
# MODE: DEV
# test-shipped-binaries.sh - validates every skill's binaries.tsv, the registry
# of prebuilt artifacts a skill SHIPS (as opposed to requires.tsv, which
# declares what the target machine must already have).
#
# A row may name a binary that is not built yet: the registry is the
# declaration, and a release builds against it. What must always hold is that
# the declaration is well formed, unambiguous, and that nothing sits under
# bin/ that no row accounts for.

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

    seen_targets="" seen_conditions=""
    while IFS= read -r row; do
        [ -n "$row" ] || continue
        fields="$(printf '%s' "$row" | awk -F'\t' '{print NF}')"
        [ "$fields" -eq 4 ] \
            || t_fail "$rel: row has $fields tab-separated fields, want 4: $row"

        target="$(printf '%s' "$row" | cut -f1)"
        condition="$(printf '%s' "$row" | cut -f2)"
        binary="$(printf '%s' "$row" | cut -f3)"
        why="$(printf '%s' "$row" | cut -f4)"

        case " $seen_targets " in
            *" $target "*) t_fail "$rel: target $target declared twice" ;;
        esac
        seen_targets="$seen_targets $target"

        case " $seen_conditions " in
            *" $condition "*) t_fail "$rel: condition '$condition' declared twice, so which row wins is ambiguous" ;;
        esac
        seen_conditions="$seen_conditions $condition"

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

t_end
