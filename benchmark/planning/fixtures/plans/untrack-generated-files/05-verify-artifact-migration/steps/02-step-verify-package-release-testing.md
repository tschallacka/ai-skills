# Verification: 02-step-verify-package-release

## Automated tests

§ 2.1
npm pack --dry-run and installer/build-release.sh: assert five libs plus REVIEWER.md present, zero lib sources, zero build-plan-libs.sh, no planning/bin path; run a scratch install from the tarball expecting completion.