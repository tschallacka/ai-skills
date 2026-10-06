#!/usr/bin/env bash
# Copies the loading animation, mods/loading/loading.tsx, into the hooks/ folder of each
# mod that uses it, as hooks/loading.tsx. A plugin may import only its own files, so each
# consumer carries its own copy; this keeps the copies identical.
#
#   mods/loading/sync.sh           copy into every consumer
#   mods/loading/sync.sh --check   fail if any consumer's copy differs from the source
#
# To add a consumer, add its mod folder name to CONSUMERS below.
set -eu

CONSUMERS="ci-board"

here="$(cd "$(dirname "$0")" && pwd)"
mods_dir="$(dirname "$here")"
source_file="$here/loading.tsx"

check=0
if [ "${1:-}" = "--check" ]; then
  check=1
fi

status=0
for mod in $CONSUMERS; do
  target="$mods_dir/$mod/hooks/loading.tsx"
  if [ "$check" -eq 1 ]; then
    if [ -f "$target" ] && cmp -s "$source_file" "$target"; then
      echo "ok      $mod"
    else
      echo "differs $mod (run mods/loading/sync.sh)"
      status=1
    fi
  else
    cp "$source_file" "$target"
    echo "synced  $mod"
  fi
done
exit "$status"
