#!/usr/bin/env bash
# Shared helpers for the planning document commands. This file is sourced.

set -euo pipefail

plan_default_root() {
    if [ -n "${PLANS_ROOT:-}" ]; then
        printf '%s\n' "${PLANS_ROOT%/}"
        return 0
    fi
    local home_dir="${HOME:-}"
    if [ -z "$home_dir" ] && [ -n "${USERPROFILE:-}" ]; then
        home_dir="$USERPROFILE"
    fi
    if [ -z "$home_dir" ] && [ -n "${HOMEDRIVE:-}${HOMEPATH:-}" ]; then
        home_dir="${HOMEDRIVE:-}${HOMEPATH:-}"
    fi
    [ -n "$home_dir" ] || plan_die "Unable to resolve the user home directory; set PLANS_ROOT"
    printf '%s/.plans\n' "${home_dir%/}"
}

plan_ensure_root_permissions() {
    local root="${1:-$(plan_default_root)}" helper_dir="${2:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)}"
    local probe
    mkdir -p "$root" || plan_die "Cannot create plan root: $root"
    [ -d "$root" ] && [ -r "$root" ] && [ -w "$root" ] && [ -x "$root" ] \
        || plan_die "Plan root is not readable, writable, and searchable: $root"
    probe="$root/.permission-probe.$$"
    ( : > "$probe" && rm -f "$probe" ) || plan_die "Plan root does not permit file editing: $root"
    [ -d "$helper_dir" ] && [ -r "$helper_dir" ] && [ -x "$helper_dir" ] \
        || plan_die "Planning helper directory is not readable/searchable: $helper_dir"
    # find(1) exits 0 regardless of what -exec returns, so `-exec test -r {} \;`
    # cannot fail the check; test in the shell. Libraries are sourced, never
    # executed (CODE-STYLE §3), so only readability is required of them.
    local helper
    while IFS= read -r helper; do
        [ -n "$helper" ] || continue
        [ -r "$helper" ] \
            || plan_die "One or more planning helpers cannot be read and executed: $helper_dir"
        case "$helper" in
            *-lib.sh) ;;
            *) [ -x "$helper" ] \
                || plan_die "One or more planning helpers cannot be read and executed: $helper_dir" ;;
        esac
    done < <(find "$helper_dir" -maxdepth 1 -type f -name '*.sh' -print)
    printf '%s\n' "$root"
}

# The single fatal path (CODE-STYLE §5): message to stderr, exit with $2. The
# default stays 64 so every single-argument caller keeps its exit status.
plan_die() {
    printf '%s: %s\n' "${0##*/}" "$1" >&2
    exit "${2:-64}"
}

# Non-fatal accumulators for checkers that report every finding. plan_fail
# bumps plan_error_count; the caller exits 1 at the end when it is non-zero.
# Neither ever exits: a sourced function must leave that decision to its caller.
plan_fail() {
    plan_error_count=$((plan_error_count + 1))
    printf 'FAIL: %s\n' "$*" >&2
}

plan_warn() {
    printf 'WARN: %s\n' "$*" >&2
}

# Commit the plan directory before a mutation so every overwrite is
# recoverable. Plan directories are usually gitignored by the host repo, so they
# carry their own git. No-op without git or without a git-initialized plan dir.
plan_git_snapshot() {
    local plan_dir="$1"
    command -v git >/dev/null 2>&1 || return 0
    [ -d "$plan_dir/.git" ] || return 0
    git -C "$plan_dir" add -A -- . >/dev/null 2>&1 || return 0
    git -C "$plan_dir" -c user.name='plan-skill' -c user.email='plan-skill@localhost' \
        commit -q -m "snapshot before ${0##*/}" >/dev/null 2>&1 || true
}

# Scratch directory the planning skill may write temporary capsules and run
# artifacts into. It lives under the system temp dir so it is fresh per boot;
# the agent's existing write access to the temp dir suffices to create it.
planning_tmpdir() {
    printf '%s\n' "${TMPDIR:-/tmp}/planning-agent"
}

# Ensure the planning scratch directory exists for this boot. Failure is
# ignored: the helpers still work when a nonstandard TMPDIR is unwritable.
planning_ensure_tmpdir() {
    local d
    d="$(planning_tmpdir)"
    mkdir -p "$d" 2>/dev/null && chmod 700 "$d" 2>/dev/null || true
}

plan_require_directory() {
    [ -d "$1" ] || plan_die "Plan directory not found: $1" 66
}

# Require an input file; refuse to clobber an existing artifact. Same exit-code
# vocabulary as plan_require_directory: 66 for "missing input", 73 for
# "already there, not overwriting".
plan_require_file() {
    [ -f "$1" ] || plan_die "File not found: $1" 66
}

plan_refuse_existing() {
    [ ! -e "$1" ] || plan_die "Refusing to overwrite an existing artifact: $1" 73
}

plan_require_safe_value() {
    local label="$1" value="$2"
    [ -n "$value" ] || plan_die "$label must not be empty"
    [[ "$value" != *'|'* ]] || plan_die "$label must not contain a Markdown table separator (|)"
    [[ "$value" != *$'\n'* && "$value" != *$'\r'* ]] || plan_die "$label must be one line"
}

plan_decode_escaped_newlines() {
    local value="$1"
    printf '%s' "${value//\\n/$'\n'}"
}

plan_document_path() {
    local plan_dir="$1" document_id="$2" unit goal step
    case "$document_id" in
        plan)
            printf '%s\n' "$plan_dir/plan-description.md"
            ;;
        review)
            printf '%s\n' "$plan_dir/adversarial-review.md"
            ;;
        coverage|inventory)
            printf '%s\n' "$plan_dir/work-unit-inventory.md"
            ;;
        stories)
            printf '%s\n' "$plan_dir/ui-user-stories.md"
            ;;
        fixes)
            printf '%s\n' "$plan_dir/fixes.md"
            ;;
        fix-keys|fixkeys)
            printf '%s\n' "$plan_dir/fix-keys.json"
            ;;
        approval)
            printf '%s\n' "$plan_dir/approval.json"
            ;;
        goal:*)
            goal="${document_id#goal:}"
            printf '%s\n' "$plan_dir/$goal/goal.md"
            ;;
        step:*)
            goal="${document_id#step:}"
            step="${goal#*/}"
            goal="${goal%%/*}"
            [ -n "$step" ] && [ "$step" != "$goal" ] || plan_die "Step document IDs use step:<goal>/<step>"
            # A trailing -testing names the step's testing companion, which is
            # a writable surface of its own (executors run the procedure it
            # records). It resolves to steps/<step>-testing.md.
            printf '%s\n' "$plan_dir/$goal/steps/$step.md"
            ;;
        unit:W*)
            unit="${document_id#unit:}"
            IFS=$'\t' read -r goal step < <(
                awk -F'|' -v wanted="$unit" '
                    function trim(value) { gsub(/^[[:space:]]+|[[:space:]]+$/, "", value); return value }
                    /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ && trim($2) == wanted {
                        print trim($9) "\t" trim($10)
                    }
                ' "$plan_dir/work-unit-inventory.md"
            )
            [ -n "${goal:-}" ] && [ -n "${step:-}" ] || plan_die "Work unit not found: $unit"
            printf '%s\n' "$plan_dir/$goal/steps/$step.md"
            ;;
        *)
            plan_die "Unknown document ID: $document_id (use plan, review, goal:<goal>, step:<goal>/<step>, unit:<WNN>, coverage, inventory, stories, fixes, fix-keys, or approval)"
            ;;
    esac
}

plan_document_kind() {
    case "$1" in
        plan) printf '%s\n' plan ;;
        review) printf '%s\n' review ;;
        coverage|inventory|stories|fixes|fix-keys|fixkeys|approval) printf '%s\n' reference ;;
        goal:*) printf '%s\n' goal ;;
        step:*)
            # A step id ending in -testing names the step's testing companion,
            # which has its own writable sections (Automated tests, ...).
            case "$1" in
                *-testing) printf '%s\n' testing ;;
                *) printf '%s\n' step ;;
            esac
            ;;
        unit:*) printf '%s\n' step ;;
        *) plan_die "Unknown document ID: $1" ;;
    esac
}

# Print the required heading and its paragraph-number prefix for a mutable
# narrative section. Structured sections are intentionally excluded.
plan_section_spec() {
    local kind="$1" section="$2"
    case "$kind/$section" in
        plan/current-state) printf '%s\t%s\n' '## Current state' 2 ;;
        plan/desired-outcome) printf '%s\t%s\n' '## Desired outcome' 3 ;;
        plan/approach) printf '%s\t%s\n' '## Approach' 4 ;;
        plan/scope) printf '%s\t%s\n' '## Scope' 5 ;;
        plan/affected-areas) printf '%s\t%s\n' '## Affected areas' 6 ;;
        plan/constraints-and-decisions) printf '%s\t%s\n' '## Constraints and decisions' 7 ;;
        plan/risks-and-open-questions) printf '%s\t%s\n' '## Risks and open questions' 8 ;;
        plan/environment-facts) printf '%s\t%s\n' '## Environment facts' 9 ;;
        plan/approach-decisions) printf '%s\t%s\n' '## Approach decisions' 10 ;;
        goal/current-state-and-prior-goal-handoffs) printf '%s\t%s\n' '## Current state and prior-goal handoffs' 2 ;;
        goal/outcome-and-definition-of-done) printf '%s\t%s\n' '## Outcome and definition of done' 3 ;;
        goal/why-this-goal-is-needed) printf '%s\t%s\n' '## Why this goal is needed' 4 ;;
        goal/scope) printf '%s\t%s\n' '## Scope' 5 ;;
        goal/affected-areas) printf '%s\t%s\n' '## Affected files, systems, data, and interfaces' 6 ;;
        goal/dependencies-and-handoffs) printf '%s\t%s\n' '## Dependencies and handoffs' 7 ;;
        goal/implementation-approach-risks-and-edge-cases) printf '%s\t%s\n' '## Implementation approach, risks, and edge cases' 8 ;;
        goal/owned-work-units) printf '%s\t%s\n' '## Owned work units' 9 ;;
        goal/goal-size-exception) printf '%s\t%s\n' '## Goal-size exception' 11 ;;
        step/objective) printf '%s\t%s\n' '## Objective' 4 ;;
        step/instructions) printf '%s\t%s\n' '## Instructions' 5 ;;
        step/acceptance-criteria) printf '%s\t%s\n' '## Acceptance criteria' 6 ;;
        step/handoff) printf '%s\t%s\n' '## Handoff' 7 ;;
        testing/automated-tests) printf '%s\t%s\n' '## Automated tests' 2 ;;
        testing/browser-verification) printf '%s\t%s\n' '## Browser verification' 3 ;;
        testing/backend-verification) printf '%s\t%s\n' '## Backend verification' 4 ;;
        testing/manual-verification) printf '%s\t%s\n' '## Manual verification' 5 ;;
        review/review-scope) printf '%s\t%s\n' '## Review scope' 1 ;;
        review/findings) printf '%s\t%s\n' '## Findings' 2 ;;
        review/rationale) printf '%s\t%s\n' '## Verdict' 3 ;;
        *) plan_die "$(plan_unknown_section "$kind" "$section")" ;;
    esac
}

# Concise, agent-friendly error for an unknown narrative section: list the
# valid ids for the document kind and, when one is close, suggest it.
plan_unknown_section() {
    local kind="$1" section="$2" valid id close="" best=""
    case "$kind" in
        plan) valid="current-state desired-outcome approach approach-decisions scope affected-areas constraints-and-decisions risks-and-open-questions environment-facts" ;;
        goal) valid="current-state-and-prior-goal-handoffs outcome-and-definition-of-done why-this-goal-is-needed scope affected-areas dependencies-and-handoffs implementation-approach-risks-and-edge-cases owned-work-units goal-size-exception" ;;
        step) valid="objective instructions acceptance-criteria handoff" ;;
        testing) valid="automated-tests browser-verification backend-verification manual-verification" ;;
        review) valid="review-scope findings rationale" ;;
        *) valid="" ;;
    esac
    for id in $valid; do
        if [ "$id" = "$section" ]; then
            printf 'Section is valid: %s\n' "$section"
            return 0
        fi
        # Simple closeness: same prefix or >50% shared prefix length.
        if [[ "$id" == "$section"* ]] || [[ "$section" == "$id"* ]]; then
            [ -z "$close" ] && close="$id"
        fi
        if [ -z "$best" ] || [ "${#id}" -lt "${#best}" ]; then
            # crude nearest: shortest id differing in fewest chars by prefix
            :
        fi
    done
    printf "Section '%s' is not a mutable narrative section for a %s document.\n" "$section" "$kind"
    if [ -n "$close" ]; then
        printf 'Closest match: %s\n' "$close"
    fi
    printf 'Valid %s section ids: %s\n' "$kind" "$valid"
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
    if [ -f "$companion" ]; then
        printf 'Reminder: testing instructions already exist at %s; review them for accuracy and completeness after updating this step.\n' "$companion" >&2
    else
        printf 'Reminder: this goal requires testing; continue with its test/proof step before marking the goal complete.\n' >&2
    fi
}

plan_render_paragraphs() {
    local number="$1" content="$2"
    [ -n "$content" ] || plan_die "Section content must not be empty"
    printf '%s' "$content" | awk -v number="$number" '
        BEGIN { RS=""; ORS="" }
        {
            text = $0
            # No "\n" inside the bracket expression: a backslash escape there
            # is undefined in POSIX awk, and [[:space:]] already covers newline.
            sub(/^[[:space:]]+/, "", text)
            sub(/[[:space:]]+$/, "", text)
            if (text == "") next
            if (count++) printf "\n\n"
            printf "§ %s.%d\n%s", number, count, text
        }
        END { if (count == 0) exit 1 }
    '
}

plan_replace_section() {
    local file="$1" heading="$2" body_file="$3" temporary_file
    temporary_file="${file}.tmp.$$"
    trap 'rm -f "$temporary_file"' RETURN
    awk -v heading="$heading" -v replacement="$body_file" '
        BEGIN {
            while ((getline line < replacement) > 0) {
                body = body (body == "" ? "" : "\n") line
            }
            close(replacement)
        }
        $0 == heading {
            if (found++) exit 2
            print
            print ""
            print body
            skipping = 1
            next
        }
        skipping && /^## / { skipping = 0; print "" }
        !skipping { print }
        END { if (found != 1) exit 2 }
    ' "$file" > "$temporary_file" || plan_die "Section heading was not found exactly once: $heading"
    mv "$temporary_file" "$file"
    trap - RETURN
}

plan_replace_paragraph() {
    local file="$1" paragraph_id="$2" content="$3" temporary_file
    [[ "$paragraph_id" =~ ^§[[:space:]][0-9]+\.[0-9]+$ ]] || plan_die "Paragraph ID must use the form '§ 2.1'"
    [[ "$content" != *$'\n\n'* ]] || plan_die "A paragraph replacement must contain exactly one paragraph; use section for multiple paragraphs"
    [ -n "$content" ] || plan_die "Paragraph content must not be empty"
    temporary_file="${file}.tmp.$$"
    trap 'rm -f "$temporary_file"' RETURN
    awk -v wanted="$paragraph_id" -v replacement="$content" '
        $0 == wanted {
            if (found++) exit 2
            print
            print replacement
            skipping = 1
            next
        }
        skipping && ($0 == "" || /^§[[:space:]][0-9]+\.[0-9]+$/ || /^## /) {
            if ($0 ~ /^§[[:space:]][0-9]+\.[0-9]+$/ || $0 ~ /^## /) print ""
            skipping = 0
        }
        !skipping { print }
        END { if (found != 1) exit 2 }
    ' "$file" > "$temporary_file" || plan_die "Paragraph was not found exactly once: $paragraph_id"
    mv "$temporary_file" "$file"
    trap - RETURN
}

plan_render_csv_table() {
    local columns="$1" csv="$2" csv_file
    [[ "$columns" =~ ^[1-9][0-9]*$ ]] || plan_die "Table column count must be a positive integer"
    csv_file="$(mktemp "${TMPDIR:-/tmp}/plan-table.XXXXXX")"
    trap 'rm -f "$csv_file"' RETURN
    plan_decode_escaped_newlines "$csv" > "$csv_file"
    awk -v expected="$columns" '
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
        function emit_row(fields, count,    i) {
            printf "|"
            for (i = 1; i <= count; i++) {
                if (fields[i] ~ /\|/ || fields[i] ~ /\r/) exit 4
                printf " %s |", fields[i]
            }
            printf "\n"
        }
        {
            if ($0 ~ /^[[:space:]]*$/) exit 5
            count = parse_csv($0, fields)
            if (count < 0) exit 2
            if (count != expected) exit 3
            emit_row(fields, count)
            if (NR == 1) {
                printf "|"
                for (i = 1; i <= expected; i++) printf "---|"
                printf "\n"
            }
        }
        END { if (NR == 0) exit 6 }
    ' "$csv_file" || plan_die "CSV table must have $columns columns on every non-empty row and no pipe characters"
    rm -f "$csv_file"
    trap - RETURN
}

plan_insert_paragraph() {
    local file="$1" paragraph_id="$2" mode="$3" body_file="$4" temporary_file
    [[ "$paragraph_id" =~ ^§[[:space:]][0-9]+\.[0-9]+$ ]] || plan_die "Paragraph ID must use the form '§ 2.1'"
    case "$mode" in before|after) ;; *) plan_die "Paragraph insertion mode must be before or after" ;; esac
    temporary_file="${file}.tmp.$$"
    trap 'rm -f "$temporary_file"' RETURN
    awk -v wanted="$paragraph_id" -v mode="$mode" -v body_file="$body_file" '
        function output(line) {
            print line
            previous_blank = (line == "")
        }
        function emit_insertion(    count, i, lines) {
            if (!previous_blank) output("")
            output("§ " target_section "." insertion_number)
            count = split(body, lines, "\n")
            for (i = 1; i <= count; i++) output(lines[i])
            output("")
        }
        BEGIN {
            while ((getline line < body_file) > 0) body = body (body == "" ? "" : "\n") line
            close(body_file)
            target_value = wanted
            sub(/^§ /, "", target_value)
            split(target_value, target_parts, /\./)
            target_section = target_parts[1]
            target_number = target_parts[2] + 0
            insertion_number = (mode == "after" ? target_number + 1 : target_number)
        }
        {
            line = $0
            is_paragraph = (line ~ /^§ [0-9]+\.[0-9]+$/)
            if (is_paragraph) {
                current_value = line
                sub(/^§ /, "", current_value)
                split(current_value, current_parts, /\./)
                section = current_parts[1]
                number = current_parts[2] + 0
                is_target = (section == target_section && number == target_number)
                if (is_target && target_found++) exit 2
                if (is_target && mode == "before") emit_insertion()
                if (pending_after && (is_paragraph || line ~ /^## /)) {
                    emit_insertion()
                    pending_after = 0
                }
                if (is_target && mode == "after") pending_after = 1
                if (section == target_section && number >= insertion_number) {
                    line = "§ " section "." (number + 1)
                }
            } else if (pending_after && line ~ /^## /) {
                emit_insertion()
                pending_after = 0
            }
            output(line)
        }
        END {
            if (pending_after) emit_insertion()
            if (target_found != 1) exit 3
        }
    ' "$file" > "$temporary_file" || plan_die "Paragraph was not found exactly once: $paragraph_id"
    mv "$temporary_file" "$file"
    trap - RETURN
}

# Delete one numbered paragraph and renumber the rest of its section so labels
# stay sequential. Targeted rather than re-emitting the section, which risks a
# transcription slip damaging paragraphs no finding was about.
plan_delete_paragraph() {
    local file="$1" paragraph_id="$2" temporary_file
    [[ "$paragraph_id" =~ ^§[[:space:]][0-9]+\.[0-9]+$ ]] || plan_die "Paragraph ID must use the form '§ 2.1'"
    temporary_file="${file}.tmp.$$"
    trap 'rm -f "$temporary_file"' RETURN
    awk -v wanted="$paragraph_id" '
        BEGIN {
            target_value = wanted
            sub(/^§ /, "", target_value)
            split(target_value, target_parts, /\./)
            target_section = target_parts[1]
            target_number = target_parts[2] + 0
        }
        /^§ [0-9]+\.[0-9]+$/ {
            current_value = $0
            sub(/^§ /, "", current_value)
            split(current_value, current_parts, /\./)
            section = current_parts[1]
            number = current_parts[2] + 0
            if (section == target_section && number == target_number) {
                if (target_found++) exit 2
                skipping = 1
                next
            }
            skipping = 0
            if (section == target_section && number > target_number) {
                print "§ " section "." (number - 1)
            } else {
                print
            }
            next
        }
        /^## / {
            # A section boundary always stops the delete: never swallow a
            # following heading even when the deleted paragraph was the last in
            # its section (that would re-parent the next section under it).
            skipping = 0
            print
            next
        }
        skipping { next }
        { print }
        END { if (target_found != 1) exit 3 }
    ' "$file" > "$temporary_file" || plan_die "Paragraph was not found exactly once: $paragraph_id"
    mv "$temporary_file" "$file"
    trap - RETURN
}

plan_replace_field() {
    local file="$1" label="$2" value="$3" temporary_file
    plan_require_safe_value "$label" "$value"
    temporary_file="${file}.tmp.$$"
    trap 'rm -f "$temporary_file"' RETURN
    awk -v label="$label" -v replacement="$value" '
        $0 ~ "^- " label ":" {
            if (found++) exit 2
            print "- " label ": " replacement
            next
        }
        { print }
        END { if (found != 1) exit 2 }
    ' "$file" > "$temporary_file" || plan_die "Field was not found exactly once: $label"
    mv "$temporary_file" "$file"
    trap - RETURN
}

plan_replace_title() {
    local file="$1" title="$2" temporary_file
    plan_require_safe_value title "$title"
    [[ "$title" != *$'\n'* ]] || plan_die "Title must be one line"
    temporary_file="${file}.tmp.$$"
    trap 'rm -f "$temporary_file"' RETURN
    awk -v replacement="$title" '
        /^# / {
            if (found++) exit 2
            sub(/:.*/, ": " replacement)
            print
            next
        }
        { print }
        END { if (found != 1) exit 2 }
    ' "$file" > "$temporary_file" || plan_die "Document title was not found exactly once"
    mv "$temporary_file" "$file"
    trap - RETURN
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

# ── Portable helper vocabulary (CODE-STYLE §1 / §5 / §7 / §8) ────────────────
# Everything below runs on bash 3.2 with BSD userland. Platform branches are
# probed once at load time, never per call.

# Refuse to run under a bash older than <major>. NOT called at load time — this
# library itself is 3.2-clean; a script that genuinely needs bash 4 calls it.
plan_require_bash() {
    local want="$1"
    [ "${BASH_VERSINFO[0]}" -ge "$want" ] || plan_die \
        "needs bash $want or newer (running ${BASH_VERSION:-unknown}); on macOS: brew install bash" 78
}

# File mode (octal, e.g. 644) and owner uid. GNU stat and BSD stat share no
# flag, so probe once and define the function accordingly rather than forking a
# probe on every call. GNU %a and BSD %Lp both print the low 12 mode bits.
if stat -c '%a' /dev/null >/dev/null 2>&1; then
    plan_stat_mode() { stat -c '%a' -- "$1"; }
    plan_stat_uid() { stat -c '%u' -- "$1"; }
else
    plan_stat_mode() { stat -f '%Lp' "$1"; }
    plan_stat_uid() { stat -f '%u' "$1"; }
fi

# Resolve a symlink chain without `readlink -f` (GNU; macOS only since 12.3).
# Relative targets resolve against the link's own directory; a non-symlink is
# echoed back. The 32-hop cap turns a cycle into a diagnosed failure.
plan_resolve_symlink() {
    local path="$1" hops=0 target
    while [ -L "$path" ]; do
        hops=$((hops + 1))
        [ "$hops" -le 32 ] || plan_die "symlink chain exceeds 32 hops (cycle?): $1" 66
        target="$(readlink "$path")"
        case "$target" in
            /*) path="$target" ;;
            *) path="$(dirname "$path")/$target" ;;
        esac
    done
    printf '%s\n' "$path"
}

# ── Temp-file bookkeeping (CODE-STYLE §8) ────────────────────────────────────
# bash keeps exactly one EXIT handler: a script installing its own after
# sourcing this library replaces plan_cleanup, and `trap - EXIT` clears the slot.
plan_track_tmp() {
    plan_tmp_files=(${plan_tmp_files[@]+"${plan_tmp_files[@]}"} "$1")
}

plan_cleanup() {
    local f body
    for f in ${plan_tmp_files[@]+"${plan_tmp_files[@]}"}; do
        [ -z "$f" ] || rm -f -- "$f"
    done
    plan_tmp_files=()
    # Chain the handler that was installed before we were sourced. `trap -p`
    # prints `trap -- 'body' EXIT`; the inner eval strips the single quotes and
    # runs the body exactly once.
    if [ -n "${plan_prior_exit_trap:-}" ]; then
        body="${plan_prior_exit_trap#trap -- }"
        body="${body% EXIT}"
        plan_prior_exit_trap=""
        eval "eval $body"
    fi
}

# Write stdin to <target> atomically. The temp lives in the target's own
# directory so the rename cannot cross a filesystem, inherits the target's mode
# when it exists, and is registered with the cleanup list.
plan_atomic_write() {
    local target="$1" dir base tmp mode
    dir="$(dirname "$target")"
    base="$(basename "$target")"
    [ -d "$dir" ] || plan_die "Target directory not found: $dir" 66
    tmp="$(mktemp "$dir/.$base.XXXXXX")" || plan_die "Cannot create a temp file in: $dir" 73
    plan_track_tmp "$tmp"
    cat > "$tmp"
    if [ -e "$target" ]; then
        mode="$(plan_stat_mode "$target" 2>/dev/null || true)"
        [ -z "$mode" ] || chmod "$mode" "$tmp"
    fi
    mv -f "$tmp" "$target"
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

# The shared awk prelude defining trim(). Used as `awk "$(plan_awk_trim) …"`.
# Both ends are anchored: an unanchored `[[:space:]]+$` alternative strips
# interior whitespace runs and silently mangles table cells.
plan_awk_trim() {
    cat <<'AWK'
function trim(v) { gsub(/^[[:space:]]+|[[:space:]]+$/, "", v); return v }
AWK
}

# ── Associative arrays for bash 3.2 ─────────────────────────────────────────
# shellcheck source=planning/scripts/plan-map-lib.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/plan-map-lib.sh"

# ── Load-time initialisation ─────────────────────────────────────────────────
# Guarded: this library is sourced more than once per process, and re-running it
# would reset plan_error_count and record plan_cleanup as its own "prior" handler.
if [ -z "${PLAN_DOCUMENT_LIB_INITIALISED:-}" ]; then
    PLAN_DOCUMENT_LIB_INITIALISED=1
    plan_error_count=0
    plan_tmp_files=()
    plan_prior_exit_trap="$(trap -p EXIT)"
    trap plan_cleanup EXIT INT TERM
fi
