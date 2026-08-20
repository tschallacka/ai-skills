#!/usr/bin/env bash
# MODE: PROD
# plan-content-diff-lib — the `plan-content.sh diff` subcommand.
#
# Sourced by plan-content.sh, never executed. Extracted so plan-content.sh
# stays under the 400-line limit in CODE-STYLE.md §3; the diff subcommand is
# self-contained (it shares no state with the other subcommands beyond the
# plan_die/plan_require_directory helpers the caller has already sourced).
#
# Usage: sourced — call plan_content_diff <plan-dir> <git-ref> [text|json]
set -euo pipefail
export LC_ALL=C

plan_content_diff() {
    local plan_dir git_ref format repo_root plan_abs plan_rel changed
        [ "$#" -ge 2 ] && [ "$#" -le 3 ] || usage
        plan_dir="$1"; git_ref="$2"; format="${3:-text}"
        plan_require_directory "$plan_dir"
        case "$format" in text|json) ;; *) plan_die "Unknown format: $format (use text or json)" ;; esac
        # The plan may be a subdirectory of a repo that covers it (e.g. a
        # git-excluded .plans root or an initiative repo holding sibling plans),
        # so walk up to the enclosing repo and scope the diff to the plan dir.
        repo_root="$(git -C "$plan_dir" rev-parse --show-toplevel 2>/dev/null || true)"
        if [ -z "$repo_root" ]; then
            plan_die "plan directory is not inside a git repository: $plan_dir"
        fi
        repo_root="$(cd "$repo_root" && pwd -P)"
        plan_abs="$(cd "$plan_dir" && pwd -P)"
        plan_rel="."
        if [ "$plan_abs" != "$repo_root" ]; then
            plan_rel="${plan_abs#"$repo_root"/}"
        fi
        # name-only, scoped to the plan subtree (relative paths).
        changed="$(git -C "$repo_root" diff --name-only "$git_ref" -- "$plan_rel" 2>/dev/null | sed "s#^$plan_rel/##" | grep -v '^$' || true)"
        if [ -z "$changed" ]; then
            git -C "$repo_root" rev-parse -q --verify "$git_ref" >/dev/null 2>&1 \
                || plan_die "git ref not found: $git_ref"
            printf 'No plan documents changed since %s\n' "$git_ref"
            return 0
        fi
        # A changed paragraph label is usually an UNCHANGED diff line, so labels
        # come from the document, not the diff. POSIX awk (CODE-STYLE.md §1)
        # with the default single-character RS, for mawk/BSD-awk parity.
        paragraph_labels() {
            local doc="$1" path
            path="$doc"
            [ "$plan_rel" = "." ] || path="$plan_rel/$doc"
            git -C "$repo_root" diff -U0 "$git_ref" -- "$path" 2>/dev/null |
                awk -v doc="$plan_abs/$doc" '
                    # After "@@ -a,b +c,d @@" the new-file side is numbered from
                    # c. POSIX ERE has no \d, hence [0-9]+.
                    /^@@/ {
                        if (match($0, /\+[0-9]+/)) {
                            new_line = substr($0, RSTART + 1, RLENGTH - 1) + 0
                            in_hunk = 1
                        } else {
                            in_hunk = 0
                        }
                        next
                    }
                    in_hunk != 1 { next }
                    {
                        lead = substr($0, 1, 1)
                        # "+++" is the file header, not added content.
                        if (lead == "+" && substr($0, 1, 3) != "+++") {
                            order[++changed_lines] = new_line
                            is_changed[new_line] = 1
                        }
                        # A "-" line does not exist on the new-file side.
                        if (lead != "-") new_line++
                    }
                    END {
                        number = 0
                        while ((getline document_line < doc) > 0) {
                            number++
                            if (document_line ~ /^§ [0-9]+\.[0-9]+$/) label = document_line
                            if (number in is_changed) label_of[number] = label
                        }
                        close(doc)
                        # First-appearance order, deduplicated. A line before the
                        # first label has none and is skipped.
                        for (position = 1; position <= changed_lines; position++) {
                            found = label_of[order[position]]
                            if (found == "") continue
                            if (found in seen) continue
                            seen[found] = 1
                            print found
                        }
                    }
                '
        }
        # Built byte by byte rather than with gsub: POSIX leaves a backslash in
        # a gsub replacement undefined unless it precedes "&" or "\", and
        # busybox awk really does swallow the one in "\\u00a7".
        json_escape_value() {
            printf '%s' "$1" | awk '
                {
                    backslash = "\\"
                    escaped = ""
                    position = 1
                    total = length($0)
                    while (position <= total) {
                        # LC_ALL=C, so substr() indexes bytes and "§" is the two
                        # bytes C2 A7. Any other non-ASCII byte passes through.
                        if (substr($0, position, 2) == "§") {
                            escaped = escaped backslash "u00a7"
                            position = position + 2
                            continue
                        }
                        character = substr($0, position, 1)
                        if (character == backslash) escaped = escaped backslash backslash
                        else if (character == "\"") escaped = escaped backslash "\""
                        else escaped = escaped character
                        position = position + 1
                    }
                    printf "%s", escaped
                }
            '
        }
        if [ "$format" = json ]; then
            printf '{"ref": "%s", "documents": [' "$(json_escape_value "$git_ref")"
            document_first=true
            while IFS= read -r changed_document; do
                [ -n "$changed_document" ] || continue
                [ "$document_first" = true ] || printf ', '
                document_first=false
                printf '{"document": "%s", "paragraphs": [' "$(json_escape_value "$changed_document")"
                label_first=true
                while IFS= read -r paragraph_label; do
                    [ -n "$paragraph_label" ] || continue
                    [ "$label_first" = true ] || printf ', '
                    label_first=false
                    printf '"%s"' "$(json_escape_value "$paragraph_label")"
                done <<< "$(paragraph_labels "$changed_document")"
                printf ']}'
            done <<< "$changed"
            printf ']}\n'
        else
            while IFS= read -r changed_document; do
                [ -n "$changed_document" ] || continue
                printf '## %s\n' "$changed_document"
                paragraph_labels "$changed_document"
            done <<< "$changed"
        fi
}
