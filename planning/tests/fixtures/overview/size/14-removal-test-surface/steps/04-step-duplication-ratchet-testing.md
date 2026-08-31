# Verification: 04-step-duplication-ratchet

## Automated tests

§ 2.1
Run the duplication ratchet and confirm it passes with the corrected count. Prove the count is the assertion: restore the old number and confirm the ratchet fails, then re-add a canonicalisation site elsewhere and confirm it fails in the other direction. A ratchet that passes under both numbers is not measuring anything.