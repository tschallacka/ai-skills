# Verification: 03-step-plan-dir-synonym

## Automated tests

§ 2.1
Run the synonym test and confirm it now exercises the plan-dir flag directly rather than deferring to a retired test. Fault-inject by removing the flag handling from one covered entry point and confirm this test, and no other, reports the loss.