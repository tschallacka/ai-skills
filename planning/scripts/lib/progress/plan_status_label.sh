#!/usr/bin/env bash
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
