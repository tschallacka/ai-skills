#!/usr/bin/env bash
# MODE: DEV
# test-mode-markers.sh — every file states its purpose, so it cannot be conflated.
#
# MODE answers who receives the file:
#
#   MODE: PROD   exclusive -- this file goes to the end user. If a release needs
#                it, it has to say PROD.
#   MODE: DEV    only a maintainer needs it. The dev side is inclusive: it has
#                every PROD file as well, so DEV means "and this too".
#
# PACKAGE is for compilers, and appears only on what a compiler consumes:
#
#   PACKAGE: PROD  compile this into the end-user artifact, which is packaged
#                  because that artifact is MODE: PROD
#   PACKAGE: DEV   compile it only into the dev build, which carries the dev and
#                  prod inputs together
#
# So a function file under scripts/lib/ is MODE: DEV and PACKAGE: PROD: the file
# is a maintainer's, and its content reaches the end user inside the compiled
# library. The library itself is MODE: PROD with no PACKAGE -- that axis belongs
# to inputs, and a compiled library is an output.
#
# The gate: MODE must agree with skill_files(). The marker is the declaration in
# the file, skill_files() is the list the installer reads, and neither is derived
# from the other -- so a disagreement means one of them is wrong, which is the
# same cross-check PACKAGE-MANIFEST.txt gets.
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
# A positive control on the comparison set itself: an empty list would make the
# MODE cross-check below pass for every file in the repository.
t_assert_eq 'the installer lists files to compare against' \
    "$([ "$(grep -c . "$work/listed")" -gt 50 ] && printf many)" 'many'

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
    # Only the header is read: a heredoc further down mentions these strings.
    sed -n '1,25p' "$1" \
        | sed -n -e "s/^# $2: \\([A-Z]*\\)\$/\\1/p" -e "s/^<!-- $2: \\([A-Z]*\\) -->\$/\\1/p" \
        | head -1
}

# PACKAGE belongs to what a compiler reads, and nothing else.
compiler_input() { # <path>
    case "$1" in
        planning/scripts/lib/*/*.sh) return 0 ;;
        installer/src/[0-9][0-9]-*.sh) return 0 ;;
    esac
    return 1
}

in_scope="$(cd "$repo_root" && git ls-files \
    planning project-specificies resource-limited-testing brainstorm \
    post-implementation-review todo bug-report installer tests \
    run-tests.sh blast-radius.sh generate-portability.sh verify-both-shells.sh \
    install.sh install-ui.sh)"

missing='' bad_mode='' bad_package='' stray_package='' input_mode='' disagree='' checked=0
while IFS= read -r path; do
    [ -n "$path" ] || continue
    [ -z "$(exempt "$path")" ] || continue
    checked=$((checked + 1))
    mode="$(marker_of "$repo_root/$path" MODE)"
    package="$(marker_of "$repo_root/$path" PACKAGE)"
    if [ -z "$mode" ]; then
        missing="$missing $path"
        continue
    fi
    case "$mode" in DEV|PROD) ;; *) bad_mode="$bad_mode $path=$mode" ;; esac

    if compiler_input "$path"; then
        case "$package" in
            DEV|PROD) ;;
            '') missing="$missing $path(no-PACKAGE)" ;;
            *) bad_package="$bad_package $path=$package" ;;
        esac
        # The file a compiler reads is a maintainer's file; what the end user
        # gets is the artifact compiled from it.
        [ "$mode" = DEV ] || input_mode="$input_mode $path"
    elif [ -n "$package" ]; then
        stray_package="$stray_package $path"
    fi

    # Only files inside a skill directory are subject to skill_files(); the
    # installer sources and the repo-level suite are not delivered by it at all.
    case "$path" in
        planning/* | project-specificies/* | resource-limited-testing/* | brainstorm/* \
            | post-implementation-review/* | todo/* | bug-report/*)
            if grep -qx "$path" "$work/listed"; then
                [ "$mode" = PROD ] || disagree="$disagree $path(marked-DEV-but-shipped)"
            else
                [ "$mode" = DEV ] || disagree="$disagree $path(marked-PROD-but-not-shipped)"
            fi
            ;;
    esac
done <<EOF
$in_scope
EOF

# A positive control: an empty scope would satisfy every check above.
t_assert_eq 'the marker scope is not empty' \
    "$([ "$checked" -gt 200 ] && printf 'over 200')" 'over 200'
t_assert_eq 'every file in scope declares its MODE' "${missing# }" ''
t_assert_eq 'every MODE marker is DEV or PROD' "${bad_mode# }" ''
t_assert_eq 'every PACKAGE marker is DEV or PROD' "${bad_package# }" ''
t_assert_eq 'PACKAGE appears only on what a compiler reads' "${stray_package# }" ''
t_assert_eq 'a compiler input is a maintainer file' "${input_mode# }" ''
t_assert_eq 'MODE agrees with the installer file list' "${disagree# }" ''

# ── the markers say the things that make them worth having ──────────────────
# A compiled library is what the end user receives, and it is an output, so it
# carries MODE and no PACKAGE. Its generator has to emit that or the next rebuild
# drops it.
for lib in plan-core-lib.sh plan-document-lib.sh plan-table-lib.sh plan-progress-lib.sh; do
    t_assert_eq "$lib goes to the end user" \
        "$(marker_of "$repo_root/planning/scripts/$lib" MODE)" 'PROD'
    t_assert_eq "$lib is an output, so it declares no PACKAGE" \
        "$(marker_of "$repo_root/planning/scripts/$lib" PACKAGE)" ''
done
# Its sources are the other case: a maintainer's files whose content is compiled
# into that library.
t_assert_eq 'a function file is a maintainer file' \
    "$(marker_of "$repo_root/planning/scripts/lib/core/plan_die.sh" MODE)" 'DEV'
t_assert_eq 'and its content is compiled into the end-user library' \
    "$(marker_of "$repo_root/planning/scripts/lib/core/plan_die.sh" PACKAGE)" 'PROD'
# install.sh is assembled from MODE: DEV parts and must not inherit their marker.
t_assert_eq 'install.sh is what a user runs, so it goes to the end user' \
    "$(marker_of "$repo_root/install.sh" MODE)" 'PROD'
t_assert_eq 'and no source marker leaked into it' \
    "$(grep -c '^# MODE: \|^# PACKAGE: ' "$repo_root/install.sh")" '1'
t_assert_eq 'an installer part is a maintainer file' \
    "$(marker_of "$repo_root/installer/src/50-manifest.sh" MODE)" 'DEV'
t_assert_eq 'whose content is compiled into install.sh' \
    "$(marker_of "$repo_root/installer/src/50-manifest.sh" PACKAGE)" 'PROD'
t_assert_eq 'the compiler itself is a maintainer file' \
    "$(marker_of "$repo_root/planning/scripts/build-plan-libs.sh" MODE)" 'DEV'
t_assert_eq 'and it is nothing else compiles, so it declares no PACKAGE' \
    "$(marker_of "$repo_root/planning/scripts/build-plan-libs.sh" PACKAGE)" ''

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
