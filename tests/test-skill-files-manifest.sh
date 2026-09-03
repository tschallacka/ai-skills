#!/usr/bin/env bash
# MODE: DEV
# test-skill-files-manifest.sh — skill_files() lists what the skill actually has.
#
# skill_files() is a hand-written list, on purpose: the planning arm is a second
# copy of PACKAGE-MANIFEST.tsv and the duplication IS the cross-check. A hand
# list only works if something notices when it drifts, in either direction:
#
#   listed but absent    an installed skill missing a file it was promised --
#                        a rename or a version bump nobody carried through here
#   present but unlisted a file added to a skill directory that no install ever
#                        delivers, so it works for the author and nowhere else
#
# Both are checked against `git ls-files`, which is the packaged set: package.json
# ships whole skill directories, so a tracked file is a shipped file.
#
# Two skills legitimately hold files no install delivers, and each exemption is a
# rule rather than a list of names, so a new file still has to justify itself.

set -euo pipefail
export LC_ALL=C

# --declarations-only drops the two assertions that need a BUILT tree -- every
# promised file existing, and every bundled artifact being executable -- and
# keeps the two that are pure declaration checks. pre-push-check runs it that
# way, because it must work on a tree that has never run setup-dev-env.sh; the
# full suite runs it whole, where a built tree is a fair assumption.
declarations_only=false
case "${1:-}" in
    --declarations-only) declarations_only=true ;;
    '') ;;
    *) printf '%s: unknown argument: %s\n' "${0##*/}" "$1" >&2; exit 64 ;;
esac

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$repo_root/planning/tests/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/skill-files.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# The five compiled libraries are generated and never tracked (MAINTAINER.md
# section 2.15), so a clean checkout has none. Build-if-missing so prepack on
# an unbuilt tree passes; this test asserts presence, the lib test owns content.
libs_missing=0
for lib in plan-core-lib.sh plan-crypt-lib.sh plan-document-lib.sh plan-progress-lib.sh plan-table-lib.sh; do
    [ -f "$repo_root/planning/scripts/$lib" ] || libs_missing=1
done
if [ "$libs_missing" -eq 1 ]; then
    "$repo_root/planning/scripts/build-plan-libs.sh" >/dev/null
fi
if [ ! -f "$repo_root/planning/REVIEWER.md" ]; then
    "$repo_root/planning/scripts/generate-reviewer.sh" >/dev/null
fi

# SOURCE_ROOT is assigned after sourcing, because 05-config.sh declares it empty.
# skill_files() reads it for the globbed arms, so an empty root silently yields a
# short list -- which is why the count assertions below are not decoration.
# shellcheck disable=SC1090
source "$repo_root/installer/src/05-config.sh"
# shellcheck disable=SC1090
source "$repo_root/installer/src/50-manifest.sh"
SOURCE_ROOT="$repo_root"
SOURCE_VERSION='test'
REPO_REF='test'

# A file a skill directory may hold without an install delivering it. The reason
# is the point: without one, the file is a packaging bug, not an exemption.
unshipped_reason() { # <skill> <path> → prints the reason, or nothing
    case "$1/$2" in
        planning/bin/*/plan-overview|planning/bin/*/plan-overview.exe)
            printf 'a release artifact built by the platform-specific workflow\n' ;;
        planning/scripts/overview-*|planning/scripts/render-plan-overview.sh|planning/scripts/runtime/overview-*|planning/templates/plan-overview.html.tmpl)
            printf 'retired legacy overview wrapper; the compiled renderer is authoritative\n' ;;
        # Developer-facing documentation and the maintainer's own tooling. The
        # shipped set of planning is PACKAGE-MANIFEST.tsv, diffed against this
        # list by planning/tests/test-installer-manifest.sh.
        planning/ARCHITECTURE.md | planning/MAINTAINER.md | planning/PACKAGE-MAP.tsv | planning/.gitignore)
            printf 'developer documentation, not part of the installed skill\n' ;;
        planning/rust-migration.tsv)
            printf 'developer migration inventory, not part of the installed skill\n' ;;
        # The per-function sources and the compiler that turns them into the
        # shipped plan-*-lib.sh. The compiled libraries are listed; their inputs
        # are not, or every install would carry both copies.
        planning/scripts/lib/* | planning/scripts/build-plan-libs.sh)
            printf 'a compiler input; the compiled library is what ships\n' ;;
        planning/scripts/*)
            case "$2" in
                *.sh) : ;;
                *) printf 'a generated extensionless Rust command; the build pipeline stages it\n' ;;
            esac ;;
        planning/bin/*)
            printf 'a compiled artifact for another supported platform\n' ;;
        *)
            # Nothing else is exempt. Every file in a skill directory is in the
            # prod arm of skill_files() or its dev arm, so one in neither is a
            # packaging omission, not a decision still to be made.
            : ;;
    esac
}

total_listed=0
for skill in "${SKILL_NAMES[@]}"; do
    skill_files "$skill" | sort > "$work/listed"
    skill_files "$skill" dev | sort > "$work/listed_dev"
    ( cd "$repo_root/$skill" && git ls-files | sort ) > "$work/tracked"

    # Untracked files are the blind spot, not empty lists: git ls-files cannot
    # see a file that has not been staged, so the accounting below excludes it
    # and reports a clean PASS about a set that silently omits the very file
    # someone just added. Naming them costs nothing and is not a failure --
    # unstaged work in progress is legitimate -- but a PASS must not be silent
    # about what it did not examine (T66).
    untracked="$( cd "$repo_root/$skill" && git ls-files --others --exclude-standard | tr '\n' ' ' )"
    [ -z "$untracked" ] || printf '%s: NOTE %s untracked file(s) not covered by this check: %s\n' \
        "${0##*/}" "$skill" "${untracked% }" >&2

    listed_count="$(grep -c . < "$work/listed" || true)"
    tracked_count="$(grep -c . < "$work/tracked" || true)"
    # A positive control on the harness itself: an empty list would satisfy the
    # "everything listed exists" check for every skill.
    [ "$listed_count" -gt 0 ] || t_fail "$skill: skill_files() listed nothing"
    [ "$tracked_count" -gt 0 ] || t_fail "$skill: the skill directory tracks nothing"
    total_listed=$((total_listed + listed_count))

    # Existence is asked of the filesystem, not of git ls-files: the installer
    # copies files, so an unstaged rename is exactly the case that must fail here.
    # (git ls-files still reported the old name, and this check passed on a
    # renamed schema until it was moved off the index.)
    absent=''
    while IFS= read -r path; do
        [ -n "$path" ] || continue
        [ -e "$repo_root/$skill/$path" ] || {
            case "$skill/$path" in
                planning/bin/*|chat/bin/*|ai-text-editor/bin/*|todo/bin/*|bug-report/bin/*) continue ;; # cross-target artifacts are CI outputs
                *) absent="$absent $path" ;;
            esac
        }
    done < "$work/listed"
    if [ "$declarations_only" = false ]; then
        t_assert_eq "$skill: every file skill_files() promises exists on disk" "${absent# }" ''
    fi
    # A present cross-target artifact must be a regular executable file: the
    # installer copies and execs it, so a stub or a wrong-arch file here fails
    # at install time in ways the suite would not see. Absent rows are
    # CI-delivered and skipped above.
    invalid=''
    while IFS= read -r path; do
        [ -n "$path" ] || continue
        case "$skill/$path" in
            planning/bin/*) ;;
            *) continue ;;
        esac
        p="$repo_root/$skill/$path"
        [ -e "$p" ] || continue
        { [ -f "$p" ] && [ -x "$p" ]; } || invalid="$invalid $path"
    done < "$work/listed"
    if [ "$declarations_only" = false ]; then
        t_assert_eq "$skill: every present bundled artifact is a regular executable" "${invalid# }" ''
    fi
    # The dev arm is inclusive, so between them the two arms must account for
    # every tracked file. A file in neither is one no install can deliver.
    unexplained=''
    while IFS= read -r path; do
        [ -n "$path" ] || continue
        grep -qx "$path" "$work/listed_dev" && continue
        [ -n "$(unshipped_reason "$skill" "$path")" ] || unexplained="$unexplained $path"
    done < <(comm -13 "$work/listed" "$work/tracked")
    t_assert_eq "$skill: the two arms account for every tracked file" \
        "${unexplained# }" ''
    # And dev has to be a superset, or calling it inclusive is not true.
    t_assert_eq "$skill: the dev arm contains the whole prod arm" \
        "$(comm -23 "$work/listed" "$work/listed_dev" | tr '\n' ' ')" ''
done

# The counts are a second control: a case arm that stopped matching would drop a
# skill's whole list, and each per-skill check above would still pass vacuously
# were it not for the guard, so pin the total the manifest is expected to carry.
t_assert_eq 'the manifest lists the expected number of files in total' \
    "$([ "$total_listed" -ge 90 ] && printf 'at least 90')" 'at least 90'
# What the run actually examined. A bare PASS reads the same whether it checked
# a hundred files or none of the one you just added (T66).
printf '%s: accounted for %s listed file(s) across %s skill(s)\n' \
    "${0##*/}" "$total_listed" "${#SKILL_NAMES[@]}" >&2


# ── the exemptions are exemptions, not a blanket ────────────────────────────
# A rule that matched everything would make the check above meaningless.
t_assert_eq 'an ordinary skill file is not exempt' \
    "$(unshipped_reason planning SKILL.md)" ''
t_assert_eq 'a new script under scripts/ is not exempt' \
    "$(unshipped_reason planning scripts/plan-new-thing.sh)" ''
t_assert_eq 'a new file in a register skill is not exempt' \
    "$(unshipped_reason todo schema.9.9.9.json)" ''
t_assert_contains 'a compiler input is exempt, and says why' 'compiled library' \
    "$(unshipped_reason planning scripts/lib/core/plan_die.sh)"

t_end
