# Verification: 03-step-bundled-rjq-notice

## Automated tests

§ 2.1
Source installer/src/20-runtime-tools.sh: without the binary assert exit 0, notice on stderr, PATH unchanged; with it, PATH prepended as before. Run installer/build.sh --check after regeneration.