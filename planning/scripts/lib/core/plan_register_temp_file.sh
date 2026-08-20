#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
plan_register_temp_file() {
    PLAN_DIE_TEMP_FILES+=("$1")
}
