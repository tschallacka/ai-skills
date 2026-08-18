#!/usr/bin/env bash
# Regression tests for planning-skill reports 20 + 21 — the language-agnostic
# command-literal detector:
#   - executable-bit signal: a path-shaped token that resolves to a
#     non-directory with the executable bit is a candidate (rule 2)
#   - bin-like parent: vendor/bin/... is a candidate by shape alone (rule 3)
#   - universal core: `sudo`, `npx`, `make`, ... are candidates without a
#     registration (rule 1)
#   - registry vocabulary: a registered command's first token teaches the
#     detector that tool word (rule 1 / report 20 §3)
#   - never-executable extensions: an executable .xml or .php is NOT a command
#     (rule 5 — the sloppy-packaging leak and Magento's exec-bit generated code)
#   - citation suffixes: `:line` / `#Lnn` are never commands (rule 6)
#   - route/prose shapes: `/sales/order/history/`, `GET /health` (rule 7)
#   - report 21 §3: rules 1-3 are the ONLY entry points. Arguments strengthen
#     a qualifying span but never qualify one on their own — a bare tool word
#     like `pytest -q` is silent until pytest is registered, and absent paths
#     without shape are silent too.

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-report20-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

# A checkout-shaped tree: the plan lives under .plans/, vendor/ next to it,
# so relative paths resolve exactly as they do when validating a real plan.
repo="$temporary_root/repo"
mkdir -p "$repo/.plans" "$repo/vendor/dummy" "$repo/generated/code"
tool="$repo/vendor/dummy/tool.sh"
xml_leak="$repo/vendor/dummy/config.xml"
generated_php="$repo/generated/code/Foo.php"
touch "$tool" "$xml_leak" "$generated_php"
chmod +x "$tool" "$xml_leak" "$generated_php"

plan="$repo/.plans/p"
"$script_dir/create-plan.sh" "$plan" 'P1 rules' >/dev/null
"$script_dir/add-goal.sh" "$plan" 01-goal 'G' 'an outcome' >/dev/null
"$script_dir/add-work-unit.sh" "$plan" W01 source app/code/Foo/A.php 'A' N/A 'a' — 01-goal 01-step-x >/dev/null
add_para() {  # <paragraph-id> <content>
    "$script_dir/update-plan-content.sh" -sp "$plan" 01-goal/01-step-x "$1" "$2" >/dev/null 2>&1
}
add_para 5.1 \
    'Run `vendor/dummy/tool.sh --verbose` and bare `vendor/dummy/tool.sh`; run `vendor/bin/phpunit --filter FooTest`; cite `vendor/dummy/config.xml` and `generated/code/Foo.php`; mention `vendor/dummy/nonexistent-cmd`.'
add_para 5.2 \
    'Snippet cites: `vendor/dummy/config.xml:12`, `Item.php:183`, and `README.md#L12`.'
add_para 5.3 \
    'Routes: `/sales/order/history/` and `/zplusoutfits/outfit/addproductstocart`.'
add_para 5.4 'Call `GET /health`; assert `/health` returns 200.'
add_para 5.5 'Run `pytest -q`; then `sudo apt install -y php-cli` and `npx prettier --write`.'

(
    cd "$repo"
    "$script_dir/validate-plan.sh" "$plan" >"$temporary_root/v.log" 2>&1 || true
)
for expected in \
    "unregistered command literal 'vendor/dummy/tool.sh --verbose'" \
    "unregistered command literal 'vendor/dummy/tool.sh'" \
    "unregistered command literal 'vendor/bin/phpunit --filter FooTest'" \
    "unregistered command literal 'sudo apt install -y php-cli'" \
    "unregistered command literal 'npx prettier --write'"; do
    grep -Fq "$expected" "$temporary_root/v.log" \
        || fail "expected WARN missing: $expected"
done
for quiet in \
    "unregistered command literal 'vendor/dummy/config.xml'" \
    "unregistered command literal 'generated/code/Foo.php'" \
    "unregistered command literal 'vendor/dummy/config.xml:12'" \
    "unregistered command literal 'Item.php:183'" \
    "unregistered command literal 'README.md#L12'" \
    "unregistered command literal 'vendor/dummy/nonexistent-cmd'" \
    "unregistered command literal 'pytest -q'" \
    "unregistered command literal 'GET /health'" \
    "unregistered command literal '/sales/order/history/'" \
    "unregistered command literal '/zplusoutfits/outfit/addproductstocart'"; do
    if grep -Fq "$quiet" "$temporary_root/v.log"; then
        fail "false positive: $quiet"
    fi
done

# Rule 5's source of truth is never-executable-extensions.json (jq-matched):
# it must be a plain array of dotted extensions covering the cases above.
extensions_file="$script_dir/../never-executable-extensions.json"
jq -e 'type == "array" and (all(.[]; type == "string" and startswith("."))) and (index(".xml") != null) and (index(".php") != null) and (index(".sql") != null)' \
    "$extensions_file" >/dev/null \
    || fail 'never-executable-extensions.json is not a dotted-extension array (needs .xml/.php/.sql)'

# Registry vocabulary: registering `pytest -q` makes pytest a word — the exact
# variant with extra args is covered by the registered command, and a
# *different* pytest invocation with a flag still WARNs.
"$script_dir/register-command.sh" "$plan" pytest 'pytest -q' 'fast failure-only run' >/dev/null
"$script_dir/update-plan-content.sh" -sp "$plan" 01-goal/01-step-x 5.5 \
    'Run `pytest -q --tb=short` and `sudo apt install -y php-cli`.' >/dev/null 2>&1
(
    cd "$repo"
    "$script_dir/validate-plan.sh" "$plan" >"$temporary_root/v2.log" 2>&1 || true
)
grep -Fq "unregistered command literal 'pytest -q --tb=short'" "$temporary_root/v2.log" \
    && fail 'registered pytest still WARNed'
grep -Fq "unregistered command literal 'sudo apt install -y php-cli'" "$temporary_root/v2.log" \
    || fail 'core-word literal lost after registration pass'
"$script_dir/update-plan-content.sh" -sp "$plan" 01-goal/01-step-x 5.5 \
    'Run `pytest --forked` and `sudo apt install -y php-cli`.' >/dev/null 2>&1
(
    cd "$repo"
    "$script_dir/validate-plan.sh" "$plan" >"$temporary_root/v3.log" 2>&1 || true
)
grep -Fq "unregistered command literal 'pytest --forked'" "$temporary_root/v3.log" \
    || fail 'unregistered variant of a registered tool not WARNed'

echo 'report 20/21 regressions: PASS'