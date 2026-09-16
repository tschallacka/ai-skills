#!/usr/bin/env bash
# MODE: DEV
# test-exec-compiled-binary-preference — plan_exec_compiled_binary_if_present
# execs a compiled binary when one is present, falls through to bash
# unchanged when one is not, exports PLANNING_SKILL_ROOT correctly, forwards
# argv exactly, and leaves plan-crypt-lib.sh re-sourceable on the fall-through
# path.
#
# The exec-path assertion that matters most: a regression that invokes the
# binary via a plain call or command substitution instead of `exec` (still
# producing correct stub output, but then falling through to also run the
# bash implementation) is the exact double-execution bug this mechanism
# exists to prevent, and it is caught here by asserting a marker placed
# immediately after the function call never appears — proving the calling
# subshell was replaced, not merely that the stub ran.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
scripts_dir="$repo_root/planning/scripts"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/exec-compiled-binary-preference.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# shellcheck source=planning/scripts/plan-core-lib.sh
source "$scripts_dir/plan-core-lib.sh"

# ---- case (a): fall-through when no matching binary exists anywhere -------
fallthrough_out="$work/fallthrough.out"
rc=0
(
    unset AI_SKILLS_BIN_ROOT
    plan_exec_compiled_binary_if_present 'this-binary-does-not-exist-anywhere' "$scripts_dir" one two three
    printf 'FALLTHROUGH_MARKER\n'
) >"$fallthrough_out" 2>&1 || rc=$?
t_assert_eq 'fall-through returns 0' "$rc" 0
t_assert_contains 'fall-through reaches the line after the call' 'FALLTHROUGH_MARKER' "$(cat "$fallthrough_out")"

# ---- case (b)/(c): exec when a matching binary exists, argv/env forwarded,
#      and the calling subshell never returns -----------------------------
bin_dir="$work/bin"
mkdir -p "$bin_dir"
stub="$bin_dir/stub-binary"
stub_out="$work/stub.out"
cat >"$stub" <<STUB
#!/usr/bin/env bash
printf 'argv:%s\n' "\$*" > "$stub_out"
printf 'PLANNING_SKILL_ROOT:%s\n' "\${PLANNING_SKILL_ROOT:-}" >> "$stub_out"
STUB
chmod +x "$stub"

exec_out="$work/exec.out"
(
    AI_SKILLS_BIN_ROOT="$bin_dir"
    export AI_SKILLS_BIN_ROOT
    plan_exec_compiled_binary_if_present 'stub-binary' "$scripts_dir" one two three
    printf 'EXEC_MARKER\n'
) >"$exec_out" 2>&1 || true

t_assert_contains 'stub received exactly the forwarded arguments' 'argv:one two three' "$(cat "$stub_out")"
# B344: this must be the actual repo root (already computed above,
# independently of plan_exec_compiled_binary_if_present's own formula) --
# NOT re-derived from scripts_dir with the same arithmetic the implementation
# uses, which would make this assertion pass even if that arithmetic were
# wrong, exactly as it silently did before B344 was found and fixed.
t_assert_contains 'PLANNING_SKILL_ROOT is exported with the actual repo root' "PLANNING_SKILL_ROOT:$repo_root" "$(cat "$stub_out")"
case "$(cat "$exec_out")" in
    *EXEC_MARKER*) t_fail 'the post-call marker appeared: the subshell was not replaced by exec' ;;
    *) ;;
esac

# ---- case (d): the fall-through path leaves plan-crypt-lib.sh re-sourceable
(
    unset AI_SKILLS_BIN_ROOT
    plan_exec_compiled_binary_if_present 'this-binary-does-not-exist-anywhere' "$scripts_dir"
    if [ -n "${PLAN_CRYPT_LIB_LOADED:-}" ]; then
        printf 'GUARD_STILL_SET\n'
        exit 1
    fi
    # shellcheck source=planning/scripts/plan-crypt-lib.sh
    source "$scripts_dir/plan-crypt-lib.sh"
    if declare -F plan_bin_dir >/dev/null; then
        printf 'REDEFINED_OK\n'
    else
        printf 'NOT_REDEFINED\n'
    fi
) >"$work/guard.out" 2>&1 || true
t_assert_contains 'PLAN_CRYPT_LIB_LOADED is unset after fall-through, so a later real source redefines plan_bin_dir' 'REDEFINED_OK' "$(cat "$work/guard.out")"

# ---- case (e): B344 -- the exported PLANNING_SKILL_ROOT actually resolves
#      for a real compiled binary relocated outside any planning/scripts-
#      containing tree, with a cwd also outside one, which is the one
#      scenario the exported value exists for and the one this function's
#      own regression coverage above cannot exercise (a stub binary does not
#      call skill_root() at all). Uses verify-skill-load's own real crate.
vsl_manifest="$repo_root/src/verify-skill-load/Cargo.toml"
if command -v cargo >/dev/null 2>&1 && [ -f "$vsl_manifest" ]; then
    if cargo build --release --manifest-path "$vsl_manifest" >/dev/null 2>&1; then
        relocated_dir="$work/relocated-vsl"
        mkdir -p "$relocated_dir"
        cp "$repo_root/target/release/verify-skill-load" "$relocated_dir/verify-skill-load"
        chmod +x "$relocated_dir/verify-skill-load"
        relocated_out="$work/relocated.out"
        relocated_rc=0
        ( cd "$relocated_dir" && PLANNING_SKILL_ROOT="$repo_root" \
            ./verify-skill-load --part part-1 --token deadbeef ) \
            >"$relocated_out" 2>&1 || relocated_rc=$?
        if [ "$relocated_rc" = 69 ]; then
            t_fail 'a real binary relocated outside the repo still exits 69 despite PLANNING_SKILL_ROOT being exported (B344 regression)'
        fi
    else
        t_skip 'could not build verify-skill-load to exercise the real-binary PLANNING_SKILL_ROOT case'
    fi
else
    t_skip 'cargo unavailable; skipping the real-binary PLANNING_SKILL_ROOT case'
fi

# ---- case (f): AR-39 -- the function must not crash for a caller_script_dir
#      outside planning/scripts/ (e.g. ci-failures/scripts), where
#      plan-crypt-lib.sh does not exist. Found by adversarial review during
#      goal 12 (ci-failures.sh, the first script in this plan wired from
#      outside planning/scripts/): sourcing plan-crypt-lib.sh relative to
#      caller_script_dir crashed unconditionally, before the exec-or-fall-
#      through branch was ever reached, regardless of AI_SKILLS_BIN_ROOT.
foreign_dir="$work/foreign-skill/scripts"
mkdir -p "$foreign_dir"
foreign_out="$work/foreign.out"
foreign_rc=0
(
    unset AI_SKILLS_BIN_ROOT
    plan_exec_compiled_binary_if_present 'this-binary-does-not-exist-anywhere' "$foreign_dir"
    printf 'FOREIGN_FALLTHROUGH_MARKER\n'
) >"$foreign_out" 2>&1 || foreign_rc=$?
t_assert_eq 'a caller_script_dir outside planning/scripts/ still falls through cleanly (rc 0)' "$foreign_rc" 0
t_assert_contains 'a caller_script_dir outside planning/scripts/ reaches the line after the call (no crash sourcing plan-crypt-lib.sh)' 'FOREIGN_FALLTHROUGH_MARKER' "$(cat "$foreign_out")"

# ---- case (g): a caller AT the repo root itself resolves PLANNING_SKILL_ROOT
#      to the repo root, not two levels above it. Found by adversarial review
#      during goal 14 (pre-push-check.sh, the first script this plan wires
#      from the repo root itself rather than a two-levels-deep skill
#      directory): the old caller_script_dir/../.. arithmetic assumed every
#      caller lives at <root>/<skill-dir>/scripts and landed PLANNING_SKILL_ROOT
#      two levels ABOVE the actual repo root for a repo-root caller. Fixed by
#      walking up from caller_script_dir for the nearest ancestor containing
#      planning/scripts, which resolves correctly at any depth.
root_caller_out="$work/root-caller.out"
(
    AI_SKILLS_BIN_ROOT="$bin_dir"
    export AI_SKILLS_BIN_ROOT
    plan_exec_compiled_binary_if_present 'stub-binary' "$repo_root" one two three
) >"$root_caller_out" 2>&1 || true
t_assert_contains 'a caller_script_dir AT the repo root resolves PLANNING_SKILL_ROOT to the repo root itself' "PLANNING_SKILL_ROOT:$repo_root" "$(cat "$stub_out")"

t_end
