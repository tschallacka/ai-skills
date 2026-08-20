#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
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
