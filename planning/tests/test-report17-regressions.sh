#!/usr/bin/env bash
# MODE: DEV
# Regression tests for planning-skill report 17:
#   - tool defect B: multi-paragraph testing companions label every paragraph
#     (create-step-testing.sh split is portable across awk variants)
#   - tool defect A: paragraph auto-create refuses sparse/unlabeled sections
#     (update-plan-content.sh contiguity guard)
#   - P0: goals that change module state/schema/config WARN without a serve
#     acceptance condition
#   - P1: unregistered command literals WARN (FAIL under --complete) until
#     registered with their "when" context; no false positives on paths/::/
#     class literals
#   - Environment facts: seeded by create-plan.sh, required heading, and its
#     placeholder is registry-known (WARN mid-draft, never FAIL)

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

# A finding no longer stops the run: this file used a byte-identical copy of
# the library's reporter that exited on the first one, so a broken subject
# reported one failure and hid the rest.
fail() { t_fail "$*"; }
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-report17-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT


# ---- defect B: every paragraph of a multi-paragraph companion is labeled ----
goal="$temporary_root/pb-goal"
mkdir -p "$goal/steps"
printf '# Step: x\n\n## Instructions\n\n§ 5.1\nrun it\n' > "$goal/steps/01-step-x.md"
"$script_dir/create-step-testing.sh" "$goal" 01-step-x 'one\ntwo\n\nthree' >/dev/null
grep -Fqx '§ 2.1' "$goal/steps/01-step-x-testing.md" || fail 'companion lacks § 2.1'
grep -Fqx '§ 2.2' "$goal/steps/01-step-x-testing.md" || fail 'companion lacks § 2.2'
grep -Fqx '§ 2.3' "$goal/steps/01-step-x-testing.md" || fail 'companion lacks § 2.3'
grep -Fqx 'one' "$goal/steps/01-step-x-testing.md" || fail 'companion lacks paragraph one'
grep -Fqx 'three' "$goal/steps/01-step-x-testing.md" || fail 'companion lacks paragraph three'

# ---- defect A: auto-create refuses sparse sections ----
plan_a="$temporary_root/defect-a"
"$script_dir/create-plan.sh" "$plan_a" 'Defect A' >/dev/null
mkdir -p "$plan_a/01-goal/steps"
printf '# Step: x\n\n## Instructions\n\n§ 5.1\nrun it\n' > "$plan_a/01-goal/steps/01-step-x.md"
companion="$plan_a/01-goal/steps/01-step-x-testing.md"
# Sparse + trailing unlabeled content (defect-B output shape): refuse.
printf '# Verification: 01-step-x\n\n## Automated tests\n\n§ 2.1\nfirst\n\nunlabeled leftover\n' > "$companion"
if "$script_dir/update-plan-content.sh" -sp "$plan_a" 01-goal/01-step-x-testing 2.2 'auto' >/dev/null 2>&1; then
    fail 'auto-create accepted a companion with trailing unlabeled content'
fi
# Sparse with a label gap: refuse.
printf '# Verification: 01-step-x\n\n## Automated tests\n\n§ 2.1\nfirst\n\n§ 2.3\nthird\n' > "$companion"
if "$script_dir/update-plan-content.sh" -sp "$plan_a" 01-goal/01-step-x-testing 2.4 'auto' >/dev/null 2>&1; then
    fail 'auto-create accepted a section with a label gap'
fi
# Healthy contiguous section: auto-create works.
printf '# Verification: 01-step-x\n\n## Automated tests\n\n§ 2.1\nfirst\n' > "$companion"
"$script_dir/update-plan-content.sh" -sp "$plan_a" 01-goal/01-step-x-testing 2.2 'auto' >/dev/null
grep -Fqx '§ 2.2' "$companion" || fail 'healthy section did not auto-create § 2.2'

# ---- Environment facts: seeded, required, placeholder registry-known ----
plan_env="$temporary_root/env-facts"
"$script_dir/create-plan.sh" "$plan_env" 'Env facts' >/dev/null
grep -Fqx '## Environment facts' "$plan_env/plan-description.md" || fail 'create-plan.sh did not seed ## Environment facts'
grep -Fq '§ 9.1' "$plan_env/plan-description.md" || fail 'Environment facts section has no § 9.1'
if ! "$script_dir/validate-plan.sh" "$plan_env" >"$temporary_root/env-warn.log" 2>&1; then
    :
else
    fail 'fresh plan unexpectedly validated clean'
fi
grep -Fq 'registered placeholder' "$temporary_root/env-warn.log" || fail 'fresh plan did not WARN on authored placeholders'
if grep -Fq 'Environment facts' "$temporary_root/env-warn.log"; then
    fail 'environment-facts placeholder was flagged as missing or unregistered'
fi

# ---- P0 serve check + P1 command registry on a realistic plan ----
plan_b="$temporary_root/p0-p1"
"$script_dir/create-plan.sh" "$plan_b" 'P0 P1' >/dev/null
"$script_dir/add-goal.sh" "$plan_b" 01-enable-module 'Enable module' 'Module enabled and site serves' >/dev/null
"$script_dir/add-work-unit.sh" "$plan_b" W01 verification 'N/A' 'Config' 'N/A' \
    'Enable the module; run setup:upgrade' 'none' 01-enable-module '01-step-x' >/dev/null
"$script_dir/update-plan-content.sh" -sp "$plan_b" 01-enable-module/01-step-x 5.1 \
    'Edit etc/module.xml, etc/config.php, and db_schema.xml; run `setup:upgrade` then `bin/magento cache:flush`.' >/dev/null
"$script_dir/create-step-testing.sh" "$plan_b/01-enable-module" 01-step-x \
    'Run `bin/magento cache:flush` and verify HTTP 200' >/dev/null
# P1: both literal occurrences WARN (step + companion).
log="$temporary_root/validate.log"
"$script_dir/validate-plan.sh" "$plan_b" >"$log" 2>&1 || true
count="$(grep -c 'unregistered command literal' "$log" || true)"
[ "$count" -eq 2 ] || fail "expected 2 unregistered literals (step + companion), found $count: $(grep 'unregistered' "$log" || true)"
grep -Fq "unregistered command literal 'bin/magento cache:flush'" "$log" || fail 'expected cache:flush WARN'
# P0: HTTP 200 in the companion satisfies the serve check -> no WARN.
if grep -Fq 'still serves' "$log"; then
    fail 'state-changing goal with a serve phrase was still WARNed'
fi
# Register both literals; WARNs clear; a variant with extra args is covered.
"$script_dir/register-command.sh" "$plan_b" cache-flush 'bin/magento cache:flush' 'routine; after a constructor change' >/dev/null
"$script_dir/register-command.sh" "$plan_b" upgrade 'bin/magento setup:upgrade' 'module/schema/config change; usually enough' >/dev/null
registered_list="$("$script_dir/register-command.sh" "$plan_b" --list)"
case "$registered_list" in
    *cache-flush*) ;;
    *) fail 'register-command.sh --list missing entry' ;;
esac
grep -Fq 'upgrade' "$plan_b/commands.json" || fail 'commands.json missing registered entry'
"$script_dir/update-plan-content.sh" -sp "$plan_b" 01-enable-module/01-step-x-testing 2.1 \
    'Run `bin/magento cache:flush --page-cache` and verify HTTP 200' >/dev/null
"$script_dir/validate-plan.sh" "$plan_b" >"$log" 2>&1 || true
grep -q 'unregistered command literal' "$log" && fail 'registered literals still WARNed'
# Remove the serve phrase: P0 WARN returns.
"$script_dir/update-plan-content.sh" -sp "$plan_b" 01-enable-module/01-step-x-testing 2.1 \
    'Run `bin/magento cache:flush --page-cache`' >/dev/null
"$script_dir/validate-plan.sh" "$plan_b" >"$log" 2>&1 || true
grep -Fq 'still serves' "$log" || fail 'state-changing goal without a serve phrase was not WARNed'
# An unregistered command-shaped literal FAILs under --complete.
"$script_dir/update-plan-content.sh" -sp "$plan_b" 01-enable-module/01-step-x-testing 2.1 \
    'Run `vendor/bin/phpunit --filter FooTest` and verify HTTP 200' >/dev/null
"$script_dir/validate-plan.sh" --complete "$plan_b" >"$log" 2>&1 || true
grep -Fq "FAIL: 01-step-x-testing.md uses unregistered command literal 'vendor/bin/phpunit --filter FooTest'" "$log" \
    || fail 'unregistered literal did not FAIL under --complete'
"$script_dir/register-command.sh" "$plan_b" phpunit 'vendor/bin/phpunit --filter FooTest' 'after changing test code' >/dev/null
"$script_dir/validate-plan.sh" --complete "$plan_b" >"$log" 2>&1 || true
grep -q "unregistered command literal 'vendor/bin/phpunit --filter FooTest'" "$log" \
    && fail 'registered literal FAILed under --complete'
"$script_dir/register-command.sh" "$plan_b" --remove phpunit >/dev/null
# A bare tool word with an argument is not a candidate: rules 1-3 are the only
# entry points (report 21 §3), so unregistered `composer install` stays silent
# until composer is registered.
"$script_dir/update-plan-content.sh" -sp "$plan_b" 01-enable-module/01-step-x-testing 2.1 \
    'Run `composer install` and verify HTTP 200' >/dev/null
"$script_dir/validate-plan.sh" --complete "$plan_b" >"$log" 2>&1 || true
grep -q "unregistered command literal 'composer install'" "$log" \
    && fail 'bare tool word with args was flagged (rule 4 must not be an entry point)'

# ---- false-positive sweep: paths, ::-symbols, class literals, routes ----
plan_fp="$temporary_root/fp"
"$script_dir/create-plan.sh" "$plan_fp" 'False positives' >/dev/null
mkdir -p "$plan_fp/01-goal/steps"
printf '# Step: x\n\n## Ownership\n\n- File: `app/design/frontend/FakeTheme/templates/order/history.phtml`\n- Symbol: `AbstractItems::getColumnHtml()`\n\n## Instructions\n\n§ 5.1\nReference `Magento_Weee::email/items/price/row.phtml` and `Foo::class`.\nCall `GET /health`; assert `/health` returns 200.\nCheck `composer.json` and `app/code/Foo/Module/etc/module.xml`.\n' > "$plan_fp/01-goal/steps/01-step-x.md"
"$script_dir/validate-plan.sh" "$plan_fp" >"$log" 2>&1 || true
if grep -q 'unregistered command literal' "$log"; then
    fail "false-positive command literal: $(grep 'unregistered' "$log")"
fi

t_end
