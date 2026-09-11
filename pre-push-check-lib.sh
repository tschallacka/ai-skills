#!/usr/bin/env bash
# MODE: DEV
# pre-push-check-lib.sh — gates 3, 3b and 4 (CODE-STYLE §3, 400-line script
# cap).
#
# Sourced by pre-push-check.sh only, after its ok/bad/note/changed helpers
# and base/base_label/repo_root are set. Each function reads those from the
# caller rather than taking them as arguments.

set -euo pipefail
export LC_ALL=C

gate_shellcheck() {
    # ---- 3. shellcheck on the scripts that differ from master ------------------
    # CI lints every live script in one invocation, naming the generated libraries
    # alongside them because a `source=` directive resolves only against files on
    # the linter's own command line — omit them and every variable a sourcing
    # script reads from them reports unassigned (SC2154).
    #
    # That invocation costs ~33s of a ~47s run and is paid in full whether the
    # change touches one script or none. Here the change set is what matters, so
    # only the scripts that differ from master are linted, with -x: shellcheck
    # then follows `source=` from disk instead of requiring the target on the
    # command line, which is what makes a per-file lint equivalent to the whole-set
    # one. Measured over all 317 live scripts, linted one at a time: 2 disagree
    # with the whole-set result without -x (plan-context-lib.sh,
    # test-portable-helpers.sh — both unresolved-source false positives), 0 with
    # it, and -x introduces no findings of its own. On this branch the gate drops
    # from 32,676ms to 454ms.
    #
    # The libraries are still built first: -x resolves them from disk, so they have
    # to exist. CI remains the authority on the full set — a change in one script
    # can in principle provoke a finding in an unchanged script that sources it,
    # and only the whole-set lint sees that.
    if [ -x planning/scripts/build-plan-libs.sh ]; then
        planning/scripts/build-plan-libs.sh >/dev/null 2>&1 || true
    fi
    # Only scripts that still EXIST: a deletion is part of the change set, and
    # feeding a deleted path to shellcheck fails with "openBinaryFile: does not
    # exist" -- so removing a superseded script used to fail this gate, which is
    # precisely the shape that teaches people to bypass a gate rather than use it.
    changed_sh=""
    for _sh in $(changed -E '\.sh$'); do
        [ -f "$_sh" ] && changed_sh="${changed_sh}${changed_sh:+
    }$_sh"
    done
    unset _sh
    if [ -z "$changed_sh" ]; then
        note "no shell scripts differ from ${base_label:-the base}; shellcheck skipped (CI lints all)"
    elif command -v shellcheck >/dev/null 2>&1; then
        # shellcheck disable=SC2086
        if shellcheck -x -s bash --severity=warning $changed_sh >/dev/null 2>&1; then
            ok "shellcheck -x --severity=warning ($(printf '%s\n' "$changed_sh" | wc -l | tr -d ' ') changed vs ${base_label:-base})"
        else
            bad "shellcheck findings at warning severity (CI gates on these)"
            # shellcheck disable=SC2086
            shellcheck -x -s bash --severity=warning $changed_sh 2>&1 | sed -n '1,40p' >&2
        fi
    else
        note "shellcheck not installed locally; CI still gates on it"
    fi
    
}

gate_static_scans() {
    # ---- 3b. the two static shell gates CI fails on, over the same changed set --
    # Both are pure static checks over shell source and both are CI-fatal, so the
    # cheap half of each belongs in the default gate rather than only in the suite.
    #
    # Scoped to $changed_sh, the list section 3 already built, so the cost is
    # proportional to the change.
    #
    # What each scoped form does and does NOT prove:
    #   - the cap check reports what THIS change is responsible for: a function
    #     newly over the 40-line cap, or an already over-cap one that grew. It is
    #     diffed against $base for exactly that reason -- flagging every over-cap
    #     function in a touched file would refuse any edit to a file that already
    #     contains one, which is how a gate teaches people to bypass it. It says
    #     nothing about the tree-wide COUNT, which is a ratchet (may shrink, never
    #     grow) and so a global property no per-file run can evaluate; CI keeps that.
    #   - the portability scan applies the real rules and allowlists to the changed
    #     files only, so it cannot see a construct introduced in a file the change
    #     did not name. CI remains the authority on the whole tree.
    if [ -z "$changed_sh" ]; then
        note "no shell scripts differ from ${base_label:-the base}; cap and portability scans skipped"
    else
        cap_test="$repo_root/planning/tests/test-function-length-ratchet.sh"
        if [ -x "$cap_test" ]; then
            # shellcheck disable=SC2086
            if cap_out="$("$cap_test" --files --base "$base" $changed_sh 2>&1)"; then
                ok "no function newly over the 40-line cap"
            else
                bad "this change puts a function over CODE-STYLE.md's 40-line cap"
                printf '%s\n' "$cap_out" | sed -n '1,20p' >&2
            fi
        else
            note "no $cap_test to check the function cap with"
        fi
    
        port_test="$repo_root/planning/tests/test-portability-contract.sh"
        if [ -x "$port_test" ]; then
            # shellcheck disable=SC2086
            if port_out="$("$port_test" --files $changed_sh 2>&1)"; then
                ok "no banned portability construct in the changed scripts"
            else
                bad "a changed script uses a construct PORTABILITY.md bans"
                printf '%s\n' "$port_out" | sed -n '1,20p' >&2
            fi
        else
            note "no $port_test to check portability constructs with"
        fi
    fi
    
}

gate_rust_crates() {
    # ---- 4. rust crates under src/ touched by the change -----------------------
    crates="$(for f in $(changed -E '^src/[^/]+/'); do
        crate="${f#src/}"; crate="${crate%%/*}"
        [ -n "$crate" ] && printf '%s\n' "$crate"
    done | LC_ALL=C sort -u)"
    if [ -n "$crates" ]; then
        if ! command -v cargo >/dev/null 2>&1; then
            note "src/ changed but cargo is not on PATH (nix develop); CI still runs fmt and test"
        else
            while IFS= read -r crate; do
                m="src/$crate/Cargo.toml"
                [ -f "$m" ] || continue
                if cargo fmt --check --manifest-path "$m" >/dev/null 2>&1; then
                    ok "cargo fmt --check: $crate"
                else
                    bad "cargo fmt --check: $crate (CI runs fmt before the build)"
                fi
                if cargo test --manifest-path "$m" >/dev/null 2>&1; then
                    ok "cargo test: $crate"
                else
                    bad "cargo test: $crate"
                fi
            done <<EOF
$crates
EOF
            # Workspace-wide, not per-crate: CI's own reasoning (native.yml's
            # comment on the workspace gate) is that a per-crate pass misses a
            # library change breaking a consumer selection did not name, and that
            # was proven true here, not hypothetically -- three CI legs failed on
            # exactly this (client.rs's `session_token_path.is_some()` then
            # `.unwrap()`, `-D warnings` turning `unnecessary_unwrap` fatal) while
            # this gate, running fmt and test only, stayed green. `-D warnings`
            # matches CI's own flags so a local pass means the same thing CI's
            # does, not a weaker guarantee with the same wording.
            clippy_log="$(mktemp "${TMPDIR:-/tmp}/pre-push-clippy.XXXXXX")"
            if cargo clippy --workspace --all-targets -- -D warnings >"$clippy_log" 2>&1; then
                ok "cargo clippy --workspace -D warnings"
            else
                bad "cargo clippy --workspace -D warnings (CI gates on this too)"
                sed -n '1,40p' "$clippy_log" >&2
            fi
            rm -f "$clippy_log"
        fi
    else
        note "no crates under src/ changed; rust gates skipped"
    fi
}
