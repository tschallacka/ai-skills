#!/usr/bin/env bash
# Seeds a small, deliberately memory-hungry "test suite" the agent can point
# the skill at -- something concrete to wrap, without needing a real project.
set -euo pipefail

mkdir -p heavytool
cat > heavytool/run_suite.sh <<'SH'
#!/usr/bin/env bash
# A stand-in for a real test/analysis suite: allocates memory in pure bash
# (no interpreter dependency) and reports how much it held before exiting
# cleanly. Bounded to a size that will not actually threaten the host, but
# shaped like the real workloads the skill exists for (a suite that COULD
# balloon if left unwrapped).
set -euo pipefail
chunk="$(head -c 1048576 /dev/zero | tr '\0' 'x')"
blocks=()
for _ in $(seq 1 80); do
    blocks+=("$chunk")
done
printf 'held %d MiB\n' "${#blocks[@]}"
SH
chmod +x heavytool/run_suite.sh

echo "fixture: seeded heavytool/run_suite.sh (an ~80MiB synthetic workload) into /workspace" >&2
