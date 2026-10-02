#!/usr/bin/env bash
# Seeds a tiny project whose test suite has one genuinely surprising,
# undocumented requirement -- discoverable only by actually running it and
# reading the failure, not from any README. This is the "confirmed deviation"
# the story expects the agent to record via project-specifics' own convention.
set -euo pipefail

mkdir -p tinytool
cat > tinytool/README.md <<'MD'
# tinytool

A tiny CLI that formats a greeting. Run its test suite with:

    ./run_tests.sh
MD

cat > tinytool/greet.sh <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf 'Hello, %s!\n' "${1:-world}"
SH
chmod +x tinytool/greet.sh

# The surprising bit: without TINYTOOL_NONINTERACTIVE=1 the suite fails with
# a message that reads like a real assertion failure ("greet.sh produced:
# Ada is not greeted") even though greet.sh itself is correct -- the actual
# cause (a missing env var the README never mentions) is nowhere in that
# message. Nothing in the README says this exists; it's discoverable only by
# reading run_tests.sh itself.
cat > tinytool/run_tests.sh <<'SH'
#!/usr/bin/env bash
set -euo pipefail
if [ "${TINYTOOL_NONINTERACTIVE:-0}" != "1" ]; then
    # Deliberately misleading: looks like greet.sh is broken. It isn't --
    # this whole branch only exists because TINYTOOL_NONINTERACTIVE=1 wasn't set.
    echo "FAIL: greet.sh produced: Ada is not greeted"
    exit 1
fi
actual="$(./greet.sh Ada)"
expected="Hello, Ada!"
if [ "$actual" = "$expected" ]; then
    echo "PASS: greet.sh"
    exit 0
fi
echo "FAIL: greet.sh produced: $actual"
exit 1
SH
chmod +x tinytool/run_tests.sh

echo "fixture: seeded tinytool/ (tests fail with a misleading message unless TINYTOOL_NONINTERACTIVE=1 is set) into /workspace" >&2
