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

# No generated binary may be tracked: per-target artifacts are CI-delivered
# (planning/MAINTAINER.md section 2.15). A re-committed blob fails here rather
# than passing quietly.
tracked_bins="$(git -C "$root" ls-files planning/bin chat/bin todo/bin bug-report/bin 2>/dev/null || true)"
t_assert_eq 'no bundled binary is tracked in git (MAINTAINER.md section 2.15)' "$tracked_bins" ''

t_end
