#!/usr/bin/env bash
# MODE: DEV
# test-release-package.sh — the tarball holds exactly what should be packed.
#
# The release asset is the article an end user receives. Until this file existed
# it was checked by hand: a diff of --list against npm pack, and a file count.
# Neither survives a change nobody re-runs it after.
#
# The expected set is derived here from the markers and the installer's prod arm,
# independently of build-release.sh --list. That is deliberate duplication: if the
# test asked the builder what it built, it would agree with itself no matter what
# the rule said. The two derivations are compared, so a disagreement fails.
#
# Four properties, in the order they matter:
#
#   1. exactly the expected paths -- nothing missing, nothing extra
#   2. every file byte-identical to the repository copy
#   3. no file marked MODE: DEV anywhere inside it
#   4. two builds of one tree produce the same bytes
#
# and then the one that subsumes them: the extracted tarball installs.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/.." && pwd)"
builder="$repo_root/installer/build-release.sh"
# shellcheck source=planning/tests/lib-test.sh
source "$repo_root/planning/tests/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/release-package.XXXXXX")"
trap 'rm -rf "$work"' EXIT

version="$(sed -n 's/^[[:space:]]*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
    "$repo_root/package.json" | head -1)"
numeric='(0|[1-9][0-9]*)'
prerelease_identifier="(0|[1-9][0-9]*|[A-Za-z-][0-9A-Za-z-]*)"
build_identifier='([0-9A-Za-z-]+)'
version_pattern="^${numeric}\.${numeric}\.${numeric}(-${prerelease_identifier}(\.${prerelease_identifier})*)?(\\+${build_identifier}(\.${build_identifier})*)?$"
t_assert_eq 'package.json states a version' \
    "$(printf '%s' "$version" | grep -Ec "$version_pattern")" '1'

for valid in 2.0.0-alpha 2.0.0-alpha.1 2.0.0-alpha.1+build.7 2.0.0+build.7; do
    t_assert_eq "valid SemVer is accepted: $valid" \
        "$(printf '%s' "$valid" | grep -Ec "$version_pattern")" '1'
done
for invalid in 2.0 x.y.z '' 02.0.0 2.0.0- 2.0.0-01 2.0.0-alpha..1; do
    t_assert_eq "malformed version is rejected: ${invalid:-empty}" \
        "$(printf '%s' "$invalid" | grep -Ec "$version_pattern" || true)" '0'
done

# ── the expected set, derived from the rule rather than from the builder ─────
declares_prod() { # <path>
    # A fixture's bytes are test input, not a declaration: a captured render
    # carries whatever marker its producer wrote. The same exemption is in
    # installer/build-release.sh and tests/test-mode-markers.sh.
    case "$1" in */tests/fixtures/*) return 1 ;; esac
    case "$(sed -n '1,25p' "$repo_root/$1" 2>/dev/null)" in
        *'# MODE: PROD'*|*'<!-- MODE: PROD -->'*) return 0 ;;
    esac
    return 1
}
# Unmarked files -- fixtures, formats with no comment syntax -- ship when the
# skill that owns them does, which is what the installer's prod arm says.
# shellcheck disable=SC1090
source "$repo_root/installer/src/05-config.sh"
# shellcheck disable=SC1090
source "$repo_root/installer/src/50-manifest.sh"
SOURCE_ROOT="$repo_root"
SOURCE_VERSION='test'
REPO_REF='test'

# B317: skill_files() now lists a skill's own bin/ artifacts only when they
# already exist on disk (chat, todo and bug-report included, matching
# ai-text-editor and interactive-shell). The builder cross/natively builds
# those same binaries itself and writes them into this same checkout
# ($repo_root/chat/bin/<triple>/…, not a scratch dir), so it has to run BEFORE
# "expected" is derived below -- otherwise a checkout where nothing had
# pre-built them sees skill_files() list none of them (correctly, for install.sh's
# own runtime), while the tarball the builder produces a few lines later has
# them anyway, and the two "expected" vs "actual" sets disagree over files
# that were always going to ship.
"$builder" --out "$work/dist" >/dev/null
tarball="$work/dist/ai-skills-$version.tar.gz"
[ -f "$tarball" ] || t_fail "the builder wrote no $tarball"

{
    printf 'README.md\nLICENSE\npackage.json\n'
    # tui-hint-plugin/editor-gate-plugin: neither is a skill (no skill_files()
    # entry) and most of their files have no comment syntax a MODE marker
    # could sit in (.json, .js), so this list is a third, deliberate copy of
    # the same file set installer/build-release.sh's own
    # tui_hint_plugin_files/editor_gate_plugin_files hardcode -- same
    # reasoning as the five names on the line above.
    printf 'tui-hint-plugin/.claude-plugin/plugin.json\n'
    printf 'tui-hint-plugin/hooks/hooks.json\n'
    printf 'tui-hint-plugin/hooks/lib.sh\n'
    printf 'tui-hint-plugin/hooks/pre-tool-use.sh\n'
    printf 'tui-hint-plugin/opencode/tui-hint-plugin.js\n'
    printf 'editor-gate-plugin/.claude-plugin/plugin.json\n'
    printf 'editor-gate-plugin/hooks/hooks.json\n'
    printf 'editor-gate-plugin/hooks/lib.sh\n'
    printf 'editor-gate-plugin/hooks/editor-token\n'
    printf 'editor-gate-plugin/hooks/pre-tool-use-bash.sh\n'
    printf 'editor-gate-plugin/hooks/pre-tool-use-edit-write.sh\n'
    while IFS= read -r path; do
        [ -n "$path" ] || continue
        declares_prod "$path" && printf '%s\n' "$path"
    done < <(cd "$repo_root" && git ls-files \
        planning project-specifics resource-limited-testing brainstorm \
        post-implementation-review todo bug-report)
    for skill in "${SKILL_NAMES[@]}"; do
        while IFS= read -r path; do
            [ -n "$path" ] || continue
            if [ ! -f "$repo_root/$skill/$path" ]; then
                case "$skill/$path" in
                    planning/bin/*/plan-overview|planning/bin/*/plan-overview.exe) continue ;;
                esac
            fi
            printf '%s/%s\n' "$skill" "$(platform_relative_path "$skill" "$path")"
        done <<EOF
$(skill_files "$skill")
EOF
    done
} | sort -u > "$work/expected"

expected_count="$(grep -c . "$work/expected")"
# A positive control: an empty expectation would make every comparison below
# trivially true.
t_assert_eq 'the expected set is not empty' \
    "$([ "$expected_count" -gt 50 ] && printf 'over 50')" 'over 50'

# ── property 1: the tarball holds exactly those paths ───────────────────────
# The builder already ran above, before "expected" was derived; $tarball is
# already written.
tar -tzf "$tarball" | sed "s|^ai-skills-$version/||" | grep -v '/$' | sort > "$work/actual"
t_assert_eq 'nothing expected is missing from the tarball' \
    "$(comm -23 "$work/expected" "$work/actual" | tr '\n' ' ')" ''
t_assert_eq 'nothing unexpected is in the tarball' \
    "$(comm -13 "$work/expected" "$work/actual" | tr '\n' ' ')" ''

# And the builder's own --list must agree with the rule, or the two have drifted.
"$builder" --list | sort -u > "$work/listed"
t_assert_eq "the builder's --list matches the derived set" \
    "$(comm -3 "$work/expected" "$work/listed" | tr '\n' ' ')" ''

# ── property 2: every file byte-identical to its source of truth ────────────
# For the generated compiled libraries that source of truth is a fresh build,
# not a repository copy: they are never tracked (MAINTAINER.md section 2.16),
# so a clean tree has none, and a stale present one must not silently pass
# here - the lib test owns staleness, this test owns what the tarball carries.
fresh_scripts="$work/fresh-scripts"
mkdir -p "$fresh_scripts"
cp -R "$repo_root/planning/scripts/." "$fresh_scripts/"
( cd "$fresh_scripts" && ./build-plan-libs.sh ) >/dev/null 2>&1 \
    || t_fail 'building the compiled libraries for comparison failed'
mkdir -p "$work/extract"
tar -xzf "$tarball" -C "$work/extract"
extracted="$work/extract/ai-skills-$version"
differing='' compared=0
while IFS= read -r path; do
    [ -n "$path" ] || continue
    compared=$((compared + 1))
    case "$path" in
        planning/scripts/plan-core-lib.sh|planning/scripts/plan-crypt-lib.sh| \
planning/scripts/plan-document-lib.sh|planning/scripts/plan-progress-lib.sh| \
planning/scripts/plan-table-lib.sh)
            reference="$fresh_scripts/$(basename "$path")" ;;
        *) reference="$repo_root/$path" ;;
    esac
    cmp -s "$reference" "$extracted/$path" || differing="$differing $path"
done < "$work/expected"
t_assert_eq 'every packed file is byte-identical to its source of truth' "${differing# }" ''
t_assert_eq 'and every expected file was actually compared' "$compared" "$expected_count"

# ── property 3: nothing marked MODE: DEV is inside it ──────────────────────
# Read the header only: a heredoc lower down mentions the marker strings.
# grep -I skips a binary compiled artifact (rjq, ai-text-editor, ...) rather
# than reading it, so its bytes are never searched for the marker text at
# all. -c, not -q: PORTABILITY(pipefail-grep-q) -- grep -q exits on the
# first match and closes the pipe, so under set -o pipefail the writer
# (head) dies of SIGPIPE and the pipeline's own status is 141, not grep's;
# -c reads to completion, so no writer ever sees a closed pipe, and its
# output is a small decimal count rather than the matched (possibly
# binary) content, which is what keeps a capture safe here.
leaked=''
while IFS= read -r path; do
    [ -n "$path" ] || continue
    hits="$(head -25 "$extracted/$path" 2>/dev/null \
        | grep -Ic -e '# MODE: DEV' -e '<!-- MODE: DEV -->' || true)"
    [ "${hits:-0}" -gt 0 ] && leaked="$leaked $path"
done < "$work/expected"
t_assert_eq 'no maintainer file reached the release' "${leaked# }" ''
# The categories that motivated the split, named so a regression says which.
t_assert_eq 'no test script is in the release' \
    "$(grep -c '^planning/tests/' "$work/actual" || true)" '0'
t_assert_eq 'no per-function library source is in the release' \
    "$(grep -c '^planning/scripts/lib/' "$work/actual" || true)" '0'
t_assert_eq 'the compiled libraries are' \
    "$(grep -c '^planning/scripts/plan-core-lib\.sh$' "$work/actual")" '1'
t_assert_eq 'and the compiler is not' \
    "$(grep -c 'build-plan-libs\.sh' "$work/actual" || true)" '0'
t_assert_eq 'the Rust planning command is executable in the release' \
    "$([ -x "$extracted/planning/scripts/plan-content" ] && printf present)" 'present'
t_assert_eq 'the Rust artifact verification contract is in the release' \
    "$([ -f "$extracted/planning/RUST-ARTIFACT-MANIFEST.md" ] && printf present)" 'present'

# ── property 4: two builds of one tree produce the same bytes ───────────────
"$builder" --out "$work/dist2" >/dev/null
t_assert_eq 'two builds of one tree are byte-identical' \
    "$(cmp -s "$tarball" "$work/dist2/ai-skills-$version.tar.gz" && printf same)" 'same'

# ── the property that subsumes the rest: the package installs ───────────────
# A tarball whose contents are correct but which cannot install is still broken.
# This tarball itself carries no installer binary at all -- build-release.sh's
# own output is the universal skill payload; installer/build-installer-release.sh
# is what adds a target's compiled installer on top of it, per release asset.
# So the installer under test is built fresh from source here, and pointed at
# the extracted tarball via --source, same shape the old install.sh smoke test
# had (an installer, and a source tree to install skills from), just with the
# two no longer bundled together in one artifact.
installer_bin="$work/installer"
if command -v cargo >/dev/null 2>&1; then
    ( cd "$repo_root" && cargo build --release -p installer ) >/dev/null 2>&1 \
        && cp "$repo_root/target/release/installer" "$installer_bin" 2>/dev/null
fi
# T72: every skill's compiled binaries share one location keyed off $HOME,
# not this install's own --target -- isolated here so the real run never
# touches this machine's actual ~/.config/tsch-ai-skills/bin.
scratch_home="$work/home"
mkdir -p "$scratch_home"
if [ -x "$installer_bin" ]; then
    rc=0
    HOME="$scratch_home" XDG_CONFIG_HOME="" "$installer_bin" install --skill todo \
        --source "$extracted" --target "$work/installed" --yes >/dev/null 2>&1 || rc=$?
    t_assert_eq 'the extracted package installs a skill' "$rc" '0'
else
    printf 'SKIP: no installer binary built and cargo unavailable to build one\n' >&2
fi
# bin/ no longer appears under an installed skill at all -- todo's binary
# lands in the shared location above instead. binaries.tsv itself still
# ships, since it is the packaging-side declaration, not the installed
# artifact.
t_assert_eq 'and the installed skill is complete' \
    "$(ls "$work/installed/todo" 2>/dev/null | sort | tr '\n' ' ')" \
     "SKILL.md binaries.tsv docs requires.tsv schema.1.4.2.json schema.$version.json "
t_assert_eq 'and its binary reached the shared bin' \
    "$([ -x "$scratch_home/.config/tsch-ai-skills/bin/todo" ] && printf present)" 'present'

t_end
