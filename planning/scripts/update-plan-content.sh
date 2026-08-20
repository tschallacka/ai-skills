#!/usr/bin/env bash
# MODE: PROD
# update-plan-content.sh — edit the numbered prose of every plan document, and
# gate the review approval.
#
# Two jobs live here, and the second one is not obvious from the name:
#
#   1. Content editor. One flag per (document, granularity) pair rewrites a
#      titled paragraph, a whole section, a field, a table, or the decomposition
#      checkbox in plan-description.md, a goal.md, a step file, or
#      adversarial-review.md.
#   2. Approval gate. `--review-status <plan> approved` is the only path that
#      flips a review to approved. It refuses while any finding is still open,
#      verifies the fix keys through verify-fix-keys.sh as `${CLAIMED_BY:-<the
#      minting session>}`, and then destroys the session secret so the same keys
#      cannot be replayed. Export CLAIMED_BY with the fixer session that wrote
#      fixes.md: the default is the minting session, which the fix-key gate
#      refuses as self-certification. See the "Approval gate" banner below; do
#      not move that logic elsewhere.
#
# Usage:
#   update-plan-content.sh <flag> [--plan-dir] <plan-directory> [args…]     (see --help)
#   update-plan-content.sh --help
#
# Sections in order:
#   Usage and flag translation — the outer flag map, which rewrites "$@" into an
#     internal command plus canonical positionals.
#   Paragraph argument parsing — the repeated `-p N.N: content` form.
#   Command dispatch — one arm per internal command, including Approval gate.
#   Context invalidation — the post-mutation handoff marker.
#
# Exit codes: 64 bad invocation, 65 the document is in an unusable state,
# 66 a required plan file is missing.

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Usage and flag translation
# ─────────────────────────────────────────────────────────────────────────────

usage() {
    local rc="${1:-64}"
    cat <<'USAGE'
Usage:
  update-plan-content.sh -dp|--description-paragraph [--plan-dir] <plan-directory> <N.N> <text>
                                             ONE paragraph; replaces it. Extra -p flags are an error.
  update-plan-content.sh -ds|--description-section [--plan-dir] <plan-directory> <section-id> -p N.1: <content> [-p N.2: <content> ...]
                                             WHOLE section; paragraphs must be sequential from N.1.
  update-plan-content.sh -gp|--goal-paragraph [--plan-dir] <plan-directory> <goal-name> <N.N> <text>
                                             ONE paragraph; replaces it. Extra -p flags are an error.
  update-plan-content.sh -gs|--goal-section [--plan-dir] <plan-directory> <goal-name> <section-id> -p N.1: <content> [-p N.2: <content> ...]
                                             WHOLE section; paragraphs must be sequential from N.1.
  update-plan-content.sh -sp|--step-paragraph [--plan-dir] <plan-directory> <goal>/<step> <N.N> <text>
                                             ONE paragraph; replaces it. Extra -p flags are an error.
  update-plan-content.sh -ss|--step-section [--plan-dir] <plan-directory> <goal>/<step> <section-id> -p N.1: <content> [-p N.2: <content> ...]
                                             WHOLE section; paragraphs must be sequential from N.1.
  update-plan-content.sh -rp|--review-paragraph [--plan-dir] <plan-directory> <N.N> <text>
                                             ONE paragraph; replaces it. Extra -p flags are an error.
  update-plan-content.sh -rs|--review-section [--plan-dir] <plan-directory> <section-id> -p N.1: <content> [-p N.2: <content> ...]
                                             WHOLE section; paragraphs must be sequential from N.1.
  update-plan-content.sh -ap|--append-paragraph [--plan-dir] <plan-directory> <document-id> <section-id> <text>
                                             Appends one paragraph with the next free number in the section.
  update-plan-content.sh -tp|--table-paragraph [--plan-dir] <plan-directory> <document-id> <N.N> <columns> <CSV>
  update-plan-content.sh -ia|--insert-after [--plan-dir] <plan-directory> <document-id> <N.N> <text>
  update-plan-content.sh -ib|--insert-before [--plan-dir] <plan-directory> <document-id> <N.N> <text>
  update-plan-content.sh --delete-paragraph [--plan-dir] <plan-directory> <document-id> <N.N>
                                              Deletes ONE paragraph and renumbers the
                                              following paragraphs in the same section.
  update-plan-content.sh -t|--title [--plan-dir] <plan-directory> <document-id> <title>
  update-plan-content.sh -f|--field [--plan-dir] <plan-directory> <document-id> <field-label> <value>
  update-plan-content.sh -tr|--testing-requirement [--plan-dir] <plan-directory> <goal-name> <yes|no> <rationale>
  update-plan-content.sh -rv|--review-status [--plan-dir] <plan-directory> <pending|approved>
  update-plan-content.sh -dr|--decomposition-review [--plan-dir] <plan-directory> <incomplete|completed>

Document IDs: plan, review, goal:<goal>, step:<goal>/<step>, or unit:<WNN>.
USAGE
    exit "$rc"
}

[ "$#" -ge 1 ] || usage
case "$1" in
    -h|--help) usage 0 ;;
esac
command="$1"; shift
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"

# `-p 2.1: text` captures everything after the colon, so the conventional space
# after it became a leading space in the written paragraph -- in every document,
# for every writer. A document is read by people, so the stray indent is a real
# defect and not only a cosmetic one.
trim_leading_space() {
    local text="$1"
    printf '%s' "${text#"${text%%[![:space:]]*}"}"
}

# The subcommand was shifted off above, so the plan directory is $1 for
# every form. Accept --plan-dir as a synonym for it.
eval "set -- $(plan_hoist_plan_dir 1 "$@")"
# Sourced unconditionally: a missing library is a broken install, not an
# optional feature (CODE-STYLE §7). plan-context-lib.sh is in
# PACKAGE-MANIFEST.txt and install.sh's skill_files(), so it is always present.
source "$script_dir/plan-context-lib.sh"

[[ "$command" == -* ]] || usage

normalize_flagged_paragraph() {
    local paragraph_id="$1"
    [[ "$paragraph_id" =~ ^[0-9]+\.[0-9]+:?$ ]] || plan_die "Paragraph must use N.N or N.N:"
    printf '%s\n' "${paragraph_id%:}:"
}

reject_swallowed_flags() {
    local content="$1" flag_form="$2" section_form
    case "$flag_form" in -ia|-ib) section_form='' ;; *) section_form="${flag_form%p}s" ;; esac
    if [[ "$content" =~ (^|[[:space:]])-(p|dp|gp|sp|rp|tp|ia|ib)[[:space:]]+[0-9]+\.[0-9]+[[:space:]]*: ]]; then
        plan_die "Content for $flag_form absorbed flag-shaped text ('-p N.N:' style); $flag_form takes exactly one paragraph and no flags. Use the section form instead:
  update-plan-content.sh ${section_form:-$flag_form} [--plan-dir] <plan-directory> ... -p N.1: '...' -p N.2: '...'
Section form requires sequential paragraphs starting at N.1."
    fi
}

# Flag translation: each arm rebuilds "$@" with `set --` into the one canonical
# positional order the dispatch below expects and rewrites $command to the
# internal name, so no dispatch arm has to know a user-facing spelling.
# ---- quoted: -gp translation ----
# -gp <plan> <goal> 4.1 text
# paragraph <plan> goal:<goal> -p 4.1:text
# ---- end quoted ----
if [[ "$command" == -* ]]; then
    case "$command" in
        -dp|--description-paragraph)
            [ "$#" -ge 3 ] || usage
            plan_dir="$1"; paragraph_id="$2"; shift 2
            paragraph_content="$*"
            reject_swallowed_flags "$paragraph_content" "$command"
            set -- "$plan_dir" plan -p "$(normalize_flagged_paragraph "$paragraph_id")$paragraph_content"
            command=paragraph
            ;;
        -ds|--description-section)
            [ "$#" -ge 3 ] || usage
            plan_dir="$1"; section="$2"; shift 2
            set -- "$plan_dir" plan "$section" "$@"
            command=section
            ;;
        -gp|--goal-paragraph)
            [ "$#" -ge 4 ] || usage
            plan_dir="$1"; goal_name="$2"; paragraph_id="$3"; shift 3
            paragraph_content="$*"
            reject_swallowed_flags "$paragraph_content" "$command"
            set -- "$plan_dir" "goal:$goal_name" -p "$(normalize_flagged_paragraph "$paragraph_id")$paragraph_content"
            command=paragraph
            ;;
        -gs|--goal-section)
            [ "$#" -ge 4 ] || usage
            plan_dir="$1"; goal_name="$2"; section="$3"; shift 3
            set -- "$plan_dir" "goal:$goal_name" "$section" "$@"
            command=section
            ;;
        -sp|--step-paragraph)
            [ "$#" -ge 4 ] || usage
            plan_dir="$1"; step_id="$2"; paragraph_id="$3"; shift 3
            paragraph_content="$*"
            reject_swallowed_flags "$paragraph_content" "$command"
            set -- "$plan_dir" "step:$step_id" -p "$(normalize_flagged_paragraph "$paragraph_id")$paragraph_content"
            command=paragraph
            ;;
        -ss|--step-section)
            [ "$#" -ge 4 ] || usage
            plan_dir="$1"; step_id="$2"; section="$3"; shift 3
            set -- "$plan_dir" "step:$step_id" "$section" "$@"
            command=section
            ;;
        -rp|--review-paragraph)
            [ "$#" -ge 3 ] || usage
            plan_dir="$1"; paragraph_id="$2"; shift 2
            paragraph_content="$*"
            reject_swallowed_flags "$paragraph_content" "$command"
            set -- "$plan_dir" adversarial-review -p "$(normalize_flagged_paragraph "$paragraph_id")$paragraph_content"
            command=paragraph
            ;;
        -rs|--review-section)
            [ "$#" -ge 3 ] || usage
            plan_dir="$1"; section="$2"; shift 2
            set -- "$plan_dir" adversarial-review "$section" "$@"
            command=section
            ;;
        -ap|--append-paragraph)
            [ "$#" -eq 4 ] || usage
            set -- "$1" "$2" "$3" "$4"
            command=append-paragraph
            ;;
        -tp|--table-paragraph)
            [ "$#" -eq 5 ] || { printf 'update-plan-content.sh: --table-paragraph requires exactly [--plan-dir] <plan-directory> <document-id> <N.N> <columns> <CSV>\n' >&2; exit 64; }
            set -- "$1" "$2" "$3" "$4" "$5"
            command=table-paragraph
            ;;
        -ia|--insert-after)
            [ "$#" -ge 4 ] || usage
            plan_dir="$1"; document_id="$2"; paragraph_id="$3"; shift 3
            paragraph_content="$*"
            reject_swallowed_flags "$paragraph_content" "$command"
            set -- "$plan_dir" "$document_id" "${paragraph_id%:}" "$paragraph_content"
            command=insert-after
            ;;
        -ib|--insert-before)
            [ "$#" -ge 4 ] || usage
            plan_dir="$1"; document_id="$2"; paragraph_id="$3"; shift 3
            paragraph_content="$*"
            reject_swallowed_flags "$paragraph_content" "$command"
            set -- "$plan_dir" "$document_id" "${paragraph_id%:}" "$paragraph_content"
            command=insert-before
            ;;
        --delete-paragraph)
            [ "$#" -eq 3 ] || usage
            set -- "$1" "$2" "$3"
            command=delete-paragraph
            ;;
        -t|--title)
            [ "$#" -eq 3 ] || { printf 'update-plan-content.sh: --title requires exactly [--plan-dir] <plan-directory> <document-id> <title>\n' >&2; exit 64; }
            set -- "$1" "$2" "$3"
            command=title
            ;;
        -f|--field)
            [ "$#" -eq 4 ] || { printf 'update-plan-content.sh: --field requires exactly [--plan-dir] <plan-directory> <document-id> <field-label> <value>\n' >&2; exit 64; }
            set -- "$1" "$2" "$3" "$4"
            command=field
            ;;
        -tr|--testing-requirement)
            [ "$#" -eq 4 ] || usage
            set -- "$1" "$2" "$3" "$4"
            command=testing-requirement
            ;;
        -rv|--review-status)
            [ "$#" -eq 2 ] || usage
            set -- "$1" "$2"
            command=review-status
            ;;
        -dr|--decomposition-review)
            [ "$#" -eq 2 ] || usage
            set -- "$1" "$2"
            command=decomposition-review
            ;;
        *) usage ;;
    esac
fi

# Three `-p N.N: content` loops exist and are not interchangeable: the outer one
# walks -p groups, the inner joins bare words into one paragraph, and the
# `paragraph` dispatch arm's copy additionally rejects a second -p.

parse_paragraph_arguments() {
    local section_number="$1" paragraph_spec paragraph_content paragraph_section paragraph_number
    shift
    paragraph_records=()
    [ "$#" -gt 0 ] || plan_die "Paragraphs must be supplied with repeated -p N.N: content arguments"
    paragraph_index=1
    while [ "$#" -gt 0 ]; do
        [ "$1" = '-p' ] || plan_die "Expected -p before paragraph $paragraph_index"
        shift
        [ "$#" -gt 0 ] || plan_die "Missing paragraph label after -p"
        paragraph_spec="$1"
        shift
        [[ "$paragraph_spec" =~ ^([0-9]+)\.([0-9]+):(.*)$ ]] || plan_die "Paragraph must use N.N: content, for example 2.1: First paragraph"
        paragraph_section="${BASH_REMATCH[1]}"
        paragraph_number="${BASH_REMATCH[2]}"
        paragraph_content="$(trim_leading_space "${BASH_REMATCH[3]}")"
        [ "$paragraph_section" = "$section_number" ] || plan_die "Paragraph $paragraph_section.$paragraph_number belongs to section $paragraph_section, expected section $section_number"
        [ "$paragraph_number" -eq "$paragraph_index" ] || plan_die "Paragraphs must be sequential, starting at $section_number.1"
        if [ -z "$paragraph_content" ]; then
            [ "$#" -gt 0 ] && [ "$1" != '-p' ] || plan_die "Missing content for paragraph $section_number.$paragraph_number"
            paragraph_content="$1"
            shift
            while [ "$#" -gt 0 ] && [ "$1" != '-p' ]; do
                paragraph_content="$paragraph_content $1"
                shift
            done
        fi
        [[ "$paragraph_content" != *$'\n'* && "$paragraph_content" != *$'\r'* ]] || plan_die "Paragraph $section_number.$paragraph_number must be one line"
        [[ "$paragraph_content" != *'§'* ]] || plan_die "Paragraph content must not contain the reserved paragraph marker §"
        [ -n "${paragraph_content//[[:space:]]/}" ] || plan_die "Paragraph $section_number.$paragraph_number has empty content"
        paragraph_records+=("$section_number.$paragraph_number"$'\t'"$paragraph_content")
        paragraph_index=$((paragraph_index + 1))
    done
}

render_paragraph_arguments() {
    local index record paragraph_id paragraph_content
    for index in "${!paragraph_records[@]}"; do
        record="${paragraph_records[$index]}"
        paragraph_id="${record%%$'\t'*}"
        paragraph_content="${record#*$'\t'}"
        printf '§ %s\n%s\n' "$paragraph_id" "$paragraph_content"
        [ "$index" -eq $((${#paragraph_records[@]} - 1)) ] || printf '\n'
    done
}

# ─────────────────────────────────────────────────────────────────────────────
# Command dispatch
# ─────────────────────────────────────────────────────────────────────────────

case "$command" in
    title)
        [ "$#" -eq 3 ] || usage
        plan_dir="$1"; document_id="$2"; title="$3"
        plan_require_directory "$plan_dir"
        plan_git_snapshot "$plan_dir"
        file="$(plan_document_path "$plan_dir" "$document_id")"
        [ -f "$file" ] || plan_die "Document not found: $file" 66
        plan_replace_title "$file" "$title"
        plan_emit_step_testing_reminder "$plan_dir" "$document_id"
        ;;
    section)
        [ "$#" -ge 4 ] || usage
        plan_dir="$1"; document_id="$2"; section="$3"; shift 3
        plan_require_directory "$plan_dir"
        plan_git_snapshot "$plan_dir"
        file="$(plan_document_path "$plan_dir" "$document_id")"
        [ -f "$file" ] || plan_die "Document not found: $file" 66
        IFS=$'\t' read -r heading number < <(plan_section_spec "$(plan_document_kind "$document_id")" "$section")
        # No `trap - EXIT` release: it would discard the library's cleanup
        # handler too (CODE-STYLE §8).
        body_file="$(mktemp "${TMPDIR:-/tmp}/plan-section.XXXXXX")"
        trap 'rm -f "$body_file"' EXIT
        parse_paragraph_arguments "$number" "$@"
        render_paragraph_arguments > "$body_file"
        plan_replace_section "$file" "$heading" "$body_file"
        rm -f "$body_file"
        plan_emit_step_testing_reminder "$plan_dir" "$document_id"
        ;;
    paragraph)
        [ "$#" -ge 4 ] || usage
        plan_dir="$1"; document_id="$2"; shift 2
        plan_require_directory "$plan_dir"
        plan_git_snapshot "$plan_dir"
        file="$(plan_document_path "$plan_dir" "$document_id")"
        [ -f "$file" ] || plan_die "Document not found: $file" 66
        [ "$1" = '-p' ] || plan_die "Paragraph replacement requires -p N.N: content"
        shift
        [ "$#" -gt 0 ] || plan_die "Missing paragraph after -p"
        paragraph_spec="$1"; shift
        [[ "$paragraph_spec" =~ ^([0-9]+)\.([0-9]+):(.*)$ ]] || plan_die "Paragraph must use N.N: content, for example 2.1: First paragraph"
        paragraph_id="§ ${BASH_REMATCH[1]}.${BASH_REMATCH[2]}"
        paragraph_content="$(trim_leading_space "${BASH_REMATCH[3]}")"
        if [ -z "$paragraph_content" ]; then
            [ "$#" -gt 0 ] && [ "$1" != '-p' ] || plan_die "Missing content for paragraph ${paragraph_id#§ }"
            paragraph_content="$1"; shift
            while [ "$#" -gt 0 ]; do
                [ "$1" != '-p' ] || plan_die "Paragraph replacement accepts exactly one -p argument"
                paragraph_content="$paragraph_content $1"; shift
            done
        fi
        [ "$#" -eq 0 ] || plan_die "Paragraph replacement accepts exactly one -p argument"
        [[ "$paragraph_content" != *$'\n'* && "$paragraph_content" != *$'\r'* ]] || plan_die "Paragraph content must be one line"
        [[ "$paragraph_content" != *'§'* ]] || plan_die "Paragraph content must not contain the reserved paragraph marker §"
        [ -n "${paragraph_content//[[:space:]]/}" ] || plan_die "Paragraph content must not be empty"
        section_num="${BASH_REMATCH[1]}"
        para_num="${BASH_REMATCH[2]}"
        para_count="$(grep -cFx -- "$paragraph_id" "$file" || true)"
        if [ "$para_count" -eq 1 ]; then
            plan_replace_paragraph "$file" "$paragraph_id" "$paragraph_content"
        elif [ "$para_count" -eq 0 ]; then
            # Auto-create the next sequential paragraph only when the section's
            # labels are contiguous 1..max with no trailing unlabeled content:
            # a sparse section means it was never authored, not that it is next.
            max_num="$(awk -v s="$section_num" '$0 ~ "^§ " s "\\.[0-9]+$" { split($0, a, "."); n = a[2] + 0; if (n > m) m = n } END { print m + 0 }' "$file")"
            label_count="$(grep -cE -- "^§ $section_num\\.[0-9]+$" "$file" || true)"
            trailing="$(awk -v s="$section_num" '
                /^§ [0-9]+\.[0-9]+$/ {
                    sec = $2; sub(/\..*/, "", sec)
                    if (sec == s) { last = NR; body_seen = 0; blank = 0 }
                    else { last = 0; blank = 0 }
                    next
                }
                /^## / { next }
                NF == 0 { if (last) blank = 1; next }
                last && NR > last {
                    if (body_seen || blank) { print "unlabeled"; exit }
                    body_seen = 1
                }
            ' "$file")"
            if [ "$max_num" -ge 1 ] && [ "$label_count" -eq "$max_num" ] && [ -z "$trailing" ] && [ "$para_num" -eq $((max_num + 1)) ]; then
                body_file="$(mktemp "${TMPDIR:-/tmp}/plan-auto-paragraph.XXXXXX")"
                trap 'rm -f "$body_file"' EXIT
                printf '%s\n' "$paragraph_content" > "$body_file"
                plan_insert_paragraph "$file" "§ $section_num.$max_num" after "$body_file"
                rm -f "$body_file"
                printf 'update-plan-content: added paragraph § %s.%s (auto-created; verify no duplication)\n' "$section_num" "$para_num" >&2
                printf 'update-plan-content: section now reads:\n' >&2
                awk -v s="$section_num" '
                    /^§ [0-9]+\.[0-9]+$/ { sec = $2; sub(/\..*/, "", sec); if (sec == s) { print "  " $0; show = 1 } else show = 0; next }
                    show && NF { print "  " $0; show = 0 }
                ' "$file" >&2
            else
                existing="$(awk -v s="$section_num" '$0 ~ "^§ " s "\\.[0-9]+$" { print $2 }' "$file" | tr '\n' ' ')"
                plan_die "Paragraph § $section_num.$para_num not found; section $section_num currently has: ${existing:-none}. Use -dp with an existing paragraph to replace it, or -ds <section> -p N.N: to add sequential paragraphs."
            fi
        else
            plan_replace_paragraph "$file" "$paragraph_id" "$paragraph_content"
        fi
        plan_emit_step_testing_reminder "$plan_dir" "$document_id"
        ;;
    append-paragraph)
        [ "$#" -eq 4 ] || usage
        plan_dir="$1"; document_id="$2"; section="$3"; paragraph_content="$4"
        plan_require_directory "$plan_dir"
        plan_git_snapshot "$plan_dir"
        file="$(plan_document_path "$plan_dir" "$document_id")"
        [ -f "$file" ] || plan_die "Document not found: $file" 66
        IFS=$'\t' read -r heading number < <(plan_section_spec "$(plan_document_kind "$document_id")" "$section")
        [[ "$paragraph_content" != *$'\n'* && "$paragraph_content" != *$'\r'* ]] || plan_die "Paragraph content must be one line"
        [[ "$paragraph_content" != *'§'* ]] || plan_die "Paragraph content must not contain the reserved paragraph marker §"
        [ -n "${paragraph_content//[[:space:]]/}" ] || plan_die "Paragraph content must not be empty"
        max_num="$(awk -v n="$number" '$0 ~ "^§ " n "\\.[0-9]+$" { split($2, a, "."); m = a[2] + 0; if (m > max) max = m } END { print max + 0 }' "$file")"
        [ "$max_num" -ge 1 ] || plan_die "Section '$section' has no numbered paragraphs; author it with the section form (-ds/-gs/-ss/-rs) from N.1"
        body_file="$(mktemp "${TMPDIR:-/tmp}/plan-append-paragraph.XXXXXX")"
        trap 'rm -f "$body_file"' EXIT
        printf '%s\n' "$paragraph_content" > "$body_file"
        plan_insert_paragraph "$file" "§ $number.$max_num" after "$body_file"
        rm -f "$body_file"
        printf 'update-plan-content: appended paragraph § %s.%s after § %s.%s\n' "$number" "$((max_num + 1))" "$number" "$max_num" >&2
        plan_emit_step_testing_reminder "$plan_dir" "$document_id"
        ;;
    table-paragraph)
        [ "$#" -eq 5 ] || usage
        plan_dir="$1"; document_id="$2"; paragraph_id="$3"; columns="$4"; csv="$5"
        plan_require_directory "$plan_dir"
        plan_git_snapshot "$plan_dir"
        file="$(plan_document_path "$plan_dir" "$document_id")"
        [ -f "$file" ] || plan_die "Document not found: $file" 66
        [[ "$paragraph_id" =~ ^[0-9]+\.[0-9]+$ ]] || plan_die "Paragraph must use N.N"
        table_file="$(mktemp "${TMPDIR:-/tmp}/plan-table-paragraph.XXXXXX")"
        trap 'rm -f "$table_file"' EXIT
        plan_render_csv_table "$columns" "$csv" > "$table_file"
        table_content="$(cat "$table_file")"
        plan_replace_paragraph "$file" "§ $paragraph_id" "$table_content"
        rm -f "$table_file"
        plan_emit_step_testing_reminder "$plan_dir" "$document_id"
        ;;
    insert-after|insert-before)
        [ "$#" -eq 4 ] || usage
        plan_dir="$1"; document_id="$2"; paragraph_id="$3"; paragraph_content="$4"
        plan_require_directory "$plan_dir"
        plan_git_snapshot "$plan_dir"
        file="$(plan_document_path "$plan_dir" "$document_id")"
        [ -f "$file" ] || plan_die "Document not found: $file" 66
        [[ "$paragraph_id" =~ ^[0-9]+\.[0-9]+$ ]] || plan_die "Paragraph must use N.N"
        [[ "$paragraph_content" != *$'\n'* && "$paragraph_content" != *$'\r'* ]] || plan_die "Inserted paragraph must be one line"
        [[ "$paragraph_content" != *'§'* ]] || plan_die "Paragraph content must not contain the reserved paragraph marker §"
        [ -n "${paragraph_content//[[:space:]]/}" ] || plan_die "Inserted paragraph must not be empty"
        insert_file="$(mktemp "${TMPDIR:-/tmp}/plan-insert-paragraph.XXXXXX")"
        trap 'rm -f "$insert_file"' EXIT
        printf '%s\n' "$paragraph_content" > "$insert_file"
        plan_insert_paragraph "$file" "§ $paragraph_id" "${command#insert-}" "$insert_file"
        rm -f "$insert_file"
        plan_emit_step_testing_reminder "$plan_dir" "$document_id"
        ;;
    delete-paragraph)
        [ "$#" -eq 3 ] || usage
        plan_dir="$1"; document_id="$2"; paragraph_id="$3"
        plan_require_directory "$plan_dir"
        plan_git_snapshot "$plan_dir"
        file="$(plan_document_path "$plan_dir" "$document_id")"
        [ -f "$file" ] || plan_die "Document not found: $file" 66
        [[ "$paragraph_id" =~ ^[0-9]+\.[0-9]+$ ]] || plan_die "Paragraph must use N.N"
        plan_delete_paragraph "$file" "§ $paragraph_id"
        plan_emit_step_testing_reminder "$plan_dir" "$document_id"
        ;;
    field)
        [ "$#" -eq 4 ] || usage
        plan_dir="$1"; document_id="$2"; label="$3"; value="$4"
        plan_require_directory "$plan_dir"
        plan_git_snapshot "$plan_dir"
        file="$(plan_document_path "$plan_dir" "$document_id")"
        [ -f "$file" ] || plan_die "Document not found: $file" 66
        plan_replace_field "$file" "$label" "$value"
        plan_emit_step_testing_reminder "$plan_dir" "$document_id"
        ;;
    testing-requirement)
        [ "$#" -eq 4 ] || usage
        plan_dir="$1"; goal_name="$2"; required="$3"; rationale="$4"
        plan_require_directory "$plan_dir"
        [[ "$goal_name" =~ ^[0-9][0-9]-[a-z0-9-]+$ ]] || plan_die "Goal name must use 01-kebab-case"
        goal_file="$plan_dir/$goal_name/goal.md"
        [ -f "$goal_file" ] || plan_die "Goal document not found: $goal_file"
        plan_git_snapshot "$plan_dir"
        plan_replace_testing_requirement "$goal_file" "$required" "$rationale"
        ;;
    # The only path that flips a review to approved: it refuses while a finding
    # is open, re-verifies the fix keys, then destroys the session secret so
    # minted keys cannot be replayed. A second entry point is a second gate.
    review-status)
        [ "$#" -eq 2 ] || usage
        plan_dir="$1"; requested_status="$2"
        plan_require_directory "$plan_dir"
        plan_git_snapshot "$plan_dir"
        review="$plan_dir/adversarial-review.md"; description="$plan_dir/plan-description.md"
        [ -f "$review" ] && [ -f "$description" ] || plan_die "Both plan-description.md and adversarial-review.md are required"
        invalidate_session=""
        case "$requested_status" in
            pending) review_status='`💤 pending`'; description_status='💤 pending' ;;
            approved)
                if grep -Eq '^\|[[:space:]]*AR-[0-9]+[[:space:]]*\|.*\|[[:space:]]*(💤 open|⏳ in progress)[[:space:]]*\|' "$review"; then
                    plan_die "Cannot approve a review with unresolved findings"
                fi
                if [ -f "$plan_dir/fix-keys.json" ]; then
                    session_id="$(sed -n 's/.*"session_id"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
                        "$plan_dir/fix-keys.json" | head -1)"
                    [ -n "$session_id" ] || plan_die "fix-keys.json has no session_id"
                    # An unnamed claimant is the minting session, so the default
                    # is self-certification and the gate refuses it. CLAIMED_BY
                    # names the fixer session that recorded fixes.md.
                    if ! verify_output="$("$script_dir/verify-fix-keys.sh" "$plan_dir" \
                        --claimed-by "${CLAIMED_BY:-$session_id}" 2>&1)"; then
                        plan_die "Cannot approve: fix-keys verification failed: $verify_output"
                    fi
                    invalidate_session="$session_id"
                fi
                review_status='`✅ approved`'; description_status='✅ approved'
                ;;
            *) plan_die "Review status must be pending or approved" ;;
        esac
        review_tmp="${review}.tmp.$$"; description_tmp="${description}.tmp.$$"
        trap 'rm -f "$review_tmp" "$description_tmp"' EXIT
        awk -v replacement="$review_status" '
            /^- Status:/ { if (found++) exit 2; print "- Status: " replacement; next }
            { print } END { if (found != 1) exit 2 }
        ' "$review" > "$review_tmp" || plan_die "Review must contain exactly one Status field"
        awk -v replacement="$description_status" '
            /^- Status:/ { if (found++) exit 2; print "- Status: " replacement; next }
            { print } END { if (found != 1) exit 2 }
        ' "$description" > "$description_tmp" || plan_die "Plan description must contain exactly one Status field"
        mv "$review_tmp" "$review"
        mv "$description_tmp" "$description"
        # Invalidation is irreversible and only the fixer can re-mint, so it is
        # the branch's last act: a die after it leaves the plan not approved and
        # un-approvable.
        [ -z "$invalidate_session" ] || rm -rf "$(planning_tmpdir)/review-fix-keys/$invalidate_session"
        ;;
    decomposition-review)
        [ "$#" -eq 2 ] || usage
        plan_dir="$1"; requested_status="$2"; inventory="$plan_dir/work-unit-inventory.md"
        plan_require_directory "$plan_dir"
        plan_git_snapshot "$plan_dir"
        [ -f "$inventory" ] || plan_die "Work-unit inventory not found: $inventory"
        case "$requested_status" in incomplete) mark=' ' ;; completed) mark=x ;; *) plan_die "Decomposition review status must be incomplete or completed" ;; esac
        temporary_file="${inventory}.tmp.$$"
        trap 'rm -f "$temporary_file"' EXIT
        awk -v mark="$mark" '/^- \[[ xX]\] / { sub(/^- \[[ xX]\]/, "- [" mark "]") } { print }' "$inventory" > "$temporary_file"
        mv "$temporary_file" "$inventory"
        ;;
    *) usage ;;
esac

# The `declare -F` probe stays even though the library is sourced
# unconditionally: a renamed helper must not turn every content edit into a
# hard failure.
if declare -F context_invalidate_after_mutation >/dev/null 2>&1 && [ -n "${plan_dir:-}" ] && [ -d "$plan_dir/context" ]; then
    context_invalidate_after_mutation "$plan_dir" "${document_id:-plan}"
fi
printf 'Updated %s\n' "$command"
