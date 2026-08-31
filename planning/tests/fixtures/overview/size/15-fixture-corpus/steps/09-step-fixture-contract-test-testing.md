# Verification: 09-step-fixture-contract-test

## Automated tests

§ 2.1
Run the fixture contract test on the corpus and confirm it passes. Then, one fixture at a time, remove the edge case that fixture exists for and confirm the test fails naming that fixture: delete the orphan's isolation by adding a dependency, remove the size exception, flip the testing requirement to yes, fill each of the four evidence gaps, complete a step in the fresh fixture, repair the truncated state, and restore the missing transition time. A removal that leaves the test green means that fixture is not actually covering its story.