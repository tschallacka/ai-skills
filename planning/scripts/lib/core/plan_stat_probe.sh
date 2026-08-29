#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
# File mode (octal, e.g. 644) and owner uid. GNU stat and BSD stat share no
# flag, so probe once and define the function accordingly rather than forking a
# probe on every call. GNU %a and BSD %Lp both print the low 12 mode bits.
if stat -c '%a' /dev/null >/dev/null 2>&1; then
    plan_stat_mode() { stat -c '%a' -- "$1"; }
    plan_stat_uid() { stat -c '%u' -- "$1"; }
else
    plan_stat_mode() { stat -f '%Lp' "$1"; }
    plan_stat_uid() { stat -f '%u' "$1"; }
fi
