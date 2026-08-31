# Step: 08-step-runtime-isolation

## Ownership

- Goal: `09-verification`
- Work unit: `W118`
- Type: `test`

## Change target

- File: `src/plan-overview/tests/runtime_isolation.rs`
- Primary symbol or file scope: `installed_binary_has_no_prohibited_children()`
- Subscope: `N/A`

## Objective

§ 4.1
Register runtime_isolation.rs to install the artifact in a temporary root and run bounded render and serve lifecycles under strace on Linux sandbox-exec on macOS and a Windows Job Object. Require readiness termination and zero prohibited child or overview-script executions.

## Instructions

§ 5.1
Create src/plan-overview/tests/runtime_isolation.rs. In a clean temporary directory run bash install.sh --install-root <temp> from the package root and set PLAN_OVERVIEW_BIN to the installed artifact. For render run PLAN_OVERVIEW_BIN --plan-dir <fixture> --out <temp>/rendered.html and require exit zero within 10 seconds. For serve run PLAN_OVERVIEW_BIN --plan-dir <fixture> --serve --port 0, require a bound-port line and a successful request to / and /state.json within 10 seconds, then send TERM, wait up to 5 seconds and assert the port is closed and no child remains. On Linux wrap each run with strace -f -e trace=execve -o trace.log; fail if strace cannot start and parse the complete trace for Bash jq Python Node Perl socat or overview-script executions. On macOS wrap each run with sandbox-exec -p '(version 1) (allow process*) (deny process-exec)'; fail if sandbox-exec is unavailable and treat any denied child or shell-script access as non-zero. On Windows create a Job Object with JOB_OBJECT_LIMIT_ACTIVE_PROCESS=1, launch the installed .exe inside it from the standard-user test process, enforce the same 10 second render and serve timeouts and 5 second termination wait, and fail if the control cannot be created. Any timeout setup error traced prohibited executable denied child or surviving process is non-zero.

## Acceptance criteria

§ 6.1
The test is discovered by the repository cargo leg and therefore by W48. Both bounded render and serve runs pass with zero prohibited child-process or overview-script invocations. Readiness is proven by the port line and requests to / and /state.json. Termination is proven by a closed port and no surviving child. Any timeout control failure trap hit traced prohibited executable denied child or shell-script access makes the test non-zero.

## Handoff

§ 7.1
W106 uses this automated isolation check alongside the platform install records; W48 consumes its crate-test result through the repository gate.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
