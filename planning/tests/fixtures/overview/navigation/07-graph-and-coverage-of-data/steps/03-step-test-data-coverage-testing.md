# Verification: 03-step-test-data-coverage

## Automated tests

§ 2.1
Run the test, then remove one stored field from the state fixture and confirm the test fails naming that field. This test is the guarantee that no stored data is unpresented, so it must enumerate fields rather than sample them.