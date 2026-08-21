#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# A section holding `- Label:` lines is field-shaped whatever the allow-list says,
# and rewriting it removes labels another mechanism may own -- `- Status:` in
# `## Verdict` belongs to the review-status gate. Refuse rather than destroy.
plan_refuse_field_section() {
    local file="$1" heading="$2" shape
    [ -f "$file" ] || return 0
    # A section whose body carries `- Label:` lines is field-shaped, and one
    # whose body OPENS with a table row is table-shaped. A narrative section may
    # still contain a table paragraph, which is why the discriminator is the
    # first body line rather than the presence of a pipe anywhere.
    shape="$(awk -v want="$heading" '
        $0 == want { inside = 1; next }
        inside && /^## / { exit }
        inside && /^[[:space:]]*$/ { next }
        inside && /^- [A-Z][^:]*:/ { fields++ }
        inside && first == "" { first = ($0 ~ /^\|/) ? "table" : "other" }
        END {
            if (fields > 0) print "fields"
            else if (first == "table") print "table"
            else print "narrative"
        }' "$file")"
    case "$shape" in
        fields)
            plan_die "Section '$heading' holds fields (- Label: value); rewriting it would remove them, and a field there may belong to another gate. Write one field at a time with --field." 65 ;;
        table)
            plan_die "Section '$heading' is a table; rewriting it as paragraphs would discard every row. Use the helper that owns that table." 65 ;;
    esac
}
