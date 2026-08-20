#!/usr/bin/env bash
# lib-process.sh - process-group teardown and the post-run browser/server audit
# for one benchmark case. Sourced, never executed.
#
# Extracted from the quoted `start-worker.sh` heredoc `setup-benchmark.sh` used
# to emit. Three jobs, in the order the case runner needs them:
#
#   1. process_pattern           the audited process families, one ERE
#   2. process_audit_probe       is `ps`+`setsid` present? record it before the run
#   3. process registry helpers  fallback roots when setsid is unavailable
#   4. process_kill_tree         recursive TERM/KILL of one pid's descendants
#   5. process_cleanup_on_signal INT/TERM handler; exits 130
#   6. process_audit             post-run verdict: pass | fail | unavailable
#
# `process_cleanup_on_signal` is a trap handler, so it necessarily reads the
# caller's live state rather than arguments: the case runner owns
# PROCESS_CLEANUP_CHILD_PID and PROCESS_CLEANUP_GROUP_ID, sets each as soon as
# the worker launches, and clears the pid once the worker has been reaped. Both
# must exist (possibly empty) before `trap process_cleanup_on_signal INT TERM`.
#
# `process_audit` prints its verdict on stdout and writes its evidence files as
# a side effect, so the caller threads the verdict explicitly:
#
#   PROCESS_AUDIT="$(process_audit "$STATE_FILE" "$GROUP_ID" "$CASE_ROOT")"
#
# The audit deliberately scopes itself to the worker's OWN process group: an
# unrelated host browser, or another parallel worker, must never taint this run.

# The audited families. Kept as one ERE so the awk filter stays a single pass.
process_pattern() {
    printf '%s\n' '(google-chrome|chromium|firefox|playwright|geckodriver|chromedriver|selenium|http\.server|php -S|vite|webpack-dev-server|node.*(serve|vite)|npm.*(run|exec).*(dev|serve))'
}

# Record auditability BEFORE the worker runs: a host missing ps or setsid can
# never produce a trustworthy verdict, and discovering that afterwards would be
# indistinguishable from a clean run.
process_audit_probe() {
    local state_file="$1"
    if command -v ps >/dev/null 2>&1 && command -v setsid >/dev/null 2>&1; then
        echo "available" > "$state_file"
    else
        echo "unavailable:ps-and-setsid-required" > "$state_file"
    fi
}

process_registry_init() {
    local registry="$1"
    mkdir -p "$(dirname "$registry")"
    : > "$registry"
}

process_registry_append() {
    local registry="$1" role="$2" pid="$3" output="$4"
    shift 4
    local preview started_at
    started_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    preview="$*"
    preview="$(printf '%s' "$preview" | tr '\t\n' '  ')"
    printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$role" "$pid" "$$" "$started_at" "$output" "$preview" >> "$registry"
}

process_prepare_no_detach_shims() {
    local shim_dir="$1" name
    mkdir -p "$shim_dir"
    for name in nohup setsid open; do
        cat > "$shim_dir/$name" <<'SHIM'
#!/usr/bin/env bash
printf '%s\n' "benchmark: detached subprocess launch is not allowed under registry isolation: ${0##*/}" >&2
printf '%s\n' "Use a foreground command so the runner can clean up and audit descendants." >&2
exit 78
SHIM
        chmod +x "$shim_dir/$name"
    done
}

process_descendants() {
    local pid="$1" child
    [ "$pid" -gt 0 ] 2>/dev/null || return 0
    while read -r child; do
        [ -n "$child" ] || continue
        printf '%s\n' "$child"
        process_descendants "$child"
    done < <(ps -eo pid=,ppid= 2>/dev/null | awk -v parent="$pid" '$2 == parent { print $1 }')
}

process_kill_tree() {
    local pid="$1"
    local signal="${2:-TERM}"
    local child

    [ "$pid" -gt 0 ] 2>/dev/null || return 0
    if command -v ps >/dev/null 2>&1; then
        while read -r child; do
            [ -n "$child" ] || continue
            process_kill_tree "$child" "$signal"
        done < <(ps -eo pid=,ppid= | awk -v parent="$pid" '$2 == parent {print $1}')
    fi
    kill -"$signal" "$pid" 2>/dev/null || true
}

PROCESS_CLEANUP_CHILD_PID=""
PROCESS_CLEANUP_GROUP_ID=""
process_cleanup_on_signal() {
    trap - INT TERM
    if [ -n "$PROCESS_CLEANUP_CHILD_PID" ]; then
        if [ -n "$PROCESS_CLEANUP_GROUP_ID" ]; then
            kill -TERM -- "-$PROCESS_CLEANUP_GROUP_ID" 2>/dev/null || true
        fi
        process_kill_tree "$PROCESS_CLEANUP_CHILD_PID" TERM
        sleep 1
        if [ -n "$PROCESS_CLEANUP_GROUP_ID" ]; then
            kill -KILL -- "-$PROCESS_CLEANUP_GROUP_ID" 2>/dev/null || true
        fi
        process_kill_tree "$PROCESS_CLEANUP_CHILD_PID" KILL
    fi
    exit 130
}

# Prints pass | fail | unavailable on stdout; writes process-after.txt and, when
# the audit ran, process-new.txt beside it.
process_audit() {
    local state_file="$1" group_id="$2" case_root="$3" isolation_mode="${4:-setsid}" registry="${5:-}"
    local verdict="unavailable"
    if [ "$isolation_mode" = registry ] && [ -n "$registry" ] && [ -f "$registry" ]; then
        : > "$case_root/process-after.txt"
        while IFS="$(printf '\t')" read -r role pid ppid started output preview; do
            [ -n "${pid:-}" ] || continue
            {
                ps -p "$pid" -o pid=,ppid=,comm=,args= 2>/dev/null || true
                while read -r child; do
                    [ -n "$child" ] || continue
                    ps -p "$child" -o pid=,ppid=,comm=,args= 2>/dev/null || true
                done <<DESCENDANTS
$(process_descendants "$pid")
DESCENDANTS
            } | awk -v role="$role" 'NF { print role "\t" $0 }' >> "$case_root/process-after.txt"
        done < "$registry"
        awk -v pattern="$(process_pattern)" 'tolower($0) ~ pattern' "$case_root/process-after.txt" |
            sort > "$case_root/process-new.txt" || true
        if [ -s "$case_root/process-new.txt" ]; then
            verdict="fail"
        else
            verdict="pass"
        fi
    elif [ "$(cat "$state_file")" = "available" ] && [ -n "$group_id" ]; then
        ps -eo pid=,ppid=,pgid=,comm=,args= |
            awk -v group="$group_id" -v pattern="$(process_pattern)" '$3 == group && tolower($0) ~ pattern' |
            sort > "$case_root/process-after.txt" || true
        cp "$case_root/process-after.txt" "$case_root/process-new.txt"
        if [ -s "$case_root/process-new.txt" ]; then
            verdict="fail"
        else
            verdict="pass"
        fi
    else
        cp "$state_file" "$case_root/process-after.txt"
    fi
    printf '%s\n' "$verdict"
}
