#!/usr/bin/env bash
plan_progress_bar() {
    local completed="$1" total="$2" width="${3:-20}" percent filled empty
    percent="$(plan_progress_percent "$completed" "$total")"
    filled=$(( percent * width / 100 ))
    empty=$(( width - filled ))
    printf '%s%s\n' \
        "$(printf '%*s' "$filled" '' | tr ' ' '#')" \
        "$(printf '%*s' "$empty" '' | tr ' ' '-')"
}
