# Verification: 13-step-npm-package-test

## Automated tests

§ 2.1
Run npm pack into a captured tarball and extract it into a clean temporary directory. Read planning/tests/fixtures/overview/npm-package-baseline.tsv as the only W105-owned expected package record. Validate its fixed header, lexical package_path ordering, exact sorted tarball file list and final tarball_bytes value against the freshly packed tarball. Invoke the extracted installer through W104 test inputs for supported Linux macOS and Windows AMD64 and one unsupported Windows architecture. Assert one executable plus render and serve output for supported cases and no artifact plus the unavailable notice for unsupported input. Require cleanup success and fail if the baseline is missing or stale rather than regenerating it in W120.
