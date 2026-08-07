# Testing: 02-step-behavior

## Automated tests

- Future check only: exercise the transition sequence `initial -> generated 1 -> generated 2 -> generated 3 -> generated 4 -> finished` and assert one append on each of the first four activations and none on the fifth.
- Assert retired buttons cannot append, repeated activation cannot append twice, completion removes all prior document nodes, exact lowercase `finished` remains, and its rendered border color is white.
- Do not run this check during the planning-only proof.

## Browser verification

- Covered downstream by W03 / US-01 with five direct presses in a fresh context.
- Do not open or render HTML during the planning-only proof.
