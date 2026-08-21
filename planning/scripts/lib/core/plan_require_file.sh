#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# Require an input file; refuse to clobber an existing artifact. Same exit-code
# vocabulary as plan_require_directory: 66 for "missing input", 73 for
# "already there, not overwriting".
plan_require_file() {
    [ -f "$1" ] || plan_die "File not found: $1" 66
}
