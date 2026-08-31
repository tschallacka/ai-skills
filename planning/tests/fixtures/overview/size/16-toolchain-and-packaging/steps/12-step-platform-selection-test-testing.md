# Verification: 12-step-platform-selection-test

## Automated tests

§ 2.1
Invoke the installer through W104 with PLAN_OVERVIEW_TEST_MODE=1 and explicit platform inputs. Assert all five normalized keys the Windows .exe path and rejection of unsupported Windows architecture without an artifact.