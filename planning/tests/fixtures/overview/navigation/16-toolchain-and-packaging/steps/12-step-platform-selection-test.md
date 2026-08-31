# Step: 12-step-platform-selection-test

## Ownership

- Goal: `16-toolchain-and-packaging`
- Work unit: `W119`
- Type: `test`

## Change target

- File: planning/tests/test-platform-selection.sh
- Primary symbol or file scope: platform_selection_contract
- Subscope: `N/A`

## Objective

§ 4.1
Test W104 normalize_platform through the installer path with PLAN_OVERVIEW_TEST_MODE=1 and explicit supported OS and architecture values including Windows_NT AMD64 and one unsupported Windows architecture. Assert normalized keys selected paths and no artifact plus the unavailable notice.

## Instructions

§ 5.1
Create planning/tests/test-platform-selection.sh and invoke installer/src/20-runtime-tools.sh through the same installer entry path W104 owns. For each supported target set PLAN_OVERVIEW_TEST_MODE=1 with the exact OS and architecture inputs and assert the normalized x86_64-unknown-linux-musl aarch64-unknown-linux-musl x86_64-apple-darwin aarch64-apple-darwin and x86_64-pc-windows-msvc keys. Set PLAN_OVERVIEW_TEST_OS=Windows_NT and PLAN_OVERVIEW_TEST_ARCH=AMD64 for the supported Windows Bash case and assert plan-overview-x86_64-pc-windows-msvc.exe. Then set an unsupported Windows architecture and assert no artifact is placed and the unavailable notice is emitted. Do not duplicate the mapping in the test.

## Acceptance criteria

§ 6.1
The repository shell-test harness invokes this test with PLAN_OVERVIEW_TEST_MODE=1 and explicit OS and architecture inputs for every supported target plus Windows_NT with AMD64 and one unsupported Windows architecture. It passes only when the W104 installer path selects the expected registry entry and Windows .exe and rejects the unsupported case with no artifact and the unavailable notice.

## Handoff

§ 7.1
W106 consumes the normalized-key records for real-host installation verification, and W48 consumes this shell test through the repository test harness.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
