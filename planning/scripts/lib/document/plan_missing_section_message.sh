#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: DEV
# A section form can only rewrite a section the document already holds, so the
# refusal has to say which sections it does hold. Without that, a valid section
# id that this particular file never received reads as a broken helper: a
# reviewer hit `-ss ... browser-verification` on a companion created without
# browser content, was told only "not found exactly once", and inferred a remedy
# (re-create with --overwrite) that cannot work -- create-step-testing.sh emits
# `## Automated tests` and nothing else.
plan_missing_section_message() {
    local file="$1" heading="$2" present
    present="$(grep '^## ' "$file" 2>/dev/null | tr '\n' ' ')"
    printf '%s not found in %s' "$heading" "${file##*/}"
    if [ -n "$present" ]; then
        printf '; it has: %s' "$present"
    fi
    printf -- ' -- a section form rewrites a section that already exists, it cannot add one. '
    case "$file" in
        *-testing.md) printf 'A testing companion carries only the sections its creator emitted; create-step-testing.sh emits "## Automated tests".' ;;
        *) printf 'Create the document with the helper that owns it, then rewrite the section.' ;;
    esac
}
