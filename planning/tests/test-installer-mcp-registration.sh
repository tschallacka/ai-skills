#!/usr/bin/env bash
# MODE: DEV
# test-installer-mcp-registration — an mcp-mode install reaches the agent, and a
# switch away takes the registration with it.
#
# Usage: test-installer-mcp-registration.sh
#
# Why this exists: `--integration mcp` decided which binary landed and nothing
# else, so the adapter sat on disk and no agent knew it existed (B284). The
# other half is that the toggle deletes the binary a registration names, which
# leaves an entry pointing at nothing (B285).
#
# The agent CLIs are stubbed. Their argv is what this asserts on, because the
# contract is "the installer asks the agent's own tool to do it" -- so the pin
# is the command line, not the config file each tool then writes. Two facts
# measured against the real binaries are pinned here so a change in either is
# a test failure rather than a silent regression:
#
#   claude   `mcp add` on an existing name exits 0 WITHOUT updating the
#            command, so an upgrade that moved the binary would keep the old
#            path unless the installer removes first.
#   opencode `mcp add NAME -- COMMAND` is undocumented and is the only
#            non-interactive way to register a local server; there is no
#            `mcp remove` at all.
set -euo pipefail
export LC_ALL=C
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
work="$(mktemp -d "${TMPDIR:-/tmp}/mcp-registration.XXXXXX")"
trap 'rm -rf "$work"' EXIT

t_begin

# The parts under test, plus the two they lean on: the manifest for
# integration_binary_mode and skill_files, and the permission part for
# opencode_configfile and backup_file.
# shellcheck disable=SC1090
source "$repo_dir/installer/src/05-config.sh"
# shellcheck disable=SC1090
source "$repo_dir/installer/src/50-manifest.sh"
# shellcheck disable=SC1090
source "$repo_dir/installer/src/70-permissions.sh"
# shellcheck disable=SC1090
source "$repo_dir/installer/src/72-mcp-registration.sh"
SOURCE_ROOT="$repo_dir"
SOURCE_VERSION='test'
REPO_REF='test'

# The generated tables live in install.sh, not in a part, so a test that
# sourced the parts alone would see every lookup empty and pass vacuously.
#
# Through a file rather than `source <(...)`: bash 3.2, which CODE-STYLE.md
# section 1 makes the floor, does not source a process substitution here, and
# the whole table then reads as empty -- which is exactly the vacuous pass this
# is guarding against.
awk '/^# BEGIN GENERATED INTEGRATION BLOCK/{p=1} p{print} /^# END GENERATED INTEGRATION BLOCK/{exit}' \
    "$repo_dir/install.sh" > "$work/integration-block.sh"
# shellcheck disable=SC1090
source "$work/integration-block.sh"

# ai-text-editor rather than chat: this branch is off master, where the editor
# is the skill that declares both modes. The step is skill-agnostic -- it reads
# the same generated table for any of them.
t_assert_eq 'the generated integration table is loaded' \
    "$(integration_binary_mode ai-text-editor ai-text-editor-mcp)" 'mcp'

# ---- stub agents ----------------------------------------------------------
# Each records its argv and nothing else. `codex` and `opencode` refuse a name
# they have not been given a command for, the way the real ones do.
stub_dir="$work/bin"
mkdir -p "$stub_dir"
for agent in claude codex opencode; do
    cat > "$stub_dir/$agent" <<STUB
#!/bin/sh
printf '%s\\n' "$agent \$*" >> "$work/calls"
exit 0
STUB
    chmod +x "$stub_dir/$agent"
done
PATH="$stub_dir:$PATH"
export PATH
: > "$work/calls"

# A skill directory shaped like an mcp-mode install: the adapter present, the
# CLI client absent.
skill_root="$work/target"
adapter_path="$skill_root/ai-text-editor/bin/$(uname -m)-probe/ai-text-editor-mcp"
mkdir -p "$(dirname "$adapter_path")"
printf '#!/bin/sh\nexit 0\n' > "$adapter_path"
chmod +x "$adapter_path"
# The skill-mode client beside it: present in both modes on disk here, and not
# the binary the step must pick.
printf '#!/bin/sh\nexit 0\n' > "$(dirname "$adapter_path")/ai-text-editor"
t_assert_eq 'the adapter is the binary the step finds' \
    "$(mcp_adapter_path ai-text-editor "$skill_root/ai-text-editor")" "$adapter_path"

# ---- 1. registration goes through each agent's own CLI --------------------
mcp_registration_for_root ai-text-editor "$skill_root" >/dev/null 2>&1 || true
calls="$(cat "$work/calls")"

# The agent kind is decided by the root, so drive each one explicitly rather
# than depending on which agents this machine has.
: > "$work/calls"
mcp_register_for_kind claude ai-text-editor "$adapter_path" >/dev/null 2>&1
mcp_register_for_kind codex ai-text-editor "$adapter_path" >/dev/null 2>&1
mcp_register_for_kind opencode ai-text-editor "$adapter_path" >/dev/null 2>&1
calls="$(cat "$work/calls")"

case "$calls" in
    *"claude mcp remove -s user ai-text-editor"*) : ;;
    *) t_fail "claude was not asked to remove before adding: [$calls]" ;;
esac
case "$calls" in
    *"claude mcp add -s user -t stdio ai-text-editor $adapter_path"*) : ;;
    *) t_fail "claude was not asked to add the adapter: [$calls]" ;;
esac
case "$calls" in
    *"codex mcp add ai-text-editor -- $adapter_path"*) : ;;
    *) t_fail "codex was not asked to add the adapter: [$calls]" ;;
esac
case "$calls" in
    *"opencode mcp add ai-text-editor -- $adapter_path"*) : ;;
    *) t_fail "opencode was not asked to add the adapter: [$calls]" ;;
esac

# The order is the fix for claude's silent no-op on an existing name: an add
# that ran first would leave a moved binary's old path in place.
remove_at="$(printf '%s\n' "$calls" | awk '/claude mcp remove/{print NR; exit}')"
add_at="$(printf '%s\n' "$calls" | awk '/claude mcp add/{print NR; exit}')"
if [ -z "$remove_at" ] || [ -z "$add_at" ]; then
    t_fail "claude was not asked to both remove and add: [$calls]"
elif [ "$remove_at" -ge "$add_at" ]; then
    t_fail "claude add ran before remove ($add_at before $remove_at)"
fi

# ---- 2. a skill with no adapter installed is unregistered, not registered --
: > "$work/calls"
rm -f "$adapter_path"
mcp_registration_for_root ai-text-editor "$skill_root" >/dev/null 2>&1 || true
case "$(cat "$work/calls")" in
    *"mcp add"*) t_fail "an absent adapter was still registered: [$(cat "$work/calls")]" ;;
    *) : ;;
esac

# ---- 3. removal only touches an entry pointing inside this install --------
# Ownership is read from the agent's own config; a name registered elsewhere is
# somebody else's and must survive the toggle.
export HOME="$work/home"
mkdir -p "$HOME"
printf '{"mcpServers":{"ai-text-editor":{"command":"%s"}}}\n' "$skill_root/ai-text-editor/bin/x/ai-text-editor-mcp" > "$HOME/.claude.json"
if command -v rjq >/dev/null 2>&1; then
    mcp_entry_is_ours claude ai-text-editor "$skill_root/ai-text-editor" \
        || t_fail 'an entry inside the skill directory was not recognised as ours'
    printf '{"mcpServers":{"ai-text-editor":{"command":"/opt/elsewhere/ai-text-editor-mcp"}}}\n' > "$HOME/.claude.json"
    if mcp_entry_is_ours claude ai-text-editor "$skill_root/ai-text-editor"; then
        t_fail 'an entry pointing outside the skill directory was claimed as ours'
    fi
else
    printf 'SKIP mcp registration: no rjq, so ownership was not asserted\n' >&2
fi

# ---- 4. a skill that declares no modes never reaches an agent -------------
: > "$work/calls"
mcp_registration_for_root todo "$skill_root" >/dev/null 2>&1 || true
t_assert_eq 'a skill with no integration.tsv calls no agent CLI' \
    "$(cat "$work/calls")" ''

t_end
