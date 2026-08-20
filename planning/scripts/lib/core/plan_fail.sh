#!/usr/bin/env bash
# Non-fatal accumulators for checkers that report every finding. plan_fail
# bumps plan_error_count; the caller exits 1 at the end when it is non-zero.
# Neither ever exits: a sourced function must leave that decision to its caller.
plan_fail() {
    plan_error_count=$((plan_error_count + 1))
    printf 'FAIL: %s\n' "$*" >&2
}
