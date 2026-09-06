# Verification: 02-step-manifest-binary-presence

## Automated tests

§ 2.1
Run tests/test-skill-files-manifest.sh three ways: bundled binary present (pass), absent (pass via skip rule), present-but-invalid via chmod -x (must fail naming the row).