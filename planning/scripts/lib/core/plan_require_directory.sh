#!/usr/bin/env bash
plan_require_directory() {
    [ -d "$1" ] || plan_die "Plan directory not found: $1" 66
}
