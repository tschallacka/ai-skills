#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat >&2 <<'USAGE'
Usage:
  update-plan-content.sh -dp|--description-paragraph <plan-directory> <N.N> <text>
  update-plan-content.sh -ds|--description-section <plan-directory> <section-id> -p N.N: <content> ...
  update-plan-content.sh -gp|--goal-paragraph <plan-directory> <goal-name> <N.N> <text>
  update-plan-content.sh -gs|--goal-section <plan-directory> <goal-name> <section-id> -p N.N: <content> ...
  update-plan-content.sh -sp|--step-paragraph <plan-directory> <goal>/<step> <N.N> <text>
  update-plan-content.sh -ss|--step-section <plan-directory> <goal>/<step> <section-id> -p N.N: <content> ...
  update-plan-content.sh -rp|--review-paragraph <plan-directory> <N.N> <text>
  update-plan-content.sh -rs|--review-section <plan-directory> <section-id> -p N.N: <content> ...
  update-plan-content.sh -tp|--table-paragraph <plan-directory> <document-id> <N.N> <columns> <CSV>
  update-plan-content.sh -ia|--insert-after <plan-directory> <document-id> <N.N> <text>
  update-plan-content.sh -ib|--insert-before <plan-directory> <document-id> <N.N> <text>
  update-plan-content.sh -t|--title <plan-directory> <document-id> <title>
  update-plan-content.sh -f|--field <plan-directory> <document-id> <field-label> <value>
  update-plan-content.sh -f|--field <plan-directory> <field-label> <value>   (document-id defaults to 'plan')
  update-plan-content.sh -tr|--testing-requirement <plan-directory> <goal-name> <yes|no> <rationale>
  update-plan-content.sh -rv|--review-status <plan-directory> <pending|approved>
  update-plan-content.sh -dr|--decomposition-review <plan-directory> <incomplete|completed>

Document IDs: plan, review, goal:<goal>, step:<goal>/<step>, or unit:<WNN>.
USAGE
    exit 64
}

[ "$#" -ge 1 ] || usage
command="$1"; shift
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"
[ -f "$script_dir/plan-context-lib.sh" ] && source "$script_dir/plan-context-lib.sh"

[[ "$command" == -* ]] || usage

normalize_flagged_paragraph() {
    local paragraph_id="$1"
    [[ "$paragraph_id" =~ ^[0-9]+\.[0-9]+:?$ ]] || plan_die "Paragraph must use N.N or N.N:"
    printf '%s\n' "${paragraph_id%:}:"
}

if [[ "$command" == -* ]]; then
    case "$command" in
        -dp|--description-paragraph)
            [ "$#" -ge 3 ] || usage
            plan_dir="$1"; paragraph_id="$2"; shift 2
            paragraph_content="$*"
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
            set -- "$plan_dir" review -p "$(normalize_flagged_paragraph "$paragraph_id")$paragraph_content"
            command=paragraph
            ;;
        -rs|--review-section)
            [ "$#" -ge 3 ] || usage
            plan_dir="$1"; section="$2"; shift 2
            set -- "$plan_dir" review "$section" "$@"
            command=section
            ;;
        -tp|--table-paragraph)
            [ "$#" -eq 5 ] || usage
            set -- "$1" "$2" "$3" "$4" "$5"
            command=table-paragraph
            ;;
        -ia|--insert-after)
            [ "$#" -ge 4 ] || usage
            plan_dir="$1"; document_id="$2"; paragraph_id="$3"; shift 3
            paragraph_content="$*"
            set -- "$plan_dir" "$document_id" "${paragraph_id%:}" "$paragraph_content"
            command=insert-after
            ;;
        -ib|--insert-before)
            [ "$#" -ge 4 ] || usage
            plan_dir="$1"; document_id="$2"; paragraph_id="$3"; shift 3
            paragraph_content="$*"
            set -- "$plan_dir" "$document_id" "${paragraph_id%:}" "$paragraph_content"
            command=insert-before
            ;;
        -t|--title)
            [ "$#" -eq 3 ] || usage
            set -- "$1" "$2" "$3"
            command=title
            ;;
        -f|--field)
            case "$#" in
                3) set -- "$1" plan "$2" "$3" ;; # omit document-id == plan document
                4) set -- "$1" "$2" "$3" "$4" ;;
                *) usage ;;
            esac
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
        paragraph_content="${BASH_REMATCH[3]}"
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

case "$command" in
    title)
        [ "$#" -eq 3 ] || usage
        plan_dir="$1"; document_id="$2"; title="$3"
        plan_require_directory "$plan_dir"
        file="$(plan_document_path "$plan_dir" "$document_id")"
        [ -f "$file" ] || plan_die "Document not found: $file"
        plan_replace_title "$file" "$title"
        plan_emit_step_testing_reminder "$plan_dir" "$document_id"
        ;;
    section)
        [ "$#" -ge 4 ] || usage
        plan_dir="$1"; document_id="$2"; section="$3"; shift 3
        plan_require_directory "$plan_dir"
        file="$(plan_document_path "$plan_dir" "$document_id")"
        [ -f "$file" ] || plan_die "Document not found: $file"
        IFS=$'\t' read -r heading number < <(plan_section_spec "$(plan_document_kind "$document_id")" "$section")
        body_file="$(mktemp "${TMPDIR:-/tmp}/plan-section.XXXXXX")"
        trap 'rm -f "$body_file"' EXIT
        parse_paragraph_arguments "$number" "$@"
        render_paragraph_arguments > "$body_file"
        plan_replace_section "$file" "$heading" "$body_file"
        rm -f "$body_file"
        trap - EXIT
        plan_emit_step_testing_reminder "$plan_dir" "$document_id"
        ;;
    paragraph)
        [ "$#" -ge 4 ] || usage
        plan_dir="$1"; document_id="$2"; shift 2
        plan_require_directory "$plan_dir"
        file="$(plan_document_path "$plan_dir" "$document_id")"
        [ -f "$file" ] || plan_die "Document not found: $file"
        [ "$1" = '-p' ] || plan_die "Paragraph replacement requires -p N.N: content"
        shift
        [ "$#" -gt 0 ] || plan_die "Missing paragraph after -p"
        paragraph_spec="$1"; shift
        [[ "$paragraph_spec" =~ ^([0-9]+)\.([0-9]+):(.*)$ ]] || plan_die "Paragraph must use N.N: content, for example 2.1: First paragraph"
        paragraph_id="§ ${BASH_REMATCH[1]}.${BASH_REMATCH[2]}"
        paragraph_content="${BASH_REMATCH[3]}"
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
        plan_replace_paragraph "$file" "$paragraph_id" "$paragraph_content"
        plan_emit_step_testing_reminder "$plan_dir" "$document_id"
        ;;
    table-paragraph)
        [ "$#" -eq 5 ] || usage
        plan_dir="$1"; document_id="$2"; paragraph_id="$3"; columns="$4"; csv="$5"
        plan_require_directory "$plan_dir"
        file="$(plan_document_path "$plan_dir" "$document_id")"
        [ -f "$file" ] || plan_die "Document not found: $file"
        [[ "$paragraph_id" =~ ^[0-9]+\.[0-9]+$ ]] || plan_die "Paragraph must use N.N"
        table_file="$(mktemp "${TMPDIR:-/tmp}/plan-table-paragraph.XXXXXX")"
        trap 'rm -f "$table_file"' EXIT
        plan_render_csv_table "$columns" "$csv" > "$table_file"
        table_content="$(cat "$table_file")"
        plan_replace_paragraph "$file" "§ $paragraph_id" "$table_content"
        rm -f "$table_file"
        trap - EXIT
        plan_emit_step_testing_reminder "$plan_dir" "$document_id"
        ;;
    insert-after|insert-before)
        [ "$#" -eq 4 ] || usage
        plan_dir="$1"; document_id="$2"; paragraph_id="$3"; paragraph_content="$4"
        plan_require_directory "$plan_dir"
        file="$(plan_document_path "$plan_dir" "$document_id")"
        [ -f "$file" ] || plan_die "Document not found: $file"
        [[ "$paragraph_id" =~ ^[0-9]+\.[0-9]+$ ]] || plan_die "Paragraph must use N.N"
        [[ "$paragraph_content" != *$'\n'* && "$paragraph_content" != *$'\r'* ]] || plan_die "Inserted paragraph must be one line"
        [[ "$paragraph_content" != *'§'* ]] || plan_die "Paragraph content must not contain the reserved paragraph marker §"
        [ -n "${paragraph_content//[[:space:]]/}" ] || plan_die "Inserted paragraph must not be empty"
        insert_file="$(mktemp "${TMPDIR:-/tmp}/plan-insert-paragraph.XXXXXX")"
        trap 'rm -f "$insert_file"' EXIT
        printf '%s\n' "$paragraph_content" > "$insert_file"
        plan_insert_paragraph "$file" "§ $paragraph_id" "${command#insert-}" "$insert_file"
        rm -f "$insert_file"
        trap - EXIT
        plan_emit_step_testing_reminder "$plan_dir" "$document_id"
        ;;
    field)
        [ "$#" -eq 4 ] || usage
        plan_dir="$1"; document_id="$2"; label="$3"; value="$4"
        plan_require_directory "$plan_dir"
        file="$(plan_document_path "$plan_dir" "$document_id")"
        [ -f "$file" ] || plan_die "Document not found: $file"
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
        plan_replace_testing_requirement "$goal_file" "$required" "$rationale"
        ;;
    review-status)
        [ "$#" -eq 2 ] || usage
        plan_dir="$1"; requested_status="$2"
        plan_require_directory "$plan_dir"
        review="$plan_dir/adversarial-review.md"; description="$plan_dir/plan-description.md"
        [ -f "$review" ] && [ -f "$description" ] || plan_die "Both plan-description.md and adversarial-review.md are required"
        case "$requested_status" in
            pending) review_status='`💤 pending`'; description_status='💤 pending' ;;
            approved)
                if grep -Eq '^\|[[:space:]]*AR-[0-9]+[[:space:]]*\|.*\|[[:space:]]*(💤 open|⏳ in progress)[[:space:]]*\|$' "$review"; then
                    plan_die "Cannot approve a review with unresolved findings"
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
        trap - EXIT
        ;;
    decomposition-review)
        [ "$#" -eq 2 ] || usage
        plan_dir="$1"; requested_status="$2"; inventory="$plan_dir/work-unit-inventory.md"
        plan_require_directory "$plan_dir"
        [ -f "$inventory" ] || plan_die "Work-unit inventory not found: $inventory"
        case "$requested_status" in incomplete) mark=' ' ;; completed) mark=x ;; *) plan_die "Decomposition review status must be incomplete or completed" ;; esac
        temporary_file="${inventory}.tmp.$$"
        trap 'rm -f "$temporary_file"' EXIT
        awk -v mark="$mark" '/^- \[[ xX]\] / { sub(/^- \[[ xX]\]/, "- [" mark "]") } { print }' "$inventory" > "$temporary_file"
        mv "$temporary_file" "$inventory"
        trap - EXIT
        ;;
    *) usage ;;
esac

if declare -F context_invalidate_after_mutation >/dev/null 2>&1 && [ -n "${plan_dir:-}" ] && [ -d "$plan_dir/context" ]; then
    context_invalidate_after_mutation "$plan_dir" "${document_id:-plan}"
fi
echo "Updated $command"
