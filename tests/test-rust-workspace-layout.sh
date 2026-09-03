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

t_assert_eq '.gitignore has exactly one root workspace target rule' \
    "$(grep -Fxc '/target/' "$root/.gitignore" || true)" 1
t_assert_eq '.gitignore has no obsolete per-crate target rule' \
    "$(grep -Fxc 'src/*/target/' "$root/.gitignore" || true)" 0

t_assert_eq 'guidelines document the root workspace target rule' \
    "$(grep -Fxc '/target/' "$root/rust-development-guidelines.md" || true)" 1
t_assert_eq 'guidelines have no obsolete per-crate target rule' \
    "$(grep -Fxc 'src/*/target/' "$root/rust-development-guidelines.md" || true)" 0

t_end
