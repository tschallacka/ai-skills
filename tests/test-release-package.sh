#!/usr/bin/env bash
# MODE: DEV
# test-release-package.sh — the tarball holds exactly what should be packed.
#
# The release asset is the article an end user receives. Until this file existed
# it was checked by hand: a diff of --list against npm pack, and a file count.
# Neither survives a change nobody re-runs it after.
#
# The expected set is derived here from the markers and the installer's prod arm,
# independently of build-release.sh --list. That is deliberate duplication: if the
# test asked the builder what it built, it would agree with itself no matter what
# the rule said. The two derivations are compared, so a disagreement fails.
#
# Four properties, in the order they matter:
#
#   1. exactly the expected paths -- nothing missing, nothing extra
#   2. every file byte-identical to the repository copy
#   3. no file marked MODE: DEV anywhere inside it
#   4. two builds of one tree produce the same bytes
#
# and then the one that subsumes them: the extracted tarball installs.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/.." && pwd)"
builder="$repo_root/installer/build-release.sh"
# shellcheck source=planning/tests/lib-test.sh
source "$repo_root/planning/tests/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/release-package.XXXXXX")"
trap 'rm -rf "$work"' EXIT

version="$(sed -n 's/^[[:space:]]*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
    "$repo_root/package.json" | head -1)"
t_assert_eq 'package.json states a version' \
    "$(printf '%s' "$version" | grep -c '^[0-9][0-9.]*[0-9]$')" '1'

# ── the expected set, derived from the rule rather than from the builder ─────
declares_prod() { # <path>
    case "$(sed -n '1,25p' "$repo_root/$1" 2>/dev/null)" in
        *'# MODE: PROD'*|*'<!-- MODE: PROD -->'*) return 0 ;;
    esac
    return 1
}
# Unmarked files -- fixtures, formats with no comment syntax -- ship when the
# skill that owns them does, which is what the installer's prod arm says.
# shellcheck disable=SC1090
source "$repo_root/installer/src/05-config.sh"
# shellcheck disable=SC1090
source "$repo_root/installer/src/50-manifest.sh"
SOURCE_ROOT="$repo_root"
SOURCE_VERSION='test'
REPO_REF='test'

{
    printf 'install.sh\ninstall-ui.sh\nREADME.md\nLICENSE\npackage.json\n'
    while IFS= read -r path; do
        [ -n "$path" ] || continue
        declares_prod "$path" && printf '%s\n' "$path"
    done < <(cd "$repo_root" && git ls-files \
        planning project-specificies resource-limited-testing brainstorm \
        post-implementation-review todo bug-report)
    for skill in "${SKILL_NAMES[@]}"; do
        skill_files "$skill" | sed "s|^|$skill/|"
    done
} | sort -u > "$work/expected"

expected_count="$(grep -c . "$work/expected")"
# A positive control: an empty expectation would make every comparison below
# trivially true.
t_assert_eq 'the expected set is not empty' \
    "$([ "$expected_count" -gt 50 ] && printf 'over 50')" 'over 50'

# ── property 1: the tarball holds exactly those paths ───────────────────────
"$builder" --out "$work/dist" >/dev/null
tarball="$work/dist/ai-skills-$version.tar.gz"
[ -f "$tarball" ] || t_fail "the builder wrote no $tarball"

tar -tzf "$tarball" | sed "s|^ai-skills-$version/||" | grep -v '/$' | sort > "$work/actual"
t_assert_eq 'nothing expected is missing from the tarball' \
    "$(comm -23 "$work/expected" "$work/actual" | tr '\n' ' ')" ''
t_assert_eq 'nothing unexpected is in the tarball' \
    "$(comm -13 "$work/expected" "$work/actual" | tr '\n' ' ')" ''

# And the builder's own --list must agree with the rule, or the two have drifted.
"$builder" --list | sort -u > "$work/listed"
t_assert_eq "the builder's --list matches the derived set" \
    "$(comm -3 "$work/expected" "$work/listed" | tr '\n' ' ')" ''

# ── property 2: every file byte-identical to the repository copy ────────────
mkdir -p "$work/extract"
tar -xzf "$tarball" -C "$work/extract"
extracted="$work/extract/ai-skills-$version"
differing='' compared=0
while IFS= read -r path; do
    [ -n "$path" ] || continue
    compared=$((compared + 1))
    cmp -s "$repo_root/$path" "$extracted/$path" || differing="$differing $path"
done < "$work/expected"
t_assert_eq 'every packed file is byte-identical to the repository copy' "${differing# }" ''
t_assert_eq 'and every expected file was actually compared' "$compared" "$expected_count"

# ── property 3: nothing marked MODE: DEV is inside it ──────────────────────
# Read the header only: a heredoc lower down mentions the marker strings.
leaked=''
while IFS= read -r path; do
    [ -n "$path" ] || continue
    case "$(sed -n '1,25p' "$extracted/$path" 2>/dev/null)" in
        *'# MODE: DEV'*|*'<!-- MODE: DEV -->'*) leaked="$leaked $path" ;;
    esac
done < "$work/expected"
t_assert_eq 'no maintainer file reached the release' "${leaked# }" ''
# The categories that motivated the split, named so a regression says which.
t_assert_eq 'no test script is in the release' \
    "$(grep -c '^planning/tests/' "$work/actual" || true)" '0'
t_assert_eq 'no per-function library source is in the release' \
    "$(grep -c '^planning/scripts/lib/' "$work/actual" || true)" '0'
t_assert_eq 'the compiled libraries are' \
    "$(grep -c '^planning/scripts/plan-core-lib\.sh$' "$work/actual")" '1'
t_assert_eq 'and the compiler is not' \
    "$(grep -c 'build-plan-libs\.sh' "$work/actual" || true)" '0'

# ── property 4: two builds of one tree produce the same bytes ───────────────
"$builder" --out "$work/dist2" >/dev/null
t_assert_eq 'two builds of one tree are byte-identical' \
    "$(cmp -s "$tarball" "$work/dist2/ai-skills-$version.tar.gz" && printf same)" 'same'

# ── the property that subsumes the rest: the package installs ───────────────
# A tarball whose contents are correct but which cannot install is still broken.
rc=0
( cd "$extracted" && printf 'n\n' | "$BASH" ./install.sh --skill todo \
    --target "$work/installed" --yes ) >/dev/null 2>&1 || rc=$?
t_assert_eq 'the extracted package installs a skill' "$rc" '0'
t_assert_eq 'and the installed skill is complete' \
    "$(ls "$work/installed/todo" 2>/dev/null | sort | tr '\n' ' ')" \
    "SKILL.md requires.tsv schema.$version.json "

t_end
