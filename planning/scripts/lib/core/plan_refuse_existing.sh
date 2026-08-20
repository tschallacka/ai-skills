#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
plan_refuse_existing() {
    [ ! -e "$1" ] || plan_die "Refusing to overwrite an existing artifact: $1" 73
}
