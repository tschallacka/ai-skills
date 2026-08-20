#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: DEV
# test-mode-markers.sh — every file declares what it is and where it ships.
#
# Two markers, two questions, and they are not the same question:
#
#   MODE: PROD      the skill executes or reads this at run time
#   MODE: DEV       it exists for whoever maintains the skill
#   PACKAGE: PROD   an ordinary install delivers it
#   PACKAGE: DEV    only a dev install does
#
# The pairs that look contradictory are the informative ones. A function file
# under scripts/lib/ is MODE: PROD (it is runtime code) and PACKAGE: DEV (the
# file never ships; the library compiled from it does). A test is MODE: DEV and
# PACKAGE: DEV. install.sh is PROD in both.
#
# The gate: PACKAGE must agree with skill_files(). The marker is the declaration
# in the file, skill_files() is the list the installer reads, and neither is
# derived from the other -- so a disagreement means one of them is wrong, which is
# the same cross-check PACKAGE-MANIFEST.txt gets.
#
# Markdown carries the marker as an HTML comment: '# MODE: PROD' would render as
# a heading, and a second top-level heading is exactly what the document rules
# forbid. In a file with YAML frontmatter the marker follows it, because the
# frontmatter has to be the first thing in the file or the skill will not load.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$repo_root/planning/tests/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/mode-markers.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# shellcheck disable=SC1090
source "$repo_root/installer/src/05-config.sh"
# shellcheck disable=SC1090
source "$repo_root/installer/src/50-manifest.sh"
SOURCE_ROOT="$repo_root"
SOURCE_VERSION='test'
REPO_REF='test'

: > "$work/listed"
for skill in "${SKILL_NAMES[@]}"; do
    skill_files "$skill" | sed "s|^|$skill/|" >> "$work/listed"
done
sort -o "$work/listed" "$work/listed"
t_assert_eq 'the installer lists files to compare against' \
    "$([ "$(grep -c . "$work/listed")" -gt 100 ] && printf many)" 'many'

# A file that cannot carry a comment, or must not: its bytes are the test input,
# or the format has no comment syntax. These are declared by skill_files() alone.
exempt() { # <path> → prints the reason, or nothing
    case "$1" in
        */tests/fixtures/*)
            printf 'a fixture; its bytes are the test input\n' ;;
        *.json | *.jsonl | *.pub | */FIXTURE-VERSION)
            printf 'the format has no comment syntax\n' ;;
        planning/PACKAGE-MANIFEST.txt | planning/PACKAGE-MAP.tsv)
            printf 'structured data with its own cross-check\n' ;;
        *.gitignore)
            printf 'read by git, not by the skill\n' ;;
    esac
}

marker_of() { # <path> <MODE|PACKAGE> → DEV, PROD, or nothing
    sed -n '1,25p' "$1" \
        | sed -n -e "s/^# $2: \\([A-Z]*\\)\$/\\1/p" -e "s/^<!-- $2: \\([A-Z]*\\) -->\$/\\1/p" \
        | head -1
}

in_scope="$(cd "$repo_root" && git ls-files \
    planning project-specificies resource-limited-testing brainstorm \
    post-implementation-review todo bug-report installer tests \
    run-tests.sh blast-radius.sh generate-portability.sh install.sh install-ui.sh)"

missing='' bad_mode='' bad_package='' disagree='' checked=0
while IFS= read -r path; do
    [ -n "$path" ] || continue
    [ -z "$(exempt "$path")" ] || continue
    checked=$((checked + 1))
    mode="$(marker_of "$repo_root/$path" MODE)"
    package="$(marker_of "$repo_root/$path" PACKAGE)"
    if [ -z "$mode" ] || [ -z "$package" ]; then
        missing="$missing $path"
        continue
    fi
    case "$mode" in DEV|PROD) ;; *) bad_mode="$bad_mode $path=$mode" ;; esac
    case "$package" in DEV|PROD) ;; *) bad_package="$bad_package $path=$package" ;; esac

    # Only files inside a skill directory are subject to skill_files(); the
    # installer sources and the repo-level suite are not delivered by it at all.
    case "$path" in
        planning/* | project-specificies/* | resource-limited-testing/* | brainstorm/* \
            | post-implementation-review/* | todo/* | bug-report/*)
            if grep -qx "$path" "$work/listed"; then
                [ "$package" = PROD ] || disagree="$disagree $path(marked-DEV-but-listed)"
            else
                [ "$package" = DEV ] || disagree="$disagree $path(marked-PROD-but-unlisted)"
            fi
            ;;
    esac
done <<EOF
$in_scope
EOF

# A positive control: an empty scope would satisfy every check above.
t_assert_eq 'the marker scope is not empty' \
    "$([ "$checked" -gt 200 ] && printf 'over 200')" 'over 200'
t_assert_eq 'every file in scope declares both markers' "${missing# }" ''
t_assert_eq 'every MODE marker is DEV or PROD' "${bad_mode# }" ''
t_assert_eq 'every PACKAGE marker is DEV or PROD' "${bad_package# }" ''
t_assert_eq 'PACKAGE agrees with the installer file list' "${disagree# }" ''

# ── the markers say the things that make them worth having ──────────────────
# A compiled library is a runtime artifact even though its sources are not
# shipped, and the generator has to say so or the next rebuild drops it.
for lib in plan-core-lib.sh plan-document-lib.sh plan-table-lib.sh plan-progress-lib.sh; do
    t_assert_eq "$lib is declared a runtime artifact" \
        "$(marker_of "$repo_root/planning/scripts/$lib" MODE)" 'PROD'
    t_assert_eq "$lib is declared shipped" \
        "$(marker_of "$repo_root/planning/scripts/$lib" PACKAGE)" 'PROD'
done
# Its sources are the opposite case: runtime code that never ships as a file.
t_assert_eq 'a function file is runtime code' \
    "$(marker_of "$repo_root/planning/scripts/lib/core/plan_die.sh" MODE)" 'PROD'
t_assert_eq 'a function file does not ship as a file' \
    "$(marker_of "$repo_root/planning/scripts/lib/core/plan_die.sh" PACKAGE)" 'DEV'
# install.sh is assembled from MODE: DEV parts and must not inherit their marker.
t_assert_eq 'install.sh is what a user runs, so it is PROD' \
    "$(marker_of "$repo_root/install.sh" MODE)" 'PROD'
t_assert_eq 'and no source marker leaked into it' \
    "$(grep -c '^# MODE: \|^# PACKAGE: ' "$repo_root/install.sh")" '2'
t_assert_eq 'an installer part is a maintainer file' \
    "$(marker_of "$repo_root/installer/src/50-manifest.sh" MODE)" 'DEV'
t_assert_eq 'the compiler is a maintainer file' \
    "$(marker_of "$repo_root/planning/scripts/build-plan-libs.sh" MODE)" 'DEV'

# ── the exemptions are exemptions, not a blanket ────────────────────────────
t_assert_eq 'an ordinary script is not exempt' "$(exempt planning/scripts/plan-root.sh)" ''
t_assert_eq 'a SKILL.md is not exempt' "$(exempt todo/SKILL.md)" ''
t_assert_contains 'a fixture is exempt, and says why' 'test input' \
    "$(exempt planning/tests/fixtures/adversary-probe/progress.md)"
t_assert_contains 'a json file is exempt, and says why' 'comment syntax' \
    "$(exempt planning/placeholders.json)"

# ── a skill still loads: the frontmatter is first ───────────────────────────
# The marker went above the frontmatter on the first attempt, which would have
# stopped every SKILL.md being parsed as a skill at all.
for skill in "${SKILL_NAMES[@]}"; do
    t_assert_eq "$skill/SKILL.md still opens with its frontmatter" \
        "$(head -1 "$repo_root/$skill/SKILL.md")" '---'
    t_assert_eq "$skill/SKILL.md still declares a name" \
        "$(sed -n '2,6p' "$repo_root/$skill/SKILL.md" | grep -c '^name: ')" '1'
done

t_end
