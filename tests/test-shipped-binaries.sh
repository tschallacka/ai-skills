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
# (.agents/MAINTAINER.md 1.10). A re-committed blob fails here rather
# than passing quietly.
tracked_bins="$(git -C "$root" ls-files planning/bin chat/bin todo/bin bug-report/bin 2>/dev/null || true)"
t_assert_eq 'no bundled binary is tracked in git (.agents/MAINTAINER.md 1.10)' "$tracked_bins" ''

# ---- T70/W08: binaries.tsv vs skill_files() drift -------------------------
#
# Each row above says what a skill SHIPS. installer/src/50-manifest.sh's
# skill_files() is a second, independently maintained declaration of the
# same fact -- one case arm per skill, hand-written, and not machine-parsed
# from binaries.tsv anywhere -- so the two can silently disagree. This finds
# every (target, binary) pair one names that the other does not.

# The (target, binary) pairs one skill's binaries.tsv declares.
drift_binaries_tsv_pairs() { # <binaries.tsv path>
    awk -F'\t' '!/^#/ && NF == 4 && $1 != "target" { print $1 "/" $3 }' "$1" | LC_ALL=C sort -u
}

# The (target, binary) pairs one skill's own arm of skill_files() actually
# stages, extracted from a manifest file (the real one, or a fixture).
# skill_files() writes this three different ways depending on the skill
# (skill_artifact_files() calls inside a uname case block, raw printf lines
# inside a uname case block, or a flat heredoc list with no case-gating at
# all -- planning's own plan-overview/rjq rows are the last two), so this
# does not look for any one call syntax: it isolates the skill's own arm
# (from its `<skill>)` opener to the next bare `<word>)` arm-opener or
# `esac`, whichever comes first -- indentation-agnostic, since arms in this
# hand-maintained function are not indented consistently with each other --
# and then pulls every literal `bin/<target>/<binary>` substring out of its
# non-comment lines. That substring is what all three forms share.
drift_skill_files_pairs() { # <skill> <manifest path>
    local skill="$1" manifest="$2"
    awk -v skill="$skill" '
        $0 ~ "^[[:space:]]+" skill "\\)[[:space:]]*$" { in_arm = 1; next }
        in_arm && /^[[:space:]]+[A-Za-z0-9_-]+\)[[:space:]]*$/ { in_arm = 0 }
        in_arm && /^[[:space:]]+esac[[:space:]]*$/ { in_arm = 0 }
        in_arm { print }
    ' "$manifest" \
        | { grep -v '^[[:space:]]*#' || true; } \
        | { grep -oE 'bin/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+' || true; } \
        | sed 's#^bin/##' \
        | LC_ALL=C sort -u
}

# One line per disagreement: "<skill>: <pair> declared in binaries.tsv but
# not in skill_files()" or the reverse. Prints nothing when the two agree.
drift_report() { # <skill> <binaries.tsv path> <manifest path>
    local skill="$1" reg="$2" manifest="$3" declared actual p
    declared="$(drift_binaries_tsv_pairs "$reg")"
    actual="$(drift_skill_files_pairs "$skill" "$manifest")"
    while IFS= read -r p; do
        [ -n "$p" ] || continue
        printf '%s: %s declared in binaries.tsv but not in skill_files()\n' "$skill" "$p"
    done <<COMM
$(comm -23 <(printf '%s\n' "$declared") <(printf '%s\n' "$actual"))
COMM
    while IFS= read -r p; do
        [ -n "$p" ] || continue
        printf '%s: %s present in skill_files() but not declared in binaries.tsv\n' "$skill" "$p"
    done <<COMM
$(comm -13 <(printf '%s\n' "$declared") <(printf '%s\n' "$actual"))
COMM
}

# Self-test first: prove the check can actually fail before trusting it not
# to. Two fixtures, because skill_files() uses two different shapes for the
# forms drift_skill_files_pairs must recognize identically -- a
# skill_artifact_files()-wrapped case-block form (every skill but planning)
# and planning's own flat, unwrapped heredoc form.
self_test_dir="$(mktemp -d "${TMPDIR:-/tmp}/t-drift.XXXXXX")"

# Fixture 1: skill_artifact_files()-wrapped form, one row deliberately
# missing from binaries.tsv relative to what "skill_files()" (the fixture
# manifest) declares.
cat > "$self_test_dir/wrapped-manifest.sh" <<'EOF'
        widget)
            case "$(uname -s):$(uname -m)" in
                Linux:x86_64)
                    skill_artifact_files widget bin/x86_64-unknown-linux-musl/widget
                    ;;
            esac
            ;;
EOF
cat > "$self_test_dir/wrapped.tsv" <<'EOF'
# MODE: PROD
target	condition	binary	why
EOF
report="$(drift_report widget "$self_test_dir/wrapped.tsv" "$self_test_dir/wrapped-manifest.sh")"
t_assert_contains 'self-test (wrapped form) detects an injected mismatch' \
    'widget: x86_64-unknown-linux-musl/widget present in skill_files() but not declared in binaries.tsv' "$report"

# Fixture 2: planning's own flat, unwrapped heredoc form -- no case-gating,
# no skill_artifact_files() wrapper. A check that only recognized the
# wrapped form would false-positive-report this as fully absent (AR-7).
cat > "$self_test_dir/flat-manifest.sh" <<'EOF'
        gadget)
            cat <<'INNER'
bin/x86_64-unknown-linux-musl/gadget
INNER
            ;;
EOF
cat > "$self_test_dir/flat.tsv" <<'EOF'
# MODE: PROD
target	condition	binary	why
x86_64-unknown-linux-musl	Linux:x86_64|amd64	gadget	test fixture
x86_64-apple-darwin	Darwin:x86_64	gadget	test fixture, deliberately not in skill_files()
EOF
report="$(drift_report gadget "$self_test_dir/flat.tsv" "$self_test_dir/flat-manifest.sh")"
t_assert_contains 'self-test (flat heredoc form) detects an injected mismatch' \
    'gadget: x86_64-apple-darwin/gadget declared in binaries.tsv but not in skill_files()' "$report"
t_assert_eq 'self-test (flat heredoc form) does not false-positive the matching row' \
    "$(printf '%s\n' "$report" | grep -c 'x86_64-unknown-linux-musl/gadget' || true)" '0'

rm -rf "$self_test_dir"

# The real check, against the real tree: every skill with a binaries.tsv.
# Excludes benchmark/ and testing-stories/, whose own archived runs carry
# copies of other skills' registries as fixture data (a fixture's own
# top-level directory is not a real skill_files() arm, and cross-checking
# it against one would false-positive every row it declares).
manifest="$root/installer/src/50-manifest.sh"
for reg in $registries; do
    rel="${reg#"$root"/}"
    skill="${rel%%/*}"
    case "$skill" in
        benchmark|testing-stories) continue ;;
    esac
    drift="$(drift_report "$skill" "$reg" "$manifest")"
    [ -z "$drift" ] || while IFS= read -r line; do
        [ -n "$line" ] || continue
        t_fail "binaries.tsv/skill_files() drift: $line"
    done <<DRIFT
$drift
DRIFT
done

t_end
