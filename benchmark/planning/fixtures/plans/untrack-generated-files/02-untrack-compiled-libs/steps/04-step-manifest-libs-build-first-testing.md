# Verification: 04-step-manifest-libs-build-first

## Automated tests

§ 2.1
Run tests/test-skill-files-manifest.sh with the libs removed: they are built first and the test passes; with libs present, build-plan-libs.sh is not invoked (mtime unchanged).