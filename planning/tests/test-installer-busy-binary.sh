#!/usr/bin/env bash
# MODE: DEV
# test-installer-busy-binary.sh — a reinstall replaces a shipped binary even
# while an older copy of it is running (B292).
#
# install_skill used to copy each file with a plain `cp`, which Linux refuses
# with ETXTBSY when the destination is a running executable's text segment.
# That aborted the whole run partway through the skill list, leaving every
# later skill uninstalled with no summary line saying so. The fix writes
# beside the target and renames over it, the same pattern the installer
# already uses for settings.json and opencode.json: a rename replaces the
# directory entry while the running process keeps its own inode, so the busy
# text segment is never touched.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/installer-busy.XXXXXX")"
trap 'rm -rf "$work"' EXIT

triple="x86_64-unknown-linux-musl"
case "$(uname -s):$(uname -m)" in
    Linux:x86_64|Linux:amd64) triple="x86_64-unknown-linux-musl" ;;
    Linux:aarch64|Linux:arm64) triple="aarch64-unknown-linux-musl" ;;
    *) t_skip 'this test only runs the busy-binary repro on Linux, where ETXTBSY applies' ;;
esac

shared_bin="$repo_root/bin/$triple/chat-server-rs"
if [ ! -x "$shared_bin" ]; then
    if command -v cargo >/dev/null 2>&1; then
        ( cd "$repo_root/src/chat-server-rs" && cargo build --release --target "$triple" >/dev/null 2>&1 ) \
            || t_fail 'cargo build chat-server-rs failed'
        mkdir -p "$repo_root/bin/$triple"
        cp "$repo_root/target/$triple/release/chat-server-rs" "$shared_bin"
        chmod +x "$shared_bin"
    else
        t_skip 'chat-server-rs is not built and no cargo is on PATH'
    fi
fi

target="$work/root"
( cd "$repo_root" && ./install.sh --skill chat --target "$target" --yes ) >/dev/null 2>&1 || true
installed_bin="$target/chat/bin/$triple/chat-server-rs"
[ -x "$installed_bin" ] || t_fail 'the install did not place chat-server-rs'

# Start the installed binary and hold it running.
chat_home="$work/chat-home"
mkdir -p "$chat_home"
AI_CHAT_HOME="$chat_home" CHAT_ANNOUNCE=0 "$installed_bin" 0 >/dev/null 2>&1 &
server_pid=$!
trap 'kill "$server_pid" 2>/dev/null || true; rm -rf "$work"' EXIT
sleep 1
kill -0 "$server_pid" 2>/dev/null || t_fail 'the installed server did not start'

# Force a reinstall to touch every file: an edited .version marker is enough
# to make the skill look changed, so the binary is recopied even though its
# own bytes have not (the fault path did not depend on content differing).
older_version="$work/older-version"
printf 'not-a-real-version\n' > "$older_version"
cp "$older_version" "$target/chat/.version"

out="$( ( cd "$repo_root" && ./install.sh --skill chat --target "$target" --yes ) 2>&1 )"
rc=$?
[ "$rc" -eq 0 ] || t_fail "reinstall over a running binary exited $rc: $out"
t_assert_contains 'the reinstall reports success, not a busy-file error' 'Installed:' "$out"
case "$out" in
    *'Text file busy'*|*'cannot create regular file'*)
        t_fail "the reinstall's own output still shows the busy-file error: $out" ;;
esac

kill -0 "$server_pid" 2>/dev/null || t_fail 'the running server was killed by the reinstall'

t_end
