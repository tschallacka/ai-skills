# Testing companion: 01-step-static-inspection

## Command verification

Future executor should run bounded file-list and source-inspection commands from the benchmark workspace after implementation, without serving or opening HTML during this static step.

Required checks:

- Confirm `button-chain.html` exists.
- Confirm no extra `.html` or `.htm` task artifacts were created.
- Confirm the source contract includes one initial button, generated-button click handling, exact text `finished`, and a visible white border style.

This planning-only proof did not run those checks because the HTML artifact must not be created yet.

