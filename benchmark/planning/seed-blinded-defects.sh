#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 4 ]; then
    printf 'Usage: %s <source-workspace> <defective-workspace> <oracle-private-dir> <defect-spec.json>\n' "$(basename "$0")" >&2
    exit 64
fi

# shellcheck source=benchmark/planning/lib-portable.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-portable.sh"

source_root="$1"
defective_root="$2"
private_root="$3"
spec_file="$4"

[ -d "$source_root" ] || { printf 'Source workspace not found: %s\n' "$source_root" >&2; exit 66; }
[ ! -e "$defective_root" ] || { printf 'Defective workspace already exists: %s\n' "$defective_root" >&2; exit 73; }
[ ! -e "$private_root" ] || { printf 'Oracle-private directory already exists: %s\n' "$private_root" >&2; exit 73; }
[ -f "$spec_file" ] || { printf 'Defect specification not found: %s\n' "$spec_file" >&2; exit 66; }

mkdir -p "$defective_root" "$private_root"
chmod 700 "$private_root"
temporary_map="$(mktemp "$private_root/defect-map.XXXXXX.json")"
oracle_key="$private_root/oracle-key"
trap 'rm -f "$temporary_map"; rmdir "$private_root" 2>/dev/null || true' EXIT

tar -C "$source_root" -cf - . | tar -C "$defective_root" -xf -

python3 - "$defective_root" "$spec_file" "$temporary_map" <<'PY'
import hashlib
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1]).resolve()
spec = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
output = pathlib.Path(sys.argv[3])
defects = spec.get("defects")
if not isinstance(defects, list) or not defects:
    raise SystemExit("defect specification must contain a non-empty defects list")
seen = set()
manifest = []
for defect in defects:
    defect_id, relative, old, new = (defect.get(key) for key in ("id", "path", "old", "new"))
    location = defect.get("location", defect.get("anchor"))
    expected_signal = defect.get("expected_signal")
    required_correction = defect.get("required_correction")
    severity = defect.get("severity")
    if not all(isinstance(value, str) and value for value in
               (defect_id, relative, old, new, location, expected_signal,
                required_correction, severity)):
        raise SystemExit("each defect requires non-empty id, path, old, new, location, expected_signal, required_correction, and severity strings")
    if defect_id in seen:
        raise SystemExit(f"duplicate defect id: {defect_id}")
    seen.add(defect_id)
    target = (root / relative).resolve()
    if root not in target.parents:
        raise SystemExit(f"defect path escapes workspace: {relative}")
    if not target.is_file():
        raise SystemExit(f"defect target not found: {relative}")
    content = target.read_text(encoding="utf-8")
    occurrences = content.count(old)
    if occurrences < 1:
        raise SystemExit(f"defect text not found: {defect_id}")
    changed = content.replace(old, new, 1)
    target.write_text(changed, encoding="utf-8")
    manifest.append({"id": defect_id, "path": relative, "location": location,
                     "old": old, "new": new,
                     "expected_signal": expected_signal,
                     "required_correction": required_correction,
                     "severity": severity,
                     "occurrences_before": occurrences,
                     "original_sha256": hashlib.sha256(content.encode()).hexdigest()})
for entry in manifest:
    final_target = root / entry["path"]
    entry["defective_sha256"] = hashlib.sha256(final_target.read_bytes()).hexdigest()
output.write_text(json.dumps({"schema_version": "1.4.2-blinded-oracle", "defects": manifest}, sort_keys=True) + "\n", encoding="utf-8")
PY

openssl rand -base64 32 > "$oracle_key"
chmod 600 "$oracle_key"
openssl enc -aes-256-cbc -pbkdf2 -salt -in "$temporary_map" -out "$private_root/defect-map.enc" -pass file:"$oracle_key"
chmod 600 "$private_root/defect-map.enc"
mkdir -p "$private_root/target-snapshot"
tar -C "$defective_root" -cf - . | tar -C "$private_root/target-snapshot" -xf -
chmod -R go-rwx "$private_root/target-snapshot"
printf '{"schema_version":"1.4.2-blinded-oracle","defective_root":"%s","encrypted_map":"%s","map_sha256":"%s"}\n' \
    "$defective_root" "$private_root/defect-map.enc" "$(benchmark_hash_file "$private_root/defect-map.enc")" > "$private_root/seed-metadata.json"
chmod 600 "$private_root/seed-metadata.json"
rm -f "$temporary_map"
trap - EXIT
printf 'Blinded defect workspace created: %s\n' "$defective_root"
printf 'Encrypted oracle material stored privately: %s\n' "$private_root"
