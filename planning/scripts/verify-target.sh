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
# It is advisory: PASS is evidence of existence and of no static layout
# counter-evidence, not a proof the surface renders in a live browser. Record
# the outcome in the plan's discovery unit; steps 2-3 are the ones a
# theme-override search misses, so this tool checks them explicitly.
#
# Usage:
#   verify-target.sh <plan-directory> <WNN> [--repo <repository-root>]
#
# --repo defaults to the current directory. The unit's File/Scope columns are
# read from work-unit-inventory.md; layout scan roots are the repository's
# view/*/layout and layout directories (Magento-style) plus etc/view.xml.
#
# Exit codes: 0 = target present with no static counter-evidence;
# 1 = target missing or a layout removes its block; 2 = usage/plan error.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/plan-document-lib.sh"

usage() {
    printf 'Usage: %s <plan-directory> <WNN> [--repo <repository-root>]\n' "$(basename "$0")" >&2
    exit 2
}

help() {
    printf 'Usage: %s <plan-directory> <WNN> [--repo <repository-root>]\n' "$(basename "$0")"
    exit 0
}

repo_root=""
filtered_args=()
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) help ;;
        --repo)
            [ "$#" -ge 2 ] || usage
            repo_root="$2"
            shift 2
            ;;
        --repo=*)
            repo_root="${1#--repo=}"
            shift
            ;;
        *)
            filtered_args+=("$1")
            shift
            ;;
    esac
done
set -- "${filtered_args[@]}"

[ "$#" -eq 2 ] || usage
plan_dir="$1"; unit="$2"
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

echo "verify-target: $unit ($unit_type) file=$unit_file scope=$unit_scope subscope=$unit_subscope"
echo "verify-target: repository=$repo_root"

# Only template/markup targets have a render surface to verify.
case "$unit_type" in
    markup|style)
        : ;;
    *)
        echo "verify-target: type $unit_type has no render surface; skipping reachability checks"
        exit 0
        ;;
esac

[ "$unit_file" != "N/A" ] || { echo "verify-target: unit file is N/A; cannot verify a render surface" >&2; exit 1; }

issues=0
warnings=0
fail() { printf 'verify-target: FAIL %s\n' "$*" >&2; issues=$((issues + 1)); }
warn() { printf 'verify-target: WARN %s\n' "$*" >&2; warnings=$((warnings + 1)); }

# 1. Existence and ownership.
target_path="$repo_root/$unit_file"
if [ -f "$target_path" ]; then
    echo "verify-target: OK target file exists: $target_path"
    case "$unit_file" in
        vendor/*) echo "verify-target: OK owner: module (vendor)" ;;
        app/code/*) echo "verify-target: OK owner: module (app/code)" ;;
        app/design/*) echo "verify-target: OK owner: theme (app/design)" ;;
        *) echo "verify-target: OK owner: unclassified path (human check)" ;;
    esac
else
    fail "target file does not exist: $target_path"
fi

# Collect layout XML files (Magento-style view layout + etc/view.xml).
layout_files=()
while IFS= read -r -d '' f; do
    layout_files+=("$f")
done < <(find "$repo_root" -type f \( -name '*.xml' -path '*/view/*/layout/*' -o -name 'view.xml' \) -print0 2>/dev/null || true)
if [ "${#layout_files[@]}" -eq 0 ]; then
    echo "verify-target: no layout XML files found; remove/re-point checks skipped"
fi

# 2+3. Layout checks keyed on the block/selector name in the unit's scope.
# Strip a leading '#'/'.' — markup scopes may name a DOM id/class.
block_name="${unit_scope#\#}"
block_name="${block_name#.}"
if [ "${#layout_files[@]}" -gt 0 ] && [ -n "$block_name" ]; then
    removed="$(grep -hoE "<referenceBlock[^>]*name=\"${block_name}\"[^>]*remove=\"true\"" "${layout_files[@]}" 2>/dev/null || true)"
    if [ -n "$removed" ]; then
        fail "a layout removes block '$block_name' that renders this target: $(printf '%s' "$removed" | head -1)"
    else
        echo "verify-target: OK no layout removes block '$block_name'"
    fi
    repointed="$(grep -HnoE "<action[^>]*setTemplate[^>]*>|<block[^>]*template=\"[^\"]*\"" "${layout_files[@]}" 2>/dev/null | grep "$block_name" || true)"
    if [ -n "$repointed" ]; then
        warn "a layout re-points block '$block_name': $(printf '%s' "$repointed" | head -1)"
    else
        echo "verify-target: OK no layout re-points block '$block_name'"
    fi
elif [ "${#layout_files[@]}" -gt 0 ]; then
    echo "verify-target: markup unit has no block name in scope; remove/re-point checks skipped"
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
            while IFS= read -r o; do echo "verify-target:   $o"; done <<< "$overrides"
        else
            echo "verify-target: OK no theme override of $base under app/design"
        fi
        ;;
esac

if [ "$issues" -gt 0 ]; then
    printf 'verify-target: %d issue(s) found for %s — record this before planning against the target\n' "$issues" "$unit" >&2
    exit 1
fi
echo "verify-target: PASS ($warnings warning(s)) — no static counter-evidence for $unit target $unit_file"