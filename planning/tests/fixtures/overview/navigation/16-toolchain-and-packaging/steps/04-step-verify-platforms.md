# Step: 04-step-verify-platforms

## Ownership

- Goal: `16-toolchain-and-packaging`
- Work unit: `W106`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `install on each declared platform`
- Subscope: `N/A`

## Objective

§ 4.1
Install from the repository and from the packed npm tarball on each declared platform and on one unsupported pair, execute the selected artifact, and run the installed artifact in render and serve modes with the prohibited interpreter stack unavailable or trapped.

## Instructions

§ 5.1
For each declared platform, install from both the checkout and the packed npm tarball by invoking the generated install.sh from the package root. Record the normalized target key, full placed file list, selected artifact, and one successful render and serve invocation. On Windows use the supported Bash environment for installation and PowerShell for the .exe execution. Repeat on an unsupported key and record no artifact plus W18's notice. Run the installed artifact with the prohibited interpreter tools unavailable or trapped, recording the platform-native process-isolation evidence.

## Acceptance criteria

§ 6.1
The supported-platform records show the same installer behavior from checkout and tarball, exactly one runnable artifact per host, and successful render and serve runs without prohibited child-process invocations. The unsupported record shows no artifact and the named unavailability notice. The tarball list and size match W105. Any trapped or observed Bash, jq, Python, Node, Perl, socat, or overview-script invocation fails this verification.

## Handoff

§ 7.1
Goal 16 is demonstrated on real hosts rather than on the intent of the selection code, and goal 09 can cite these six records instead of repeating them.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
