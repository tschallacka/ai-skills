#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
env_tool="$repo_dir/planning/scripts/plan-env.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

plans_root="$tmp/plans"
plan_root="$plans_root/demo-plan"
PLANS_ROOT="$plans_root" "$repo_dir/planning/scripts/create-plan.sh" "$plan_root" 'Demo plan' >/dev/null

[ -f "$plans_root/.env" ]
[ -f "$plan_root/.env" ]
[ "$(stat -c '%a' "$plans_root/.env")" = 600 ]
[ "$(stat -c '%a' "$plan_root/.env")" = 600 ]
"$env_tool" check "$plan_root" "$plans_root" >/dev/null

global_before="$(sha256sum "$plans_root/.env")"
plan_before="$(sha256sum "$plan_root/.env")"
printf '%s\n' unrelated > "$plan_root/keep.me"
PLANS_ROOT="$plans_root" "$env_tool" write-plan "$plan_root" "$plans_root"
[ "$global_before" = "$(sha256sum "$plans_root/.env")" ]
[ "$plan_before" = "$(sha256sum "$plan_root/.env")" ]
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
sed -i "s|^PLAN_STEPS_ROOT=.*|PLAN_STEPS_ROOT=$tmp/outside|" "$bad/.env"
if "$env_tool" check "$bad" "$plans_root" >/dev/null 2>&1; then
    printf '%s\n' 'foreign derived path was accepted' >&2
    exit 1
fi

cp "$plan_root/.env" "$bad/.env"
chmod 600 "$bad/.env"
sed -i "s|^PLANS_ROOT=.*|PLANS_ROOT=$tmp/other-plans|" "$bad/.env"
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
sed -i 's|^PLAN_NAME=.*|PLAN_NAME=$HOME|' "$bad/.env"
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

printf '%s\n' 'test-plan-env: PASS'
