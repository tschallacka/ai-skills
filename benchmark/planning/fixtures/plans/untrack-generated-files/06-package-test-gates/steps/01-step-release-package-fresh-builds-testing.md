# Verification: 01-step-release-package-fresh-builds

## Automated tests

§ 2.1
Run tests/test-release-package.sh (pass on untracked tree); inject a stale lib into the tarball input and expect the byte-identity assertion to fail; confirm exactly-once, zero-sources and installability assertions unchanged.