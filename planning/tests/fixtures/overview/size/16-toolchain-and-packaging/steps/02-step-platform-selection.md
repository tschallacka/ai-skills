# Step: 02-step-platform-selection

## Ownership

- Goal: `16-toolchain-and-packaging`
- Work unit: `W104`
- Type: `source`

## Change target

- File: `installer/src/20-runtime-tools.sh`
- Primary symbol or file scope: artifact platform selection and placement branch
- Subscope: `N/A`

## Objective

§ 4.1
Implement one normalize_platform function for production uname and PROCESSOR_ARCHITECTURE inputs plus test-only PLAN_OVERVIEW_TEST_MODE inputs PLAN_OVERVIEW_TEST_OS and PLAN_OVERVIEW_TEST_ARCH. Use it for artifact selection and placement and route unmatched keys to W18.

## Instructions

§ 5.1
Own host detection artifact selection and placement in installer/src/20-runtime-tools.sh. The single normalize_platform function accepts production uname and PROCESSOR_ARCHITECTURE inputs and when PLAN_OVERVIEW_TEST_MODE=1 accepts PLAN_OVERVIEW_TEST_OS and PLAN_OVERVIEW_TEST_ARCH through that same function. Normalize POSIX values to exactly x86_64-unknown-linux-musl aarch64-unknown-linux-musl x86_64-apple-darwin or aarch64-apple-darwin. Map Windows_NT with AMD64 to x86_64-pc-windows-msvc and reject other Windows architecture values. Select the registry path for the exact key, place plan-overview-x86_64-pc-windows-msvc.exe on Windows and the exact non-extension artifact name elsewhere. Any unmatched key calls W18's notice branch. Do not implement that notice here.

## Acceptance criteria

§ 6.1
On each declared platform exactly one matching artifact is placed and executes once with the Windows .exe name defined above. In PLAN_OVERVIEW_TEST_MODE=1 the same normalize_platform function receives explicit OS and architecture values and produces the same keys as production detection. On an unsupported pair no artifact is placed and control reaches W18's unavailable-platform notice. The selection test records the detection inputs and selected path rather than presence alone.

## Handoff

§ 7.1
W20 verifies the no-artifact path against a selection that genuinely declines to place one, and goal 09 can list an installed file set that matches the host it installed on. This unit depends on W18 because its own acceptance criterion requires the unavailability notice to be what the installer prints on a host matching no artifact, and W18 owns that notice; adversarial finding AR-55 recorded that the criterion named W18 while no edge existed in either direction, so an executor running this unit first would have had no notice to print.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
