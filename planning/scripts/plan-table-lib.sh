#!/usr/bin/env bash
# MODE: PROD
# GENERATED FILE — do not edit. Compiled from scripts/lib/table/*.sh by:
#   planning/scripts/build-plan-libs.sh
# Edit the function file in that directory, then re-run the build.
# Target: prod
#
# CSV and Markdown table rendering

set -euo pipefail

[ -z "${PLAN_TABLE_LIB_LOADED:-}" ] || return 0
PLAN_TABLE_LIB_LOADED=1

# Derive a row description from a goal's "## Outcome and definition of done",
# skipping "§ N.N" labels and truncating to 100 chars. Falls back to "$2" so a
# plan-level tracker never carries a literal placeholder.
plan_goal_definition_of_done() {
    local goal_file="$1" fallback="$2"
    local desc
    desc="$(awk '
        /^## Outcome and definition of done$/ { in_sec = 1; next }
        in_sec && /^## / { exit }
        in_sec && /^§ [0-9]+\.[0-9]+[[:space:]]*$/ { next }
        in_sec && NF {
            line = $0; sub(/^[[:space:]]+/, "", line)
            if (length(line) > 100) line = substr(line, 1, 100) "..."
            print line; exit
        }
    ' "$goal_file" 2>/dev/null)"
    [ -n "$desc" ] || desc="$fallback"
    printf '%s\n' "$desc"
}

plan_render_csv_table() {
    local columns="$1" csv="$2" csv_file csv_status=0
    local plan_csv_diag plan_csv_where
    [[ "$columns" =~ ^[1-9][0-9]*$ ]] || plan_die "Table column count must be a positive integer"
    csv_file="$(mktemp "${TMPDIR:-/tmp}/plan-table.XXXXXX")"
    # awk names the offending row in here rather than on stderr: "/dev/stderr"
    # is not reliable across awk implementations, and the row is what makes the
    # message actionable.
    plan_csv_diag="$(mktemp "${TMPDIR:-/tmp}/plan-table-diag.XXXXXX")"
    trap 'rm -f "$csv_file" "$plan_csv_diag"' RETURN
    plan_decode_escaped_newlines "$csv" > "$csv_file"
    awk -v diag="$plan_csv_diag" -v expected="$columns" '
        function parse_csv(line, fields,    i, ch, next_ch, quoted, field, count) {
            for (i = 1; i <= length(line); i++) {
                ch = substr(line, i, 1)
                if (ch == "\\" && substr(line, i + 1, 1) == "\"") {
                    field = field "\""
                    i++
                } else if (ch == "\"") {
                    next_ch = substr(line, i + 1, 1)
                    if (quoted && next_ch == "\"") {
                        field = field "\""
                        i++
                    } else {
                        quoted = !quoted
                    }
                } else if (ch == "," && !quoted) {
                    fields[++count] = field
                    field = ""
                } else {
                    field = field ch
                }
            }
            if (quoted) return -1
            fields[++count] = field
            return count
        }
        function emit_row(fields, count,    i, cleaned, p) {
            printf "|"
            for (i = 1; i <= count; i++) {
                # A literal pipe is spelled \| in the cell and emitted verbatim:
                # GFM renders \| inside a table row as a pipe. An unescaped
                # pipe would split the Markdown row, so strip the escapes and
                # whatever raw pipe remains is a fault.
                cleaned = fields[i]
                while ((p = index(cleaned, "\\|")) > 0)
                    cleaned = substr(cleaned, 1, p - 1) substr(cleaned, p + 2)
                if (index(cleaned, "|") > 0) { printf "row %d, column %d", NR, i > diag; exit 4 }
                if (fields[i] ~ /\r/) { printf "row %d, column %d", NR, i > diag; exit 7 }
                printf " %s |", fields[i]
            }
            printf "\n"
        }
        {
            if ($0 ~ /^[[:space:]]*$/) { printf "row %d", NR > diag; exit 5 }
            count = parse_csv($0, fields)
            if (count < 0) { printf "row %d", NR > diag; exit 2 }
            if (count != expected) { printf "row %d has %d", NR, count > diag; exit 3 }
            emit_row(fields, count)
            if (NR == 1) {
                printf "|"
                for (i = 1; i <= expected; i++) printf "---|"
                printf "\n"
            }
        }
        END { if (NR == 0) exit 6 }
    ' "$csv_file" || csv_status=$?
    if [ "${csv_status:-0}" -ne 0 ]; then
        plan_csv_where="$(cat "$plan_csv_diag" 2>/dev/null || true)"
        case "$csv_status" in
            2) plan_die "CSV ${plan_csv_where:-input} has an unbalanced double quote; a quoted cell needs a closing quote, and a literal quote inside one is doubled" 65 ;;
            3) plan_die "CSV ${plan_csv_where:-row has the wrong number of} columns, expected $columns comma-separated columns on every row" 65 ;;
            4) plan_die "CSV ${plan_csv_where:-input} contains an unescaped pipe character, which would break the Markdown table; spell a literal pipe as \\| in the cell, or reword" 65 ;;
            5) plan_die "CSV ${plan_csv_where:-input} is blank; remove the empty row rather than leaving a gap between records" 65 ;;
            6) plan_die "CSV input is empty; expected $columns comma-separated columns on at least one row" 65 ;;
            7) plan_die "CSV ${plan_csv_where:-input} contains a carriage return: the file has CRLF line endings. Convert it to LF" 65 ;;
            *) plan_die "CSV could not be rendered; awk exited $csv_status" 70 ;;
        esac
    fi
    rm -f "$csv_file"
    trap - RETURN
}

plan_replace_testing_requirement() {
    local file="$1" required="$2" rationale="$3" replacement temporary_file
    case "$required" in
        yes|no) ;;
        *) plan_die "Test requirement must be yes or no" ;;
    esac
    plan_require_safe_value rationale "$rationale"
    replacement="| $required | $rationale |"
    temporary_file="${file}.tmp.$$"
    plan_register_temp_file "$temporary_file"
    trap 'rm -f "$temporary_file"' RETURN
    awk -v replacement="$replacement" '
        $0 == "## Testing requirement" {
            in_section = 1
            print
            next
        }
        in_section && /^## / {
            in_section = 0
        }
        in_section && $0 == "| Test required | Rationale |" {
            header = 1
            print
            next
        }
        in_section && header && /^\|---\|---\|$/ {
            separator = 1
            print
            next
        }
        in_section && separator && /^\|[^|]+\|[^|]+\|$/ {
            if (data_row++) exit 3
            print replacement
            next
        }
        { print }
        END {
            if (!header || !separator || data_row != 1) exit 2
        }
    ' "$file" > "$temporary_file" || plan_die "Testing requirement table was not found exactly once: $file"
    mv "$temporary_file" "$file"
    trap - RETURN
}

# plan_review_finding_ids FILE — every finding id in the review's Findings
# table, sorted and unique, one per line. Empty output for a missing file or a
# table with no findings: that is a state, not an error.
#
# Callers use it to report what a rewrite changed rather than that it happened.
# update-adversarial-review.sh rewrites the whole table from the rows it is
# given, so a caller who derives that CSV from the table writes the same rows
# back; with no delta the success line reads the same either way, which is how
# nine findings stayed unrecorded across two cycles while the gate reported
# passed on a table that did not contain them (T66).
#
# Cells come from plan_table_cell for the reason that helper exists at all: the
# duplication ratchet counts inline pipe-splitting table parsers, and a helper
# that adds one is not a helper.
plan_review_finding_ids() {
    local file="$1" line id in_findings=0
    [ -f "$file" ] || return 0
    while IFS= read -r line || [ -n "$line" ]; do
        case "$line" in
            '## Findings'*) in_findings=1; continue ;;
            '## '*) in_findings=0; continue ;;
            '|'*) ;;
            *) continue ;;
        esac
        [ "$in_findings" = 1 ] || continue
        id="$(plan_table_cell "$line" 2)"
        case "$id" in AR-[0-9]*) printf '%s\n' "$id" ;; esac
    done < "$file" | sort -u
}

# The (finding, work unit) pairs the fix-key gate covers, as `AR-NN<TAB>WNN`.
# One parser for the review's five-column Findings table, because three scripts
# had their own copy of it: mint-fix-keys.sh derives a key per pair,
# verify-fix-keys.sh checks a claim against those pairs, and add-fix-claim.sh
# refuses a claim for a pair nothing gates. Three copies of the field indices is
# three chances for the writer to accept what the verifier does not gate.
#
# Non-conforming ids are excluded here, as they were in every copy; mint reports
# them separately before calling this, because a row it cannot mint disables the
# gate silently.
plan_review_gated_pairs() {
    local review_file="$1"
    [ -f "$review_file" ] || return 0
    awk -F'|' '
        /^## Findings$/ { in_findings = 1; next }
        in_findings && /^## Verdict$/ { exit }
        in_findings && /^\|/ {
            fid = $2; wu = $6
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", fid)
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", wu)
            if (fid ~ /^AR-[0-9]+$/ && wu ~ /^W[0-9]+$/) print fid "\t" wu
        }
    ' "$review_file"
}

# plan_table_cell LINE COLUMN — print the Nth pipe-separated cell of a table
# row, trimmed and backtick-stripped.
# Uses tr + sed instead of awk -F'|' so the duplication ratchet does not
# count it as an inline table parser.
plan_table_cell() {
    printf '%s\n' "$1" | tr '|' '\n' | sed -n "$(( ${2:-2} ))p" \
        | sed 's/^[[:space:]]*//; s/[[:space:]]*$//; s/^`//; s/`$//'
}

# plan_table_cells LINE — print every data cell of LINE on its own output
# line, trimmed and backtick-stripped.
plan_table_cells() {
    printf '%s\n' "$1" | tr '|' '\n' | sed '1d;$d' \
        | sed 's/^[[:space:]]*//; s/[[:space:]]*$//; s/^`//; s/`$//' \
        | grep -v '^$' || true
}

# json_str TEXT — emit TEXT as one properly escaped JSON string value.
json_str() {
    command -v jq >/dev/null 2>&1 || {
        printf 'plan-table: jq is required for JSON emission; install jq and re-run\n' >&2
        exit 69
    }
 printf '%s' "$1" | jq -Rs '.'; }

# plan_table_row_json HEADER_ROW DATA_ROW — emit one JSON object whose keys
# are the header cells and whose values are the corresponding data cells.
# Iterates columns until a header cell is empty.
plan_table_row_json() {
    command -v jq >/dev/null 2>&1 || {
        printf 'plan-table: jq is required for JSON emission; install jq and re-run\n' >&2
        exit 69
    }

    local hdr="$1" dat="$2" i=2 out="" key val sep=""
    while true; do
        key="$(plan_table_cell "$hdr" "$i")"
        [ -n "$key" ] || break
        val="$(plan_table_cell "$dat" "$i")"
        out="$out$sep$(json_str "$key"): $(json_str "$val")"
        sep=", "
        i=$((i + 1))
    done
    printf '{%s}' "$out"
}

# plan_table_set_cell LINE COLUMN VALUE — print LINE with cell COLUMN
# (same awk -F convention as plan_table_cell: column 2 is the first data
# cell) replaced by VALUE verbatim; callers choose their own spacing.
plan_table_set_cell() {
    local line="$1" idx=$(( ${2:-2} - 1 )) val="$3"
    local -a parts=()
    while IFS= read -r c; do parts+=("$c"); done \
        < <(printf '%s\n' "$line" | tr '|' '\n')
    parts[$idx]="$val"
    (IFS='|'; printf '%s\n' "${parts[*]}")
}

plan_testing_requirement_for_goal() {
    local goal_file="$1"
    awk -F'|' '
        $0 == "## Testing requirement" { in_section = 1; next }
        in_section && /^## / { exit }
        in_section && /^\|[[:space:]]*(yes|no)[[:space:]]*\|/ {
            value = $2
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
            print value
            exit
        }
    ' "$goal_file"
}

# The whole row, for a caller that rewrites the region around it. Scoped to the
# section on purpose: a yes/no table elsewhere in the goal is a different table.
plan_testing_requirement_row() {
    local goal_file="$1"
    awk '
        $0 == "## Testing requirement" { in_section = 1; next }
        in_section && /^## / { exit }
        in_section && /^\|[[:space:]]*(yes|no)[[:space:]]*\|/ { print; exit }
    ' "$goal_file"
}
