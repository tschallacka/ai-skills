#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: DEV
# ── Temp-file bookkeeping (CODE-STYLE §8) ────────────────────────────────────
# bash keeps exactly one EXIT handler: a script installing its own after
# sourcing this library replaces plan_cleanup, and `trap - EXIT` clears the slot.
plan_track_tmp() {
    plan_tmp_files=(${plan_tmp_files[@]+"${plan_tmp_files[@]}"} "$1")
}
