# Testing companion: implement and verify button chain

## Browser verification

Open the future local `button-chain.html` directly in a browser. Record the initial button count. Activate the current last button once at a time and after each activation record the total count and vertical order. The first three activations must produce counts 2, 3, and 4, with exactly one new button each time. Activate the fourth generated button and verify the document is cleared, no button remains, and the only visible completion text is exactly `finished` in lowercase with a nonzero visible white border. Pass only if all assertions hold; otherwise fail with the first divergent count, order, or completion property.

## Static/automated verification

Inspect `button-chain.html` after implementation to confirm the initial button, one-append path, generated-count threshold of four, terminal clear path, exact string, and white border declaration. If a project-neutral HTML checker is available, run it against that file and record its exit status; do not treat syntax validity as a substitute for the browser sequence.

## Current proof result

Not run. The benchmark safety boundary forbids creating, opening, inspecting, serving, or testing HTML.
