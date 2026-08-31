# Verification: 08-step-runtime-isolation

## Automated tests

§ 2.1
In a clean temporary install root run the installed artifact with bounded render and serve lifecycles. Require render completion within 10 seconds. For serve require the port line and successful / and /state.json responses within 10 seconds then terminate within 5 seconds and assert the port is closed and no child remains. Use strace execve filtering on Linux sandbox-exec process-exec denial on macOS and a Windows Job Object with active-process limit one. Fail when a control is unavailable or when any prohibited interpreter or overview script is executed.