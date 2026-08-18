#!/usr/bin/env bash
# verify-target.sh — statically check that a work unit's target surface renders.
#
# The plan gate requires evidence that a template/block/layout target actually
# renders before work is planned against it (a file existing is not evidence).
# This helper performs the static half of that check for one work unit:
#   1. the target file exists, and whose it is (core / module / theme);
#   2. no layout removes the block that renders it (<referenceBlock remove="true">);
#   3. no layout re-points it to another template (setTemplate / re-registered
#      <block ... template>);
#   4. for a module template, whether any theme overrides it.
#
# Usage:
#   verify-target.sh <plan-directory> <WNN> [--repo <repository-root>]
#   verify-target.sh --help
#
# Exit codes: 0 = target present with no static counter-evidence (or the unit's
# type has no render surface, in which case NOTHING was checked — see below);
# 1 = target missing or a layout removes its block; 64 = usage/plan error.
#
# It is advisory: PASS is evidence of existence and of no static layout
# counter-evidence, not a proof the surface renders in a live browser. Record
# the outcome in the plan's discovery unit; steps 2-3 are the ones a
# theme-override search misses, so this tool checks them explicitly.
#
# --repo defaults to the current directory. The unit's File/Scope columns are
# read from work-unit-inventory.md; layout scan roots are the repository's
# view/*/layout and layout directories (Magento-style) plus etc/view.xml.
#
# KNOWN GAP (deliberate, reported, not fixed here): the reachability checks (2-4)
# run only for `markup` and `style` units. A template target recorded as
# `source`, and every `discovery` unit, exits 0 having run NO reachability check
# at all, even though SKILL.md:1237 presents this script as the static half of
# the reachability gate at SKILL.md:227-248. The skip line on stdout says so
# explicitly so a caller cannot mistake it for a pass.
#
# Output discipline (CODE-STYLE.md section 10): stdout carries exactly one
# result line (PASS / SKIP); every per-check OK, WARN and FAIL diagnostic goes
# to stderr, so `x="$(verify-target.sh …)"` yields the verdict alone.

set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} <plan-directory> <WNN> [--repo <repository-root>]
       ${0##*/} --help
USAGE
    exit "$rc"
}

note() { printf 'verify-target: %s\n' "$*" >&2; }

# A flag loop, not a `filtered_args` pre-scan: `set -- "${filtered_args[@]}"` is
# unbound under `set -u` before bash 4.4 when every argument was a flag.
repo_root=""
plan_dir=""
unit=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --repo) [ "$#" -ge 2 ] || usage; repo_root="$2"; shift 2 ;;
        --repo=*) repo_root="${1#--repo=}"; shift ;;
        --) shift; break ;;
        -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; usage ;;
        *)
            if [ -z "$plan_dir" ]; then
                plan_dir="$1"
            elif [ -z "$unit" ]; then
                unit="$1"
            else
                usage
            fi
            shift
            ;;
    esac
done
if [ -z "$plan_dir" ] || [ -z "$unit" ]; then
    usage
fi

[[ "$unit" =~ ^W[0-9][0-9]+$ ]] || plan_die "Work-unit ID must use W01"
plan_require_directory "$plan_dir"
[ -n "$repo_root" ] || repo_root="$(pwd)"
[ -d "$repo_root" ] || plan_die "repository root not found: $repo_root"

inventory="$plan_dir/work-unit-inventory.md"
[ -f "$inventory" ] || plan_die "work-unit inventory not found: $inventory"

IFS=$'\t' read -r unit_type unit_file unit_scope unit_subscope < <(awk -F'|' -v wanted="$unit" '
    /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {
        id = $2; gsub(/^[[:space:]]+|[[:space:]]+$/, "", id)
        if (id == wanted) {
            t = $3; f = $4; s = $5; ss = $6
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", t)
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", f); gsub(/^`|`$/, "", f)
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", s); gsub(/^`|`$/, "", s)
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", ss); gsub(/^`|`$/, "", ss)
            print t "\t" f "\t" s "\t" ss
        }
    }' "$inventory")
[ -n "$unit_type" ] || plan_die "work unit $unit not found in $inventory"

note "$unit ($unit_type) file=$unit_file scope=$unit_scope subscope=$unit_subscope"
note "repository=$repo_root"

# Only template/markup targets have a render surface to verify. Every other type
# leaves this script with nothing checked — say so on the result line.
case "$unit_type" in
    markup|style) ;;
    *)
        printf 'verify-target: SKIP %s — type %s has no render surface; NO reachability check ran for this type (checks 2-4 apply to markup|style only)\n' \
            "$unit" "$unit_type"
        exit 0
        ;;
esac

[ "$unit_file" != "N/A" ] || { note "unit file is N/A; cannot verify a render surface"; exit 1; }

issues=0
warnings=0
fail() { printf 'verify-target: FAIL %s\n' "$*" >&2; issues=$((issues + 1)); }
warn() { printf 'verify-target: WARN %s\n' "$*" >&2; warnings=$((warnings + 1)); }

# 1. Existence and ownership.
target_path="$repo_root/$unit_file"
if [ -f "$target_path" ]; then
    note "OK target file exists: $target_path"
    case "$unit_file" in
        vendor/*) note "OK owner: module (vendor)" ;;
        app/code/*) note "OK owner: module (app/code)" ;;
        app/design/*) note "OK owner: theme (app/design)" ;;
        *) note "OK owner: unclassified path (human check)" ;;
    esac
else
    fail "target file does not exist: $target_path"
fi

# Collect layout XML files (Magento-style view layout + etc/view.xml).
layout_files=()
while IFS= read -r -d '' f; do
    layout_files+=("$f")
done < <(find "$repo_root" -type f \( -name '*.xml' -path '*/view/*/layout/*' -o -name 'view.xml' \) -print0 2>/dev/null || true)

# Strip a leading '#'/'.' — markup scopes may name a DOM id/class. Skip the
# block entirely when no layout file was found: grep with no file operand reads
# stdin and hangs, and an empty array is unbound under `set -u` before bash 4.4.
block_name="${unit_scope#\#}"
block_name="${block_name#.}"
if [ "${#layout_files[@]}" -eq 0 ]; then
    note "no layout XML files found; remove/re-point checks did not run"
elif [ -n "$block_name" ]; then
    removed="$(grep -hoE "<referenceBlock[^>]*name=\"${block_name}\"[^>]*remove=\"true\"" ${layout_files[@]+"${layout_files[@]}"} 2>/dev/null || true)"
    if [ -n "$removed" ]; then
        fail "a layout removes block '$block_name' that renders this target: $(printf '%s' "$removed" | head -1)"
    else
        note "OK no layout removes block '$block_name'"
    fi
    repointed="$(grep -HnoE "<action[^>]*setTemplate[^>]*>|<block[^>]*template=\"[^\"]*\"" ${layout_files[@]+"${layout_files[@]}"} 2>/dev/null | grep "$block_name" || true)"
    if [ -n "$repointed" ]; then
        warn "a layout re-points block '$block_name': $(printf '%s' "$repointed" | head -1)"
    else
        note "OK no layout re-points block '$block_name'"
    fi
else
    note "markup unit has no block name in scope; remove/re-point checks did not run"
fi

# 4. Theme override: a module template may be overridden by a same-named file
# under app/design. Report for human judgement — overrides are not inherently
# wrong, but a plan targeting the module copy must know the theme wins.
case "$unit_file" in
    app/code/*|vendor/*)
        base="$(basename "$unit_file")"
        overrides="$(find "$repo_root/app/design" -type f -name "$base" -print 2>/dev/null || true)"
        if [ -n "$overrides" ]; then
            warn "theme override of $base exists (human check — the theme copy renders instead):"
            while IFS= read -r o; do note "  $o"; done <<< "$overrides"
        else
            note "OK no theme override of $base under app/design"
        fi
        ;;
esac

if [ "$issues" -gt 0 ]; then
    printf 'verify-target: %d issue(s) found for %s — record this before planning against the target\n' "$issues" "$unit" >&2
    exit 1
fi
printf 'verify-target: PASS (%s warning(s)) — no static counter-evidence for %s target %s\n' \
    "$warnings" "$unit" "$unit_file"
