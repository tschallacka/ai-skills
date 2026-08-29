#!/usr/bin/env bash
# MODE: DEV
set -euo pipefail


source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
# Report the failing expression: these assertions are bare `[ ... ]` under
# set -e, which otherwise exits 1 in silence.
t_trap_assertions

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
env_tool="$repo_dir/planning/scripts/plan-env.sh"
tmp="$(mktemp -d)"
tmp="$(cd "$tmp" && pwd -P)"
trap 'rm -rf "$tmp"' EXIT

plans_root="$tmp/plans"
plan_root="$plans_root/demo-plan"
PLANS_ROOT="$plans_root" "$repo_dir/planning/scripts/create-plan.sh" "$plan_root" 'Demo plan' >/dev/null

[ -f "$plans_root/.env" ]
[ -f "$plan_root/.env" ]
[ "$(t_stat_mode "$plans_root/.env")" = 600 ]
[ "$(t_stat_mode "$plan_root/.env")" = 600 ]
"$env_tool" check "$plan_root" "$plans_root" >/dev/null

global_before="$(t_sha256 "$plans_root/.env")"
plan_before="$(t_sha256 "$plan_root/.env")"
printf '%s\n' unrelated > "$plan_root/keep.me"
PLANS_ROOT="$plans_root" "$env_tool" write-plan "$plan_root" "$plans_root"
[ "$global_before" = "$(t_sha256 "$plans_root/.env")" ]
[ "$plan_before" = "$(t_sha256 "$plan_root/.env")" ]
[ -f "$plan_root/keep.me" ]

helper_output="$tmp/helper-output"
cat > "$tmp/helper.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
plan_root="$1"
plans_root="$2"
tool="$3"
"$tool" check "$plan_root" "$plans_root" >/dev/null
set -a
source "$($tool path global "$plan_root" "$plans_root")"
source "$($tool path plan "$plan_root" "$plans_root")"
set +a
printf '%s\n' "$PLAN_NAME|$PLAN_DESCRIPTION_FILE|$PLAN_STEPS_ROOT"
EOF
chmod 700 "$tmp/helper.sh"
"$tmp/helper.sh" "$plan_root" "$plans_root" "$env_tool" > "$helper_output"
grep -Fqx "demo-plan|$plan_root/plan-description.md|$plan_root/steps" "$helper_output"
usage_file="$tmp/plan-env-usage"
if "$env_tool" >"$usage_file" 2>&1; then
    printf '%s\n' 'missing plan-env arguments were accepted' >&2
    exit 1
fi
grep -Fq 'check <plan-root>' "$usage_file"
grep -Fq 'path global|plan' "$usage_file"

bad="$tmp/bad-plan"
mkdir -p "$bad"
cp "$plan_root/.env" "$bad/.env"
printf 'EVIL=$(touch %q)\n' "$tmp/sentinel" >> "$bad/.env"
chmod 600 "$bad/.env"
if "$env_tool" check "$bad" "$plans_root" >/dev/null 2>&1; then
    printf '%s\n' 'unsafe manifest was accepted' >&2
    exit 1
fi
[ ! -e "$tmp/sentinel" ]

cp "$plan_root/.env" "$bad/.env"
chmod 644 "$bad/.env"
if "$env_tool" check "$bad" "$plans_root" >/dev/null 2>&1; then
    printf '%s\n' 'weak manifest mode was accepted' >&2
    exit 1
fi

cp "$plan_root/.env" "$bad/.env"
chmod 600 "$bad/.env"
printf 'UNEXPECTED=value\n' >> "$bad/.env"
if "$env_tool" check "$bad" "$plans_root" >/dev/null 2>&1; then
    printf '%s\n' 'unexpected manifest key was accepted' >&2
    exit 1
fi

# A pipe in a value is refused by the checker's copy of the character rule —
# the third site carrying it (the reader copy is pinned by the parity block
# below), so a rule weakened there fails here rather than silently.
grep -v '^PLAN_NAME=' "$plan_root/.env" > "$bad/.env"
printf 'PLAN_NAME=a|b\n' >> "$bad/.env"
chmod 600 "$bad/.env"
if "$env_tool" check "$bad" "$plans_root" >/dev/null 2>&1; then
    printf '%s\n' 'manifest value with a pipe was accepted' >&2
    exit 1
fi

cp "$plan_root/.env" "$bad/.env"
chmod 600 "$bad/.env"
printf 'PLAN_ENV_SCHEMA_VERSION=0\n' >> "$bad/.env"
if "$env_tool" check "$bad" "$plans_root" >/dev/null 2>&1; then
    printf '%s\n' 'stale manifest schema was accepted' >&2
    exit 1
fi

cp "$plan_root/.env" "$bad/.env"
chmod 600 "$bad/.env"
if "$env_tool" check "$bad" "$plans_root" >/dev/null 2>&1; then
    printf '%s\n' 'foreign-root manifest was accepted' >&2
    exit 1
fi

cp "$plan_root/.env" "$bad/.env"
chmod 600 "$bad/.env"
t_sed_i "s|^PLAN_STEPS_ROOT=.*|PLAN_STEPS_ROOT=$tmp/outside|" "$bad/.env"
if "$env_tool" check "$bad" "$plans_root" >/dev/null 2>&1; then
    printf '%s\n' 'foreign derived path was accepted' >&2
    exit 1
fi

cp "$plan_root/.env" "$bad/.env"
chmod 600 "$bad/.env"
t_sed_i "s|^PLANS_ROOT=.*|PLANS_ROOT=$tmp/other-plans|" "$bad/.env"
if "$env_tool" check "$bad" "$plans_root" >/dev/null 2>&1; then
    printf '%s\n' 'mismatched local root was accepted' >&2
    exit 1
fi

cp "$plan_root/.env" "$bad/.env"
chmod 600 "$bad/.env"
printf 'PLAN_NAME=duplicate\n' >> "$bad/.env"
if "$env_tool" check "$bad" "$plans_root" >/dev/null 2>&1; then
    printf '%s\n' 'duplicate manifest key was accepted' >&2
    exit 1
fi

cp "$plan_root/.env" "$bad/.env"
chmod 600 "$bad/.env"
t_sed_i 's|^PLAN_NAME=.*|PLAN_NAME=$HOME|' "$bad/.env"
if "$env_tool" check "$bad" "$plans_root" >/dev/null 2>&1; then
    printf '%s\n' 'variable expansion was accepted' >&2
    exit 1
fi

if nobody_uid="$(id -u nobody 2>/dev/null)" && chown "$nobody_uid" "$bad/.env" 2>/dev/null; then
    if "$env_tool" check "$bad" "$plans_root" >/dev/null 2>&1; then
        printf '%s\n' 'foreign manifest owner was accepted' >&2
        exit 1
    fi
fi

mv "$bad/.env" "$bad/.env.real"
ln -s .env.real "$bad/.env"
if "$env_tool" check "$bad" "$plans_root" >/dev/null 2>&1; then
    printf '%s\n' 'symlinked manifest was accepted' >&2
    exit 1
fi

# ── Duplication parity (T6) ───────────────────────────────────────────────────
# plan-env.sh sources no library on purpose — the manifest gate verifies a skill
# root before any library is trusted — so read_pinned_snapshot_repo here
# re-implements lib/core/plan_snapshot_repo.sh, and the GNU/BSD stat probe is
# carried twice (here and in plan_stat_probe.sh). The copies cannot share code,
# so this pins them to identical behaviour instead: same verdict on the same
# manifest fixtures, and the same stat flag pair. The known deliberate
# difference is return-status shape only (this reader reports absence as empty
# output with status 0; the library reports it as status 1), so parity compares
# output, not status.
parity="$tmp/parity"
mkdir -p "$parity/with-pin" "$parity/hostile" "$parity/pipe" "$parity/empty" "$parity/absent"
printf 'PLAN_SNAPSHOT_REPO=%q\n' "$plans_root/a repo with spaces" > "$parity/with-pin/.env"
printf "PLAN_SNAPSHOT_REPO='/tmp/x\$(touch y);rm -r / | true'\n" > "$parity/hostile/.env"
# A pipe is the only metacharacter here: each refused class needs an isolated
# fixture, or a rule weakened on one character stays hidden behind another.
printf "PLAN_SNAPSHOT_REPO='/tmp/a|b'\n" > "$parity/pipe/.env"
printf 'PLAN_SNAPSHOT_REPO=\n' > "$parity/empty/.env"

awk '/^read_pinned_snapshot_repo\(\) \{/{f=1} f{print} f&&/^\}$/{exit}' "$env_tool" \
    > "$parity/reader-fn.sh"
grep -Fq 'read_pinned_snapshot_repo()' "$parity/reader-fn.sh"
{
    printf '#!/usr/bin/env bash\nset -uo pipefail\n'
    cat "$parity/reader-fn.sh"
    printf '\nread_pinned_snapshot_repo "$1"\n'
} > "$parity/reader.sh"
mkdir -p "$parity/hostile" "$parity/pipe"
# Both runners always exit 0: parity is compared through their output, and the
# library's deliberate status-1 for an absent pin would otherwise fire this
# test's ERR trap on every clean run. A broken runner shows up as an output
# mismatch below.
run_reader() { "$BASH" "$parity/reader.sh" "$1" 2>/dev/null; return 0; }
run_library_reader() {
    "$BASH" -c '
        set -uo pipefail
        source "'"$repo_dir"'/planning/scripts/lib/core/plan_snapshot_repo.sh"
        plan_snapshot_repo "$1"
    ' parity-library "$1" 2>/dev/null || true
}

# Values are captured into variables before comparing: a bare [ ] whose
# operands come straight from $( ) does not abort under set -e when it fails,
# and this epilogue would then print PASS over a failed assertion.
reader_value="$(run_reader "$parity/with-pin/.env")"
[ "$reader_value" = "$plans_root/a repo with spaces" ]
for parity_case in with-pin hostile pipe empty absent; do
    reader_value="$(run_reader "$parity/$parity_case/.env")"
    library_value="$(run_library_reader "$parity/$parity_case")"
    [ "$reader_value" = "$library_value" ]
done

# The stat probes must name the same set of formats, so a format fix on one
# side cannot leave the other side probing something the other abandoned.
# Compared as a set: the load-time probe lines legitimately repeat a format the
# function definitions also use.
env_stat_flags="$(grep -oE "stat -[cf] '[^']+'" "$env_tool" | LC_ALL=C sort -u | tr '\n' ';')"
probe_stat_flags="$(grep -oE "stat -[cf] '[^']+'" "$repo_dir/planning/scripts/lib/core/plan_stat_probe.sh" | LC_ALL=C sort -u | tr '\n' ';')"
[ -n "$env_stat_flags" ]
[ "$env_stat_flags" = "$probe_stat_flags" ]

printf '%s\n' 'test-plan-env: PASS'
