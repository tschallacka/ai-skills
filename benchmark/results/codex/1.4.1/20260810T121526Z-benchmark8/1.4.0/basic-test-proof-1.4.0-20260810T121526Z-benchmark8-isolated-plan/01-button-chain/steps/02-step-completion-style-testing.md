# Testing companion: 02-step-completion-style

## Browser verification

US-01 must reach its terminal state by pressing generated button 4 on the fifth direct click overall. Inspect the rendered completion message visually and confirm that the exact lowercase text `finished` is visible on a contrasting background with the explicit 1px solid white border. Pass only when both the text and distinguishable border are observable; fail if the casing, text, visibility, contrast, thickness, style, or border color is wrong.

## Automated tests

No separate automated test is planned for this inline style target. The required proof is the direct browser assertion in US-01, owned by W04.
