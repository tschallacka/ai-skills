#!/usr/bin/env bash
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
