#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat >&2 <<'USAGE'
Usage:
  update-plan-content.sh title <plan-directory> <document-id> <title>
  update-plan-content.sh section <plan-directory> <document-id> <section-id> -p N.N: <content> [-p N.N: <content> ...]
  update-plan-content.sh paragraph <plan-directory> <document-id> -p N.N: <content>
  update-plan-content.sh field <plan-directory> <document-id> <field-label> <value>
  update-plan-content.sh review-status <plan-directory> <pending|approved>
  update-plan-content.sh decomposition-review <plan-directory> <incomplete|completed>

Document IDs: plan, review, goal:<goal>, step:<goal>/<step>, or unit:<WNN>.
USAGE
    exit 64
}

[ "$#" -ge 1 ] || usage
command="$1"; shift
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"

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
        [[ ! "$paragraph_content" =~ (^|[[:space:]])-p($|[[:space:]]) ]] || plan_die "Paragraph content must not contain the reserved -p argument token"
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
        [[ ! "$paragraph_content" =~ (^|[[:space:]])-p($|[[:space:]]) ]] || plan_die "Paragraph content must not contain the reserved -p argument token"
        [ -n "${paragraph_content//[[:space:]]/}" ] || plan_die "Paragraph content must not be empty"
        plan_replace_paragraph "$file" "$paragraph_id" "$paragraph_content"
        ;;
    field)
        [ "$#" -eq 4 ] || usage
        plan_dir="$1"; document_id="$2"; label="$3"; value="$4"
        plan_require_directory "$plan_dir"
        file="$(plan_document_path "$plan_dir" "$document_id")"
        [ -f "$file" ] || plan_die "Document not found: $file"
        plan_replace_field "$file" "$label" "$value"
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

echo "Updated $command"
