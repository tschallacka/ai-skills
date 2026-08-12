# Verification: 01-step-seed-encrypted-defects

## Automated tests

- Verify encrypted-manifest creation, defect application, hashes, permissions,
  absence of plaintext mapping, and absence of the key from target inputs.
- Fail if the seed mapping or key appears in target-visible files, arguments,
  environment snapshots, or logs.
