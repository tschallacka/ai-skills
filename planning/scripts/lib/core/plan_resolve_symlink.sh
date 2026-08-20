#!/usr/bin/env bash
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
