#!/usr/bin/env bash
# MODE: PROD
# overview-state.sh - emit the complete reviewing state of one plan as a
# single JSON document on stdout. This document is the one source that both
# delivery modes (file artifact and served page) render from, so they cannot
# disagree about what the plan says.
#
# Usage:
#   overview-state.sh [--plan-dir] <plan-directory>
#   overview-state.sh --help
#
# Exit codes: 64 bad invocation, 66 missing plan directory.

set -euo pipefail
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/scripts/plan-document-lib.sh
source "$script_dir/plan-document-lib.sh"
eval "set -- $(plan_hoist_plan_dir 1 "$@")"

export LC_ALL=C

# rjq is the ceiling of the required runtime: refuse with 69 rather than
# half-emitting when it is missing (mirrors validate-plan.sh).
if ! command -v rjq >/dev/null 2>&1; then
    printf '%s: rjq is required (it assembles the JSON state); install rjq and re-run\n' \
        "${0##*/}" >&2
    exit 69
fi

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} [--plan-dir] <plan-directory>
       ${0##*/} --help

Emits the reviewing state of one plan as a single JSON document on stdout.
USAGE
    exit "$rc"
}

plan_dir=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --) shift; break ;;
        -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; usage ;;
        *) [ -z "$plan_dir" ] || usage; plan_dir="$1"; shift ;;
    esac
done
[ -n "$plan_dir" ] || usage
plan_require_directory "$plan_dir"
[ -f "$plan_dir/plan-description.md" ] || plan_die "plan-description.md not found: $plan_dir/plan-description.md" 66

# jstr TEXT: emit TEXT as one JSON string (safe inside any JSON position).
# Reads from a heredoc, NOT stdin — inside a while-read loop over the
# inventory file, an unredirected rjq would eat the remaining lines.
jstr() { rjq -Rs 'rtrimstr("\n")' <<< "$1"; }

# sec FILE HEADING: print the paragraphs under "## HEADING" until the next "##".
sec() {
    awk -v h="$2" '
        $0 == "## " h { f = 1; next }
        /^## / && f { exit }
        f { print }
    ' "$1"
}

emit=0
out() { [ "$emit" = 0 ] && emit=1; printf '%s' "$1"; }
comma() { [ "$emit" = 0 ] && emit=1 || printf ','; }

printf '{'

# ---- identity -----------------------------------------------------------------
title="$(sed -n 's/^# Plan: //p' "$plan_dir/plan-description.md" | head -1)"
ui_affected="$(grep -m1 '^- UI affected:' "$plan_dir/plan-description.md" | sed 's/^- UI affected: //' || true)"
review_status="$(sed -n 's/^- Status:[[:space:]]*`//p' "$plan_dir/adversarial-review.md" 2>/dev/null | head -1 || true)"
description_text="$(sec "$plan_dir/plan-description.md" 'Current state')$(sec "$plan_dir/plan-description.md" 'Desired outcome')"
comma
printf '"identity":{"title":%s,"uiAffected":%s,"reviewStatus":%s,"description":%s}' \
    "$(jstr "$title")" "$(jstr "${ui_affected:-no}")" "$(jstr "$review_status")" "$(jstr "$description_text")"

# ---- goals ---------------------------------------------------------------------
comma
printf '"goals":['
gfirst=1
for gdir_src in "$plan_dir"/*/; do :; done
while IFS= read -r gdir; do
    [ -f "$gdir/goal.md" ] || continue
    gid="$(basename "$gdir")"
    outcome="$(sec "$gdir/goal.md" 'Outcome and definition of done' | head -3)"
    treq=""
    trq_in=0
    while IFS= read -r trq_line || [ -n "$trq_line" ]; do
        case "$trq_line" in
            *'Testing requirement'*|*'Testing-requirement'*) trq_in=1; continue ;;
        esac
        [ "$trq_in" = 1 ] || continue
        case "$trq_line" in '## '*) break ;; esac
        case "$trq_line" in '|'*) ;; *) continue ;; esac
        # Row filter and cell spacing replicate the awk this replaced: the
        # raw line gate (Goal or dash marker skips), the key cell UNTRIMMED,
        # the value cell trimmed only.
        case "$trq_line" in *Goal*|*---*) continue ;; esac
        trq_raw2="$(printf '%s\n' "$trq_line" | tr '|' '\n' | sed -n 2p)"
        trq_val3="$(printf '%s\n' "$trq_line" | tr '|' '\n' | sed -n 3p | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
        [ -n "$treq" ] && treq="$treq"$'\n'
        treq="$treq$trq_raw2: $trq_val3"
    done < "$gdir/goal.md"
    [ $gfirst = 1 ] && gfirst=0 || printf ','
    printf '{"id":%s,"outcome":%s,"testingRequirement":%s}' \
        "$(jstr "$gid")" "$(jstr "$outcome")" "$(jstr "$treq")"
done < <(find "$plan_dir" -mindepth 1 -maxdepth 1 -type d | sort)
printf ']'

# ---- steps (with companion + status + criteria) ---------------------------------
comma
printf '"steps":['
sfirst=1
while IFS= read -r sfile; do
    rel="${sfile#"$plan_dir"/}"
    goal_dir_name="${rel%%/*}"
    step_name="$(basename "$sfile" .md)"
    unit="$(sed -n 's/^- Work unit: `//p' "$sfile" | tr -d '`')"
    stype="$(sed -n 's/^- Type: `//p' "$sfile" | tr -d '`')"
    sfile_tgt="$(sed -n 's/^- File: `//p' "$sfile" | tr -d '`')"
    instr="$(sec "$sfile" 'Instructions')"
    crit="$(sec "$sfile" 'Acceptance criteria')"
    companion="null"
    [ -f "${sfile%.md}-testing.md" ] && companion="\"${step_name}-testing.md\""
    # status from the owning goal's progress file
    pstatus="unknown"
    pfile="$plan_dir/$goal_dir_name/progress.md"
    if [ -f "$pfile" ]; then
        row="$(grep -F "| $step_name |" "$pfile" | head -1 || true)"
        case "$row" in *"✅ completed"*) pstatus="completed" ;; *"⏳ in progress"*) pstatus="in-progress" ;; *"💤 incomplete"*) pstatus="incomplete" ;; esac
    fi
    [ $sfirst = 1 ] && sfirst=0 || printf ','
    printf '{"goal":%s,"step":%s,"unit":%s,"type":%s,"target":%s,"companion":%s,"status":%s,"instructions":%s,"criteria":%s}' \
        "$(jstr "$goal_dir_name")" "$(jstr "$step_name")" "$(jstr "$unit")" "$(jstr "$stype")" \
        "$(jstr "$sfile_tgt")" "$companion" "$(jstr "$pstatus")" "$(jstr "$instr")" "$(jstr "$crit")"
done < <(find "$plan_dir" -mindepth 2 -path '*/steps/*.md' ! -name '*-testing.md' | sort)
printf ']'

# ---- dependency edges from the inventory ----------------------------------------
comma
printf '"edges":['
inv="$plan_dir/work-unit-inventory.md"
efirst=1
if [ -f "$inv" ]; then
    while IFS= read -r line; do
        # Only well-formed single-line rows: at least 10 inner columns.
        case "$line" in '| W'*) ;; *) continue ;; esac
        nfields=$(( $(printf '%s' "$line" | tr -cd '|' | wc -c) + 1 ))
        [ "$nfields" -ge 10 ] || continue
uid="$(plan_table_cell "$line" 2)"
deps="$(plan_table_cell "$line" 8)"
        case "$uid" in W[0-9]*) ;; *) continue ;; esac
        case "$deps" in ''|—) continue ;; esac
        dep_i=0
        oldIFS="$IFS"; IFS=','
        for d in $deps; do
            IFS="$oldIFS"
            d="$(printf '%s' "$d" | sed 's/^ *//; s/ *$//')"
            case "$d" in W[0-9]*) ;; *) continue ;; esac
            [ $efirst = 1 ] && efirst=0 || printf ','
            printf '{"from":%s,"to":%s}' "$(jstr "$uid")" "$(jstr "$d")"
        done
        IFS="$oldIFS"
    done < "$inv"
fi
printf ']'

# ---- testing requirement marks ---------------------------------------------------
comma
printf '"testingMarks":['
tfirst=1
for gdir_src in "$plan_dir"/*/; do :; done
while IFS= read -r gdir; do
    [ -f "$gdir/goal.md" ] || continue
    gid="$(basename "$gdir")"
    grep -qE '^\|.*\| *yes *\|' "$gdir/goal.md" || continue
    while IFS= read -r sfile; do
        base="$(basename "$sfile" .md)"
        [ -f "${sfile%.md}-testing.md" ] && continue
        [ $tfirst = 1 ] && tfirst=0 || printf ','
        printf '{"goal":%s,"step":%s}' "$(jstr "$gid")" "$(jstr "$base")"
    done < <(find "$gdir/steps" -maxdepth 1 -name '*.md' ! -name '*-testing.md' 2>/dev/null | sort)
done < <(find "$plan_dir" -mindepth 1 -maxdepth 1 -type d | sort)
printf ']'

# ---- coverage rows ----------------------------------------------------------------
comma
printf '"coverage":['
cfirst=1
if [ -f "$inv" ]; then
    in_cov=0
    while IFS= read -r line; do
        case "$line" in '## Definition-of-done coverage') in_cov=1; continue ;; esac
        case "$line" in '## '*) in_cov=0; continue ;; esac
        [ "$in_cov" = 1 ] || continue
        case "$line" in '| '*) ;; *) continue ;; esac
        case "$line" in '|---'*) continue ;; esac
        case "$line" in '| Required outcome'*) continue ;; esac
c_out="$(plan_table_cell "$line" 2)"
c_units="$(plan_table_cell "$line" 3)"
        [ -n "$c_out" ] || continue
        [ $cfirst = 1 ] && cfirst=0 || printf ','
        printf '{"outcome":%s,"units":%s}' "$(jstr "$c_out")" "$(jstr "$c_units")"
    done < "$inv"
fi
printf ']'

# ---- findings (live table) + archived rounds ---------------------------------------
comma
printf '"findings":['
ffirst=1
rev="$plan_dir/adversarial-review.md"
if [ -f "$rev" ]; then
    while IFS= read -r line; do
        case "$line" in '| AR'*) ;; *) continue ;; esac
fid="$(plan_table_cell "$line" 2)"
item="$(plan_table_cell "$line" 3)"
change="$(plan_table_cell "$line" 4)"
status="$(plan_table_cell "$line" 5)"
wu="$(plan_table_cell "$line" 6)"
        cycle="$(sed -n '/^## Cycle [0-9]/,$p' "$plan_dir/adversarial-review-history.md" 2>/dev/null | grep -qF "| $fid |" && echo archived || echo current)"
        [ $ffirst = 1 ] && ffirst=0 || printf ','
        printf '{"id":%s,"item":%s,"change":%s,"status":%s,"workUnit":%s,"cycle":%s}' \
            "$(jstr "$fid")" "$(jstr "$item")" "$(jstr "$change")" "$(jstr "$status")" "$(jstr "$wu")" "$(jstr "$cycle")"
    done < <(awk '/^## Findings$/{f=1;next} /^## Verdict$/{f=0} f' "$rev")
fi
printf ']'

cycles=0
[ -f "$plan_dir/adversarial-review-history.md" ] && cycles="$(grep -c '^## Cycle [0-9]' "$plan_dir/adversarial-review-history.md" || true)"
comma
# generatedAt is the revision stamp the served page swaps /sections by
# (T43b); OVERVIEW_NOW pins it for deterministic tests, like the renderer.
printf '"cycles":%s,"reviewTarget":2,"generatedAt":"%s","generatedBy":"overview-state.sh"' \
    "$cycles" "${OVERVIEW_NOW:-serve-live}"

printf '}'
