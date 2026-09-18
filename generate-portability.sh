#!/usr/bin/env bash
# MODE: DEV
# generate-portability — write PORTABILITY.md from portability-rules.json.
#
# Usage:
#   generate-portability.sh [--check]
#
# --check writes to a temp file and diffs instead of overwriting, exit 1 when
# PORTABILITY.md is stale. planning/tests/test-portability-contract.sh runs it.
#
# PORTABILITY.md is generated so the gotchas cannot drift from the registry and
# so no agent has to rediscover one by tripping over it in an unrelated file.
# Edit portability-rules.json, or the `# PORTABILITY:` comment at the site.
set -euo pipefail
export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism. Placed immediately after repo_root is
# computed, before rules/output are set -- as early as structurally possible,
# matching every prior goal's precedent. This script already declares
# set -euo pipefail above, matching what sourcing plan-core-lib.sh itself
# wants, so no call-site set +e fix is needed here. generate-portability.sh
# lives at the repository root itself, one level shallower than
# planning/scripts, so the relative path to plan-core-lib.sh crosses one
# directory level down, matching pre-push-check.sh/setup-dev-env.sh's own
# precedent.
gp_script_dir="$repo_root"
# plan-core-lib.sh is generated (gitignored) by build-plan-libs.sh, so it does
# not exist on a genuinely fresh checkout -- guard the source+exec on it
# already being present, unconditionally falling through to this script's own
# bash implementation when it is not, matching B346's fix for
# build-plan-libs.sh's own self-referential case.
if [ -f "$gp_script_dir/planning/scripts/plan-core-lib.sh" ]; then
    source "$gp_script_dir/planning/scripts/plan-core-lib.sh"
    plan_exec_compiled_binary_if_present generate-portability "$gp_script_dir" "$@"
fi

printf '%s: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it\n' "generate-portability" >&2
exit 69
