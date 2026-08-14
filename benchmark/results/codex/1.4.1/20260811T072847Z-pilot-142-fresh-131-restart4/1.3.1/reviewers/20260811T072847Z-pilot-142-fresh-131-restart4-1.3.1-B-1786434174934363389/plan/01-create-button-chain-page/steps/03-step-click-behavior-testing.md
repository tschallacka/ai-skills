# Testing companion: 03-step-click-behavior

## Behavior verification

- Target work unit: `W03`
- Future check: covered by W04 automated behavior simulation and W05 browser story validation.
- Required assertions:
  - Only the current last button appends a new button.
  - Each valid append click creates exactly one button below the previous last button.
  - The fourth generated button is the fourth appended button after the initial button.
  - Clicking the fourth generated button clears prior document content.
  - The final rendered message text is exactly `finished`.

## Result for this planning proof

- Status: not run.
- Evidence: this benchmark forbids creating, opening, inspecting, or testing HTML during the planning-only proof.
- Completion requirement for future executor: do not mark W03 complete until W04 has passing automated evidence and W05 is ready to run against the implemented behavior.
