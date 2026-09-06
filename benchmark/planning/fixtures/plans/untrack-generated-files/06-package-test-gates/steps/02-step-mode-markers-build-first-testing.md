# Verification: 02-step-mode-markers-build-first

## Automated tests

§ 2.1
Run tests/test-mode-markers.sh on a clean tree (build-if-missing fires, pass); plant a wrong MODE marker in a compiled lib and expect the scan to fail; confirm the build/generate steps run once per invocation.