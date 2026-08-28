#!/usr/bin/env bash
# MODE: PROD
# GENERATED FILE — do not edit. Compiled from scripts/lib/progress/*.sh by:
#   planning/scripts/build-plan-libs.sh
# Edit the function file in that directory, then re-run the build.
# Target: prod
#
# progress arithmetic and the status glyphs

set -euo pipefail

[ -z "${PLAN_PROGRESS_LIB_LOADED:-}" ] || return 0
PLAN_PROGRESS_LIB_LOADED=1

# plan_count_progress_rows FILE STATUS_CELL — print "completed total" for a
# progress tracker table: every data row counts, rows whose status cell
# contains "completed" count twice. Header (Goalname) and dash separator rows
# never count. Top-level by necessity, not style: bash 3.2 cannot parse a
# `case` inside $( ) or < <( ) (see PORTABILITY.md, case-in-substitution), so
# call sites keep only this function's invocation inside the substitution —
# read -r a b < <(plan_count_progress_rows "$file" 5).
plan_count_progress_rows() {
    [ -n "${1:-}" ] || { printf 'plan_count_progress_rows: file required\n' >&2; return 1; }
    [ -f "$1" ] || { printf 'plan_count_progress_rows: no such file: %s\n' "$1" >&2; return 1; }
    local cfile="$1" status_cell="${2:-5}"
    local completed=0 total=0 prow pgoal pstatus
    while IFS= read -r prow || [ -n "$prow" ]; do
        case "$prow" in '|'*) ;; *) continue ;; esac
        pgoal="$(plan_table_cell "$prow" 2)"
        pstatus="$(plan_table_cell "$prow" "$status_cell")"
        case "$pgoal" in Goalname) continue ;; esac
        [[ $pgoal =~ ^-+$ ]] && continue
        [[ $pstatus =~ ^-+$ ]] && continue
        total=$((total + 1))
        case "$pstatus" in *completed*) completed=$((completed + 1)) ;; esac
    done < "$cfile"
    printf '%s %s\n' "$completed" "$total"
}

plan_emit_step_testing_reminder() {
    local plan_dir="$1" document_id="$2" step_file goal_dir goal_file required companion
    case "$document_id" in
        step:*|unit:*) ;;
        *) return 0 ;;
    esac
    step_file="$(plan_document_path "$plan_dir" "$document_id")"
    goal_dir="$(dirname "$(dirname "$step_file")")"
    goal_file="$goal_dir/goal.md"
    [ -f "$goal_file" ] || return 0
    required="$(plan_testing_requirement_for_goal "$goal_file")"
    [ "$required" = yes ] || return 0
    companion="${step_file%.md}-testing.md"
    if [ ! -f "$companion" ]; then
        printf 'Reminder: this goal requires testing; continue with its test/proof step before marking the goal complete.\n' >&2
        return 0
    fi
    plan_companion_is_behind "$plan_dir" "$step_file" "$companion" || return 0
    printf 'Reminder: %s was already behind %s before this edit; review it for accuracy and completeness.\n' \
        "${companion##*/}" "${step_file##*/}" >&2
}

# plan_companion_is_behind PLAN_DIR STEP COMPANION -- true when the companion
# was already older than the step BEFORE the current call wrote anything.
#
# mtime cannot answer this: the reminder runs after the write, so the step is
# always the newer file and the check would fire on every edit -- which is what
# made the old unconditional line worthless (T67). Every mutating helper commits
# the pre-mutation state first, so HEAD is the tree as it stood before this call
# and git can answer it exactly. A plan with no usable history stays silent: an
# always-firing reminder carries no information, so under-reporting beats it.
plan_companion_is_behind() {
    local plan_dir="$1" step="$2" companion="$3" step_at companion_at
    git -C "$plan_dir" rev-parse --git-dir >/dev/null 2>&1 || return 1
    step_at="$(git -C "$plan_dir" log -1 --format=%ct -- "${step#"$plan_dir"/}" 2>/dev/null)"
    companion_at="$(git -C "$plan_dir" log -1 --format=%ct -- "${companion#"$plan_dir"/}" 2>/dev/null)"
    [ -n "$step_at" ] && [ -n "$companion_at" ] || return 1
    [ "$companion_at" -lt "$step_at" ]
}

plan_progress_bar() {
    local completed="$1" total="$2" width="${3:-20}" percent filled empty
    percent="$(plan_progress_percent "$completed" "$total")"
    filled=$(( percent * width / 100 ))
    empty=$(( width - filled ))
    printf '%s%s\n' \
        "$(printf '%*s' "$filled" '' | tr ' ' '#')" \
        "$(printf '%*s' "$empty" '' | tr ' ' '-')"
}

# Status glyph: nothing started, something started, everything done. Written as
# `if` blocks rather than the call sites' `[ … ] && icon=…` chain, which returns
# non-zero under `set -e` when the test fails.
plan_progress_icon() {
    local completed="$1" percent="$2" icon='💤'
    if [ "$completed" -gt 0 ]; then
        icon='⏳'
    fi
    if [ "$percent" -eq 100 ]; then
        icon='✅'
    fi
    printf '%s\n' "$icon"
}

# ── Progress rendering ───────────────────────────────────────────────────────
# Half-up rounding (+ total / 2) and the 20-column default width are contract:
# every caller must render byte-identical output.
plan_progress_percent() {
    local completed="$1" total="$2"
    if [ "$total" -gt 0 ]; then
        printf '%s\n' "$(( (completed * 100 + total / 2) / total ))"
    else
        printf '0\n'
    fi
}

# The label a progress table's Completion status cell carries, from the status
# word its command was given. Non-zero on an unknown word, so the caller keeps
# owning the usage message. The glyphs are the on-disk contract.
plan_status_label() {
    case "$1" in
        incomplete) printf '%s\n' '💤 incomplete' ;;
        in-progress|in_progress) printf '%s\n' '⏳ in progress' ;;
        completed) printf '%s\n' '✅ completed' ;;
        *) return 1 ;;
    esac
}

# Derive a row description from a step's Objective paragraph: the text after
# the first "§ N.N" label inside "## Objective", truncated to 100 chars. Falls
# back to "$2" so a progress table never carries a literal placeholder.
plan_step_objective() {
    local step_file="$1" fallback="$2"
    local desc
    desc="$(awk '
        /^## Objective$/ { in_obj = 1; next }
        /^§ [0-9]+\.[0-9]+$/ && in_obj { after_label = 1; next }
        after_label && NF {
            line = $0; sub(/^[[:space:]]+/, "", line)
            if (length(line) > 100) line = substr(line, 1, 100) "..."
            print line; exit
        }
        /^## / && in_obj { exit }
    ' "$step_file" 2>/dev/null)"
    [ -n "$desc" ] || desc="$fallback"
    printf '%s\n' "$desc"
}
