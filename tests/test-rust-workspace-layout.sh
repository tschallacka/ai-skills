#!/usr/bin/env bash
# MODE: DEV
# test-rust-workspace-layout.sh - keep the workspace's generated-file contract
# aligned between Cargo, .gitignore, and the maintainer documentation.

set -euo pipefail
export LC_ALL=C

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$root/planning/tests/lib-test.sh"
t_begin

t_assert_eq 'the workspace has one root Cargo.lock' \
    "$(test -f "$root/Cargo.lock" && printf '%s' present)" present

nested_locks="$(find "$root/src" -mindepth 2 -maxdepth 2 -type f -name Cargo.lock -print | sort)"
t_assert_eq 'workspace crates do not carry stale nested Cargo.lock files' \
    "$nested_locks" ''

gitignore_target_rules="$(grep -E '(^|/)target/$' "$root/.gitignore" | sort -u || true)"
guideline_target_rules="$(sed -n '/^```gitignore$/,/^```$/p' \
    "$root/rust-development-guidelines.md" | grep -E '(^|/)target/$' | sort -u || true)"
t_assert_eq 'the guidelines and .gitignore document the same target rules' \
    "$guideline_target_rules" "$gitignore_target_rules"

nested_target="src/rjq/target/x86_64-unknown-linux-musl/release/rjq"
nested_ignored=''
if git -C "$root" check-ignore -q --no-index "$nested_target"; then
    nested_ignored=ignored
fi
t_assert_eq 'a crate-shaped target path is actually ignored' "$nested_ignored" ignored

t_end
