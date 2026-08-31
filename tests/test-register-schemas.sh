#!/usr/bin/env bash
# MODE: DEV
# test-register-schemas.sh — the register skills ship a schema for their version.
#
# The registers carry no backwards compatibility: an agent meeting a file written
# by an older skill reads that version's schema.<version>.json to upgrade it, or
# rewrites the file. Two things have to hold for that to work at all. The
# installed version must have a schema shipped beside SKILL.md -- so a version
# bump with no new schema file is a failure here, not a discovery in the field --
# and each upgrade_from recipe must run, because SKILL.md tells the agent to run
# it rather than read it.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$repo_root/planning/tests/lib-test.sh"
t_begin

# rjq is the reader for every register this test validates. When it is missing,
# name the fix instead of dying with a bare command-not-found three lines in.
if ! command -v rjq >/dev/null 2>&1; then
    printf '%s\n' "rjq is required: run ./bootstrap.sh (builds it into the gitignored planning/bin path) or download it from the project releases page (queued as T70)." >&2
    exit 1
fi

work="$(mktemp -d "${TMPDIR:-/tmp}/register-schemas.XXXXXX")"
trap 'rm -rf "$work"' EXIT

package_version="$(rjq -r '.version' "$repo_root/package.json")"
t_assert_eq 'package.json states a version' \
    "$(printf '%s' "$package_version" | grep -Ec '^[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*(\-[0-9A-Za-z][0-9A-Za-z.-]*)?$')" '1'

# The first fenced json block in a SKILL.md is the worked example of the file the
# skill writes. Checking it against the schema keeps the two from drifting: the
# example is what an agent copies.
skill_example() { # <skill> → the first ```json block
    awk '/^```json$/ { inside = 1; next } inside && /^```$/ { exit } inside' "$repo_root/$1/SKILL.md"
}

for skill in todo bug-report; do
    schema="$repo_root/$skill/schema.$package_version.json"

    [ -f "$schema" ] || t_fail "$skill ships no schema.$package_version.json for the installed version"
    t_assert_eq "$skill: its schema is valid JSON" \
        "$(rjq -e 'type == "object"' "$schema" >/dev/null 2>&1 && printf yes)" 'yes'
    t_assert_eq "$skill: the schema names its own skill" "$(rjq -r '.schema' "$schema")" "$skill"
    t_assert_eq "$skill: the schema names the package version" \
        "$(rjq -r '.version' "$schema")" "$package_version"
    t_assert_eq "$skill: the schema says what to do with a version it cannot upgrade" \
        "$(rjq -r '.if_no_schema_for_a_version | length > 0' "$schema")" 'true'

    # The installer has to hand the schema over, or the agent that needs it never
    # sees one. skill_files() is the single list the manifest is built from.
    # The manifest derives the schema filename from package.json, so grepping the
    # generated installer for a literal proves nothing. Install the skill and look
    # at what arrived -- the only thing the agent reading the schema depends on.
    installed="$work/installed-$skill"
    "$BASH" "$repo_root/install.sh" --skill "$skill" --target "$installed" --yes >/dev/null 2>&1 \
        || t_fail "$skill: the installer refused to install it"
    [ -f "$installed/$skill/schema.$package_version.json" ] \
        || t_fail "$skill: installing it did not deliver schema.$package_version.json (got: $(ls "$installed/$skill" | tr '\n' ' '))"
    t_assert_eq "$skill: the delivered schema matches the source" \
        "$(cmp -s "$schema" "$installed/$skill/schema.$package_version.json" && printf same)" 'same'
    t_assert_eq "$skill: the install records the version the schema is named for" \
        "$(sed -n 's/^package_version=//p' "$installed/$skill/.version")" "$package_version"

    example="$work/$skill-example.json"
    skill_example "$skill" > "$example"
    # A positive control: an empty extraction would satisfy every check below.
    t_assert_eq "$skill: the SKILL.md example was extracted and parses" \
        "$(rjq -e 'type == "object"' "$example" >/dev/null 2>&1 && printf yes)" 'yes'

    item_key="$(rjq -r '.header.item_key' "$schema")"
    t_assert_eq "$skill: the example holds items under the documented key" \
        "$(rjq -r --arg k "$item_key" '(.[$k] | length) > 0' "$example")" 'true'
    t_assert_eq "$skill: the example carries every required header field" \
        "$(rjq -r --slurpfile s "$schema" \
            '. as $doc | [$s[0].header.required[] as $f | select(($doc | has($f)) | not) | $f] | join(", ")' \
            "$example")" ''
    # Every item must carry the required fields and stay inside the enums. The
    # message names the offending id, because "an item is wrong" is not findable.
    offenders="$(rjq -r --slurpfile s "$schema" --arg k "$item_key" '
        $s[0] as $schema
        | [ .[$k][] as $item
            | ( [$schema.item.required[] as $f | select(($item | has($f)) | not) | "\($item.id): missing \($f)"]
              + [$schema.item.fields | to_entries[]
                 | select(.value.enum) as $field
                 | select($item[$field.key] as $v | $v != null and ($field.value.enum | index($v) | not))
                 | "\($item.id): \($field.key)=\($item[$field.key]) is not in the enum"] )[] ]
        | join("; ")' "$example")"
    t_assert_eq "$skill: every example item conforms to the schema" "$offenders" ''
done

# ── the todo upgrade recipe runs, and produces a conforming file ─────────────
# It is the one predecessor that exists: the unversioned registers this repo
# kept before the schema header. The recipe is quoted in the schema for an agent
# to run verbatim, so it is run verbatim here.
schema="$repo_root/todo/schema.$package_version.json"
t_assert_eq 'todo: the schema knows how to upgrade an unversioned file' \
    "$(rjq -r '.upgrade_from | has("unversioned")' "$schema")" 'true'

cat > "$work/TODO.json" <<'JSON'
{
  "comment": "An unversioned queue.",
  "items": [
    {"id": "X1", "title": "Queued", "status": "open", "parent": null,
     "detail": "d", "evidence": null, "blocked_on": null, "refs": []},
    {"id": "X2", "title": "Not real after all", "status": "not-a-defect", "parent": "X1",
     "detail": "d", "evidence": "checked, nothing there", "blocked_on": null, "refs": []}
  ]
}
JSON
recipe="$(rjq -r '.upgrade_from.unversioned.steps[] | select(startswith("now=") or startswith("rjq "))' "$schema")"
t_assert_eq 'todo: the upgrade recipe has runnable steps' \
    "$(printf '%s\n' "$recipe" | grep -c '^rjq ')" '1'
( cd "$work" && "$BASH" -c "set -euo pipefail; $recipe" )

t_assert_eq 'todo: the upgraded file records the current version' \
    "$(rjq -r '.skill_version' "$work/TODO.json")" "$package_version"
t_assert_eq 'todo: the upgraded file names its schema' "$(rjq -r '.skill' "$work/TODO.json")" 'todo'
t_assert_eq 'todo: no task was dropped in the upgrade' "$(rjq -r '.tasks | length' "$work/TODO.json")" '2'
t_assert_eq 'todo: the retired status is mapped into the current enum' \
    "$(rjq -r '.tasks[] | select(.id == "X2") | .status' "$work/TODO.json")" 'dropped'
t_assert_eq 'todo: the old evidence field survives as note' \
    "$(rjq -r '.tasks[] | select(.id == "X2") | .note' "$work/TODO.json")" 'checked, nothing there'
t_assert_eq 'todo: the nesting survives' \
    "$(rjq -r '.tasks[] | select(.id == "X2") | .parent' "$work/TODO.json")" 'X1'
t_assert_eq 'todo: every upgraded task carries the required fields' \
    "$(rjq -r --slurpfile s "$schema" \
        '[.tasks[] as $t | $s[0].item.required[] as $f | select(($t | has($f)) | not)] | length' \
        "$work/TODO.json")" '0'
t_assert_eq 'todo: an upgraded task with no priority is given the default' \
    "$(rjq -r '.tasks[] | select(.id == "X1") | .priority' "$work/TODO.json")" 'normal'

t_end
